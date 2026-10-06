use vb6_compiler::ast::*;
use vb6_compiler::parse_source;

#[test]
fn test_parse_sub_main_msgbox() {
    let source = r#"
Sub Main()
    MsgBox "Hello VB6"
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse Sub Main");
    assert_eq!(ast.declarations.len(), 1);

    match &ast.declarations[0] {
        Declaration::Sub(sub_decl) => {
            assert_eq!(sub_decl.name.to_ascii_uppercase(), "MAIN");
            assert_eq!(sub_decl.params.len(), 0);
            assert_eq!(sub_decl.visibility, Visibility::Default);
            assert_eq!(sub_decl.body.len(), 1);

            match &sub_decl.body[0] {
                Statement::Call {
                    name,
                    arguments,
                    has_call_keyword,
                    ..
                } => {
                    assert_eq!(name.to_ascii_uppercase(), "MSGBOX");
                    assert!(!has_call_keyword);
                    assert_eq!(arguments.len(), 1);

                    match &arguments[0] {
                        Expression::Literal(Literal::String(text)) => {
                            assert_eq!(text, "Hello VB6");
                        }
                        other => panic!("Expected string literal argument, got {:?}", other),
                    }
                }
                other => panic!("Expected Call statement, got {:?}", other),
            }
        }
        other => panic!("Expected Sub declaration, got {:?}", other),
    }
}

#[test]
fn test_parse_case_insensitivity() {
    let source = r#"
sub main()
    msgbox "Hello VB6"
end sub
"#;

    let ast = parse_source(source).expect("Case-insensitive parsing failed");
    assert_eq!(ast.declarations.len(), 1);
    if let Declaration::Sub(s) = &ast.declarations[0] {
        assert_eq!(s.name.to_ascii_uppercase(), "MAIN");
    } else {
        panic!("Expected Sub declaration");
    }
}

#[test]
fn test_parse_with_call_syntax() {
    let source = r#"
Public Sub Main()
    Call MsgBox("Hello VB6")
End Sub
"#;

    let ast = parse_source(source).expect("Parsing with Call keyword failed");
    assert_eq!(ast.declarations.len(), 1);
    if let Declaration::Sub(s) = &ast.declarations[0] {
        assert_eq!(s.visibility, Visibility::Public);
        assert_eq!(s.body.len(), 1);
        if let Statement::Call {
            name,
            arguments,
            has_call_keyword,
            ..
        } = &s.body[0]
        {
            assert_eq!(name.to_ascii_uppercase(), "MSGBOX");
            assert!(has_call_keyword);
            assert_eq!(arguments.len(), 1);
        } else {
            panic!("Expected Call statement");
        }
    } else {
        panic!("Expected Sub declaration");
    }
}

#[test]
fn test_parse_with_comments_and_newlines() {
    let source = r#"
' Module1.bas - Initial test
' Author: Clean Room implementation

Sub Main()
    ' Show greeting box
    MsgBox "Hello VB6"
End Sub
"#;

    let ast = parse_source(source).expect("Parsing with comments failed");
    // Declarations include comments and the sub
    let subs: Vec<_> = ast
        .declarations
        .iter()
        .filter_map(|d| match d {
            Declaration::Sub(s) => Some(s),
            _ => None,
        })
        .collect();
    assert_eq!(subs.len(), 1);
    assert_eq!(subs[0].name.to_ascii_uppercase(), "MAIN");
}
