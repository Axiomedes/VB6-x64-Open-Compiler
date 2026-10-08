use vb6_compiler::parse_source;
use vb6_compiler::semantic::{SemanticAnalyzer, SemanticError};
use vb6_compiler::types::VbType;

#[test]
fn test_detect_type_mismatch_long_equals_string() {
    let source = r#"
Sub Main()
    Dim X As Long
    X = "Hola"
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse source");
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze_file(&ast);

    assert!(result.is_err(), "Expected semantic analysis to fail with Type Mismatch");
    let errors = result.unwrap_err();
    assert_eq!(errors.len(), 1);

    match &errors[0] {
        SemanticError::TypeMismatch { expected, found, .. } => {
            assert_eq!(*expected, VbType::Long);
            assert_eq!(*found, VbType::String);
        }
        other => panic!("Expected TypeMismatch, got {:?}", other),
    }
}

#[test]
fn test_valid_numeric_coercions() {
    let source = r#"
Sub Main()
    Dim X As Long
    Dim Y As Integer
    X = 100
    Y = 5
    X = Y
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse source");
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze_file(&ast);

    assert!(result.is_ok(), "Expected valid numeric assignments to succeed");
}

#[test]
fn test_duplicate_symbol_detection() {
    let source = r#"
Sub Main()
    Dim X As Long
    Dim X As String
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse source");
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze_file(&ast);

    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| matches!(e, SemanticError::DuplicateSymbol { ref name, .. } if name.eq_ignore_ascii_case("X"))));
}

#[test]
fn test_argument_count_mismatch() {
    let source = r#"
Sub Process(ByVal A As Long, ByVal B As Long)
End Sub

Sub Main()
    Process 10
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse source");
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze_file(&ast);

    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| matches!(e, SemanticError::ArgumentCountMismatch { ref proc_name, expected: 2, found: 1, .. } if proc_name.eq_ignore_ascii_case("Process"))));
}

#[test]
fn test_option_explicit_enforcement() {
    let source = r#"
Sub Main()
    Z = 50
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse source");

    // Without Option Explicit, undeclared variables default to Variant (valid VB6)
    let mut analyzer_default = SemanticAnalyzer::new();
    assert!(analyzer_default.analyze_file(&ast).is_ok());

    // With Option Explicit, undeclared variables are an error
    let mut analyzer_strict = SemanticAnalyzer::new();
    analyzer_strict.option_explicit = true;
    let result_strict = analyzer_strict.analyze_file(&ast);
    assert!(result_strict.is_err());
    let errors = result_strict.unwrap_err();
    assert!(errors.iter().any(|e| matches!(e, SemanticError::UndeclaredIdentifier { ref name, .. } if name == "Z")));
}
