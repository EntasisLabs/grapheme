//! `GraphemeEngine::builder().element_catalog` loads a data-only catalog.

use grapheme_sdk::GraphemeEngine;

const CATALOG: &str = r#"{
  "$catalog": "test/divider",
  "elements": [
    {
      "name": "Divider",
      "role": "separator",
      "children": false,
      "props": [],
      "events": []
    }
  ]
}"#;

#[test]
fn element_catalog_compiles_a_host_element_and_names_the_catalog() {
    let source =
        "state S { n: Int = 0 }\nmutation Nop on S { apply { n: $n } }\nview V on S { Divider }\n";
    let engine = GraphemeEngine::builder()
        .element_catalog(CATALOG)
        .expect("catalog")
        .build();
    engine
        .store_session(source)
        .expect("Divider is in the catalog");

    let Err(err) = engine.store_session(
        "state S { n: Int = 0 }\nmutation Nop on S { apply { n: $n } }\nview V on S { Grid }\n",
    ) else {
        panic!("Grid was accepted");
    };
    let err = err.to_string();
    assert!(
        err.contains("unknown element 'Grid'")
            && err.contains("catalog 'test/divider'")
            && err.contains("--> 3:"),
        "{err}"
    );

    let Err(rejected) = GraphemeEngine::builder().element_catalog(
        r#"{"$catalog":"bad","elements":[{"name":"Box","role":"group","props":[{"name":"tint","type":"Colour"}]}]}"#,
    ) else {
        panic!("invalid catalog was accepted");
    };
    let rejected = rejected.to_string();
    assert!(
        rejected.contains("catalog 'bad'") && rejected.contains("unknown type 'Colour'"),
        "{rejected}"
    );
}
