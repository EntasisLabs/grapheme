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
    assert_eq!(ir["$ir"], "grapheme-ui/1");
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
        ("on click: Read", "'Read' is not a mutation"),
        (
            "on click: Bump",
            "'Bump' is not a mutation on state 'Counter'",
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
    assert!(
        artifact.lint_warnings.is_empty(),
        "{:?}",
        artifact.lint_warnings
    );
}
