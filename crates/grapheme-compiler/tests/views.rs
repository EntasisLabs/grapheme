//! RFC-0007 step 2: view grammar, View IR lowering and binding checks.

const PRELUDE: &str = r#"
state Counter {
  count: Int = 0
  derive double: Int = $count * 2
}
state Other { n: Int = 0 }
mutation Increment on Counter {
  apply { count: $count + 1 }
}
mutation Add($by: Int) on Counter {
  apply { count: $count + $by }
}
mutation Bump on Other {
  apply { n: $n + 1 }
}
query Read on Counter -> Int { $count }
"#;

fn compile(views: &str) -> Result<grapheme_compiler::pipeline::CompilationArtifact, String> {
    grapheme_compiler::compile(&format!("{PRELUDE}{views}")).map_err(|e| e.to_string())
}

fn error(views: &str) -> String {
    match compile(views) {
        Ok(_) => panic!("expected a compile error for: {views}"),
        Err(e) => e,
    }
}

#[test]
fn lowers_elements_props_keys_and_bindings() {
    let artifact = compile(
        r#"
view CounterView on Counter {
  Column(gap: 4) {
    value: Text("{$count} / {$double}")
    Text($count)
    increment: Button(label: "Increment") { on click: Increment }
    Button { on click: Add(by: $count + 1), Text("Add") }
  }
}
"#,
    )
    .expect("compiles");
    let ir = grapheme_artifact::view_ir_document(&artifact.mir.views);
    assert_eq!(ir["$ir"], "grapheme-ui/2");
    let root = &ir["views"][0]["root"];
    assert_eq!(ir["views"][0]["store"], "Counter");
    assert_eq!(root["$prim"], "Column");
    assert_eq!(root["key"], "CounterView");
    assert_eq!(root["props"]["gap"], 4);
    let keys: Vec<_> = root["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["key"].as_str().unwrap())
        .collect();
    assert_eq!(
        keys,
        [
            "CounterView/value",
            "CounterView/0",
            "CounterView/increment",
            "CounterView/1"
        ]
    );
    assert_eq!(
        root["children"][0]["props"]["content"],
        "{$state.count} / {$state.double}"
    );
    assert_eq!(
        root["children"][1]["props"]["content"],
        serde_json::json!({"$var": "state.count"})
    );
    assert_eq!(
        root["children"][2]["events"]["click"],
        serde_json::json!({"$action": "Increment"})
    );
    assert_eq!(root["children"][3]["events"]["click"]["$action"], "Add");
    assert_eq!(root["children"][3]["children"][0]["key"], "CounterView/1/0");
    assert!(
        artifact.lint_warnings.is_empty(),
        "{:?}",
        artifact.lint_warnings
    );
}

#[test]
fn programs_without_views_emit_no_views_field() {
    let artifact = compile("").expect("compiles");
    let mir = serde_json::to_value(&artifact.mir).unwrap();
    assert!(mir.get("views").is_none());
}

#[test]
fn view_references_lower_to_view_nodes() {
    let artifact = compile(
        r#"
view Footer { Row { Text("footer") } }
view Main on Counter { Column { footer: Footer } }
"#,
    )
    .expect("compiles");
    let main = artifact
        .mir
        .views
        .iter()
        .find(|v| v.name == "Main")
        .unwrap();
    let footer = &main.root.children[0];
    assert_eq!(footer.view.as_deref(), Some("Footer"));
    assert_eq!(footer.key, "Main/footer");
    assert!(footer.prim.is_none());
}

#[test]
fn rejects_steps_and_module_calls() {
    let err = error(r#"view V on Counter { Column { Text("a") |> core.echo() } }"#);
    assert!(err.contains("a view contains elements only"), "{err}");
    let err = error(r#"view V on Counter { Column { core.echo(x: 1) } }"#);
    assert!(err.contains("a view contains elements only"), "{err}");
}

#[test]
fn rejects_unknown_elements_props_events_and_children() {
    for (source, expected) in [
        (
            r#"view V on Counter { Grid { Text("a") } }"#,
            "unknown element 'Grid'",
        ),
        (
            r#"view V on Counter { Button(color: "red") { on click: Increment } }"#,
            "'Button' has no prop 'color'",
        ),
        (
            r#"view V on Counter { Column(gap: 1, gap: 2) { Text("a") } }"#,
            "prop 'gap' is given more than once",
        ),
        (
            r#"view V on Counter { Column(gap: "wide") { Text("a") } }"#,
            "prop 'gap' expects Int, found String",
        ),
        (
            r#"view V on Counter { Column { on click: Increment } }"#,
            "'Column' has no event 'click'",
        ),
        (
            r#"view V on Counter { Text("a") { Text("b") } }"#,
            "'Text' cannot have children",
        ),
        (
            r#"view V on Counter { Column("a") { Text("b") } }"#,
            "'Column' takes no positional content",
        ),
        (
            r#"view V on Counter { Column { a: Text("x"), a: Text("y") } }"#,
            "two children are named 'a'",
        ),
        (
            r#"view V on Counter { Text("a") Text("b") }"#,
            "exactly one root element",
        ),
        (r#"view V on Missing { Text("a") }"#, "not a declared state"),
        (
            r#"view V on Counter { Text("{$missing}") }"#,
            "'$missing' is not a field",
        ),
        (r#"view V { Text($count) }"#, "has no `on` state"),
        (r#"view Text { Text("a") }"#, "name of a built-in primitive"),
        (
            r#"view A on Other { Text("a") } view B on Counter { Column { A } }"#,
            "view 'A' is on state 'Other'",
        ),
    ] {
        let err = error(source);
        assert!(
            err.contains(expected),
            "{source}: expected '{expected}', got '{err}'"
        );
    }
}

#[test]
fn checks_binding_targets_and_arguments() {
    for (binding, expected) in [
        (
            "on click: Missing",
            "binding target 'Missing' is not declared",
        ),
        ("on click: Read", "'Read' is not a mutation or task"),
        (
            "on click: Bump",
            "'Bump' is not a mutation or task on state 'Counter'",
        ),
        ("on click: Add", "'Add' needs argument 'by'"),
        (
            "on click: Add(by: 1, by: 2)",
            "argument 'by' is given more than once",
        ),
        ("on click: Add(step: 1)", "'Add' has no parameter 'step'"),
        (
            "on click: Add(by: \"one\")",
            "argument 'by' of 'Add' expects Int, found String",
        ),
        (
            "on click: Increment\n on click: Increment",
            "event 'click' is bound more than once",
        ),
    ] {
        let err = error(&format!(
            "view V on Counter {{ Button(label: \"x\") {{ {binding} }} }}"
        ));
        assert!(
            err.contains(expected),
            "{binding}: expected '{expected}', got '{err}'"
        );
    }
    compile(r#"view V on Counter { Button(label: "x") { on click: Add(by: $count) } }"#)
        .expect("a field of the parameter's type is accepted");
    let err = error(r#"view V { Button(label: "x") { on click: Increment } }"#);
    assert!(err.contains("has no `on` state"), "{err}");
}

#[test]
fn textfield_change_supplies_value_as_payload_param() {
    let source = |body: &str| {
        format!(
            r#"
state Form {{ query: String = "" }}
mutation SetQuery($value: String) on Form {{ apply {{ query: $value }} }}
mutation NeedsTwo($value: String, $extra: String) on Form {{
  apply {{ query: $value }}
}}
view V on Form {{ {body} }}
"#
        )
    };
    let artifact = grapheme_compiler::compile(&source(
        r#"TextField(value: $query, label: "Q") { on change: SetQuery }"#,
    ))
    .unwrap();
    let view = artifact.mir.views.iter().find(|v| v.name == "V").unwrap();
    let binding = view.root.events.get("change").expect("change binding");
    assert_eq!(binding.target, "SetQuery");
    assert!(binding.args.is_empty());
    assert_eq!(binding.payload_params, vec!["value".to_string()]);

    let err = grapheme_compiler::compile(&source(
        r#"TextField(label: "Q") { on change: NeedsTwo }"#,
    ))
    .unwrap_err()
    .to_string();
    assert!(
        err.contains("'NeedsTwo' needs argument 'extra'"),
        "{err}"
    );

    let err = grapheme_compiler::compile(&source(
        r#"TextField(label: "Q") { on click: SetQuery }"#,
    ))
    .unwrap_err()
    .to_string();
    assert!(err.contains("'TextField' has no event 'click'"), "{err}");
}

#[test]
fn warns_when_an_interactive_element_has_no_accessible_name() {
    let artifact =
        compile(r#"view V on Counter { Column { Button { on click: Increment } } }"#).unwrap();
    let lint = artifact
        .lint_warnings
        .iter()
        .find(|w| w.code == "view-accessibility")
        .expect("accessibility lint");
    assert!(lint.message.contains("element 'V/0'"), "{}", lint.message);

    for named in [
        r#"view V on Counter { Button(label: "Go") { on click: Increment } }"#,
        r#"view V on Counter { Button { on click: Increment, Text("Go") } }"#,
    ] {
        let artifact = compile(named).unwrap();
        assert!(
            !artifact
                .lint_warnings
                .iter()
                .any(|w| w.code == "view-accessibility"),
            "{named}"
        );
    }
}

#[test]
fn counter_example_compiles_under_strict_policy() {
    let source = include_str!("../../../examples/view-counter.gr");
    let ast = grapheme_compiler::parse(source).expect("parses");
    let artifact = grapheme_compiler::compile_program(
        ast,
        grapheme_compiler::CompileOptions {
            capability_policy: grapheme_artifact::CapabilityPolicy::default(),
            executable_kind_policy_mode:
                grapheme_compiler::verifier::ExecutableKindPolicyMode::StrictMutationOnly,
        },
    )
    .expect("strict compile");
    assert_eq!(artifact.mir.views.len(), 2);
    assert_eq!(artifact.mir.ui_glyphs.len(), 1);
    assert_eq!(artifact.mir.ui_glyphs[0].name, "CounterApp");
    assert_eq!(artifact.mir.ui_glyphs[0].root, "CounterView");
    assert_eq!(artifact.mir.ui_glyphs[0].expose, vec!["Increment"]);
    assert!(
        artifact.lint_warnings.is_empty(),
        "{:?}",
        artifact.lint_warnings
    );
}

#[test]
fn ui_glyph_rejects_unknown_store_expose_and_cross_store_root() {
    assert!(
        error("glyph App on Missing { Text(\"x\") }").contains("not a declared state"),
        "{}",
        error("glyph App on Missing { Text(\"x\") }")
    );
    assert!(
        error(
            r#"
view CounterView on Counter { Text("x") }
glyph App on Counter {
  expose Missing
  CounterView
}
"#
        )
        .contains("exposes 'Missing'"),
        "{}",
        error(
            r#"
view CounterView on Counter { Text("x") }
glyph App on Counter {
  expose Missing
  CounterView
}
"#
        )
    );
    assert!(
        error(
            r#"
view OtherView on Other { Text("x") }
glyph App on Counter { OtherView }
"#
        )
        .contains("which is on Other"),
        "{}",
        error(
            r#"
view OtherView on Other { Text("x") }
glyph App on Counter { OtherView }
"#
        )
    );
}

#[test]
fn ui_glyph_synthesizes_an_inline_root_view() {
    let artifact = compile(
        r#"
glyph CounterApp on Counter {
  Column {
    value: Text("{$count}")
    go: Button(label: "Go") { on click: Increment }
  }
}
"#,
    )
    .expect("compiles");
    assert_eq!(artifact.mir.ui_glyphs.len(), 1);
    assert_eq!(artifact.mir.ui_glyphs[0].root, "CounterApp");
    let view = artifact
        .mir
        .views
        .iter()
        .find(|v| v.name == "CounterApp")
        .expect("synthetic view");
    assert_eq!(view.store.as_deref(), Some("Counter"));
    assert_eq!(view.root.prim.as_deref(), Some("Column"));
    assert_eq!(view.root.key, "CounterApp");
}

const LIST_STATE: &str = r#"
struct Item { id: String label: String }
state Items {
  active: Bool = true
  items: [Item] = []
}
"#;

#[test]
fn lowers_view_if_and_for_with_stable_keys() {
    let artifact = compile(&format!(
        r#"{LIST_STATE}
view ItemsView on Items {{
  Column {{
  if $active {{
    Text("on")
  }}
  for $item in $items key $item.id {{
    Text($item.label)
  }}
  }}
}}
"#
    ))
    .expect("compiles");
    let ir = grapheme_artifact::view_ir_document(&artifact.mir.views);
    let root = &ir["views"][0]["root"];
    assert_eq!(root["$prim"], "Column");
    let keys: Vec<_> = root["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["key"].as_str().unwrap())
        .collect();
    assert_eq!(keys, ["ItemsView/$if", "ItemsView/$for"]);
    let if_node = &root["children"][0];
    assert!(if_node.get("$prim").is_none());
    assert!(if_node.get("$if").is_some());
    assert_eq!(if_node["children"][0]["key"], "ItemsView/$if/0");
    let for_node = &root["children"][1];
    assert!(for_node.get("$for").is_some());
    assert_eq!(for_node["$for"]["item"], "item");
    assert_eq!(
        for_node["$for"]["source"],
        serde_json::json!({"$var": "state.items"})
    );
    assert_eq!(
        for_node["$for"]["key"],
        serde_json::json!({"$var": "item.id"})
    );
    assert_eq!(for_node["children"][0]["key"], "ItemsView/$for/0");
}

#[test]
fn rejects_for_item_shadowing_store_field() {
    let err = error(&format!(
        r#"{LIST_STATE}
view V on Items {{
  Column {{
    for $active in $items key $active.id {{ Text("x") }}
  }}
}}
"#
    ));
    assert!(
        err.contains("for item '$active' has the same name as a field of the store"),
        "{err}"
    );
}

#[test]
fn rejects_unknown_item_field_outside_for_allows_inside_for() {
    let err = error(&format!(
        r#"{LIST_STATE}
view V on Items {{ Text($item.label) }}
"#
    ));
    assert!(
        err.contains("'$item.label' is not a field of state 'Items'"),
        "{err}"
    );
    compile(&format!(
        r#"{LIST_STATE}
view V on Items {{
  Column {{
    for $item in $items key $item.id {{
      Text($item.label)
    }}
  }}
}}
"#
    ))
    .expect("item fields resolve inside for");
}

#[test]
fn nested_if_inside_for_compiles() {
    let artifact = compile(&format!(
        r#"{LIST_STATE}
view V on Items {{
  Column {{
    for $item in $items key $item.id {{
      if $active {{
        Text($item.label)
      }}
    }}
  }}
}}
"#
    ))
    .expect("compiles");
    let view = artifact.mir.views.iter().find(|v| v.name == "V").unwrap();
    let for_node = view.root.children.iter().find(|c| c.repeat.is_some()).unwrap();
    assert_eq!(for_node.key, "V/$for");
    let if_child = for_node
        .children
        .iter()
        .find(|c| c.when.is_some())
        .unwrap();
    assert_eq!(if_child.key, "V/$for/$if");
    assert_eq!(
        if_child.children[0].props.get("content"),
        Some(&serde_json::json!({"$var": "item.label"}))
    );
}

#[test]
fn view_if_requires_bool() {
    let err = error(
        r#"
state S { show: Int = 0 }
view V on S {
  Column { if $show { Text("x") } }
}
"#,
    );
    assert!(err.contains("if condition must be Bool, found Int"), "{err}");
}

#[test]
fn nested_for_sees_the_outer_item_and_rejects_shadowing() {
    compile(
        r#"
struct Post { id: String title: String }
struct Group { name: String posts: [Post] }
state Feed { groups: [Group] = [] }
view V on Feed {
  Column {
    for $group in $groups key $group.name {
      for $post in $group.posts key $post.id {
        Text($group.name)
      }
    }
  }
}
"#,
    )
    .expect("outer item stays in scope");
    let err = error(
        r#"
struct Post { id: String }
struct Group { name: String posts: [Post] }
state Feed { groups: [Group] = [] }
view V on Feed {
  Column {
    for $group in $groups key $group.name {
      for $group in $group.posts key $group.id {
        Text("x")
      }
    }
  }
}
"#,
    );
    assert!(err.contains("shadows an outer for binding"), "{err}");
}

#[test]
fn else_and_for_index_lower() {
    let artifact = compile(
        r#"
state S { show: Bool = true, items: [Int] = [] }
view V on S {
  Column {
    if $show { Text("yes") } else { Text("no") }
    for $item, $index in $items key $index {
      Text("{$index}")
    }
  }
}
"#,
    )
    .expect("else and index compile");
    let view = artifact.mir.views.iter().find(|v| v.name == "V").unwrap();
    let if_node = view.root.children.iter().find(|c| c.when.is_some()).unwrap();
    assert_eq!(if_node.else_children.len(), 1);
    let for_node = view.root.children.iter().find(|c| c.repeat.is_some()).unwrap();
    assert_eq!(
        for_node.repeat.as_ref().and_then(|r| r.index.as_deref()),
        Some("index")
    );
}

#[test]
fn unnamed_if_does_not_use_a_child_name() {
    let artifact = compile(
        r#"
state S { show: Bool = true }
view V on S {
  Column {
    if: Text("named")
    if $show { Text("branch") }
  }
}
"#,
    )
    .expect("compiles");
    let view = artifact.mir.views.iter().find(|v| v.name == "V").unwrap();
    let keys: Vec<_> = view.root.children.iter().map(|c| c.key.as_str()).collect();
    assert_eq!(keys, ["V/if", "V/$if"]);
}

#[test]
fn textfield_change_payload_must_match_the_parameter_type() {
    let err = error(
        r#"
state S { n: Int = 0, text: String = "" }
mutation SetInt($value: Int) on S { apply { n: $value } }
view V on S {
  TextField(label: "Q") { on change: SetInt }
}
"#,
    );
    assert!(
        err.contains("has type Int") && err.contains("String"),
        "{err}"
    );
    let err = error(
        r#"
state S { text: String = "" }
mutation Set($value: String) on S { apply { text: $value } }
view V on S {
  TextField(label: "Q") { on change: Set(value: "fixed") }
}
"#,
    );
    assert!(
        err.contains("supplied by the binding and by the event payload"),
        "{err}"
    );
    compile(
        r#"
state S { text: String = "" }
mutation Set($value: String) on S { apply { text: $value } }
view V on S {
  TextField(label: "Q") { on change: Set }
}
"#,
    )
    .expect("String payload matches a String parameter");
}
