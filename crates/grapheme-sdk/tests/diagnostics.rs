//! Structured diagnostics carry source spans for parse and type errors.

use grapheme_sdk::GraphemeEngine;

#[test]
fn parse_error_diagnostic_has_a_span() {
    let engine = GraphemeEngine::builder().build();
    let diagnostics = engine.diagnostics("state S { n: Int = \n");
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.kind, "parse");
    assert_eq!(diagnostic.severity, "error");
    assert!(!diagnostic.message.is_empty());
    let span = diagnostic.span.expect("parse error span");
    assert!(span.line >= 1, "{span:?}");
    assert!(span.column >= 1, "{span:?}");
    assert!(span.end_line >= span.line, "{span:?}");
}

#[test]
fn type_error_diagnostic_has_a_span() {
    let source = r#"
state S { n: Int = 0 }
mutation Set($value: Int) on S { apply { n: $value } }
view V on S {
  TextField(label: "Q") { on change: Set }
}
"#;
    let engine = GraphemeEngine::builder().build();
    let diagnostics = engine.diagnostics(source);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.kind, "type");
    assert!(
        diagnostic.message.contains("parameter")
            && diagnostic.message.contains("String")
            && diagnostic.message.contains("Int"),
        "{}",
        diagnostic.message
    );
    let span = diagnostic.span.expect("type error span");
    assert_eq!(span.line, 5, "on change is line 5: {span:?}");
    assert!(span.column >= 1, "{span:?}");

    let err = match engine.store_session(source) {
        Ok(_) => panic!("type error should not compile"),
        Err(err) => err,
    };
    let from_error = err.diagnostics();
    assert_eq!(from_error.len(), 1);
    assert_eq!(from_error[0].span, Some(span));
    assert!(err.to_string().to_lowercase().contains("type"));
}

#[test]
fn successful_compile_has_no_diagnostics() {
    let engine = GraphemeEngine::builder().build();
    let diagnostics =
        engine.diagnostics("state S { n: Int = 0 }\nmutation Nop on S { apply { n: $n } }\n");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}
