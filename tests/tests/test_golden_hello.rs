use vb6_compiler::ast::*;
use vb6_compiler::parse_source;
use vb6_project::VbProject;

#[test]
fn test_golden_001_hello_project_and_ast() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let mut project_path = std::path::PathBuf::from(&manifest_dir).join("golden/001_hello/001_hello.vbp");
    if !project_path.exists() {
        project_path = std::path::PathBuf::from("tests/golden/001_hello/001_hello.vbp");
    }
    assert!(project_path.exists(), "Golden project file does not exist at {:?}", project_path);

    let project_dir = project_path.parent().unwrap().to_path_buf();
    let project = VbProject::load_from_file(&project_path).expect("Failed to load golden .vbp");
    assert_eq!(project.settings.name, "HelloVB6");
    assert_eq!(project.settings.startup, "Sub Main");
    assert_eq!(project.modules.len(), 1);

    let module = &project.modules[0];
    let code = &module.code;
    let ast = parse_source(code).expect("Failed to parse module source");

    assert_eq!(ast.declarations.len(), 1);
    match &ast.declarations[0] {
        Declaration::Sub(s) => {
            assert_eq!(s.name.to_ascii_uppercase(), "MAIN");
            assert_eq!(s.params.len(), 0);
            assert_eq!(s.body.len(), 1);

            match &s.body[0] {
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
                        other => panic!("Unexpected argument: {:?}", other),
                    }
                }
                other => panic!("Unexpected statement: {:?}", other),
            }
        }
        other => panic!("Unexpected declaration: {:?}", other),
    }

    // Save serialized golden AST
    let json = serde_json::to_string_pretty(&ast).expect("Failed to serialize AST to JSON");
    let golden_json_path = project_dir.join("expected_ast.json");
    std::fs::write(&golden_json_path, json).expect("Failed to write expected_ast.json");
}
