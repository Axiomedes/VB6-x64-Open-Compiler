//! Milestone 9 Tests: Variables and Basic Assignment
//! Covers Dim (local stack), Dim/Private/Public (module-level global),
//! Static procedure variables, variable initialization defaults,
//! implicit coercions, and scoping/shadowing.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use vb6_backend::Backend;
use vb6_compiler::ir::compile_to_ir;
use vb6_compiler::parse_source;
use vb6_compiler::semantic::{SemanticAnalyzer, SemanticError};
use vb6_linker::Linker;

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_path(prefix: &str, ext: &str) -> PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("{}_{}_{}_{}.{}", prefix, std::process::id(), time, id, ext))
}

fn compile_and_run(source: &str, module_name: &str) -> (bool, String) {
    let ast = parse_source(source).expect("Failed to parse source");
    let mut sem = SemanticAnalyzer::new();
    sem.analyze_file(&ast).expect("Semantic analysis failed");

    let ir = compile_to_ir(&ast, module_name);

    let obj_path = unique_path(&format!("m9_{}", module_name), "obj");
    let exe_path = unique_path(&format!("m9_{}", module_name), "exe");

    let backend = Backend::new("x64");
    backend
        .compile_to_object(&ir, &obj_path, true)
        .expect("Backend failed to emit object");

    let linker = Linker::new().expect("Failed to create linker");
    linker
        .link_executable(&[&obj_path], &exe_path)
        .expect("Linker failed to produce executable");

    let output = Command::new(&exe_path)
        .output()
        .expect("Failed to run executable");

    let _ = std::fs::remove_file(&obj_path);
    let _ = std::fs::remove_file(&exe_path);

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    (output.status.success(), stdout)
}

#[test]
fn test_default_initial_values_native_x64() {
    let source = r#"
Sub Main()
    Dim b As Byte
    Dim i As Integer
    Dim l As Long
    Dim flag As Boolean
    If b = 0 And i = 0 And l = 0 And flag = False Then
        Debug.Print 777
    End If
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "DefaultsTest");
    assert!(success, "Executable failed");
    assert!(stdout.contains("777"), "Expected 777 in stdout, got: {}", stdout);
}

#[test]
fn test_module_level_variables_shared_across_subs() {
    let source = r#"
Dim Counter As Long
Public Multiplier As Long

Sub Increment()
    Counter = Counter + Multiplier
End Sub

Sub Main()
    Counter = 10
    Multiplier = 5
    Call Increment()
    Call Increment()
    Debug.Print Counter
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "ModuleVarsTest");
    assert!(success, "Executable failed");
    assert!(stdout.contains("20"), "Expected 20 in stdout, got: {}", stdout);
}

#[test]
fn test_static_procedure_variable_persists_across_calls() {
    let source = r#"
Sub Accumulate()
    Static Total As Long
    Total = Total + 10
    Debug.Print Total
End Sub

Sub Main()
    Call Accumulate()
    Call Accumulate()
    Call Accumulate()
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "StaticVarsTest");
    assert!(success, "Executable failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert!(lines.contains(&"10"), "Expected 10 in output lines: {:?}", lines);
    assert!(lines.contains(&"20"), "Expected 20 in output lines: {:?}", lines);
    assert!(lines.contains(&"30"), "Expected 30 in output lines: {:?}", lines);
}

#[test]
fn test_local_variable_shadows_module_variable() {
    let source = r#"
Dim ShadowVar As Long

Sub ChangeLocal()
    Dim ShadowVar As Long
    ShadowVar = 999
End Sub

Sub Main()
    ShadowVar = 42
    Call ChangeLocal()
    Debug.Print ShadowVar
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "ShadowTest");
    assert!(success, "Executable failed");
    assert!(stdout.contains("42"), "Expected 42 in stdout, got: {}", stdout);
}

#[test]
fn test_implicit_coercions_in_assignment() {
    let source = r#"
Sub Main()
    Dim B As Byte
    Dim I As Integer
    Dim L As Long
    Dim D As Double

    B = 10
    I = B
    L = I
    D = L
    Debug.Print D
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "CoercionTest");
    assert!(success, "Executable failed");
    assert!(stdout.contains("10"), "Expected 10 in stdout, got: {}", stdout);
}

#[test]
fn test_multiple_declarations_in_single_dim() {
    let source = r#"
Sub Main()
    Dim X As Long, Y As Long, Z As Long
    X = 15
    Y = 25
    Z = X + Y
    Debug.Print Z
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "MultiDimTest");
    assert!(success, "Executable failed");
    assert!(stdout.contains("40"), "Expected 40 in stdout, got: {}", stdout);
}

#[test]
fn test_semantic_object_required_for_set() {
    let source = r#"
Sub Main()
    Dim X As Long
    Set X = 10
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse");
    let mut sem = SemanticAnalyzer::new();
    let res = sem.analyze_file(&ast);

    assert!(res.is_err(), "Expected error for Set on Long variable");
    let errors = res.unwrap_err();
    assert!(
        errors.iter().any(|e| matches!(e, SemanticError::ObjectRequired { .. })),
        "Expected ObjectRequired error, found: {:?}",
        errors
    );
}

#[test]
fn test_semantic_redim_allowed_on_declared_variable() {
    let source = r#"
Sub Main()
    Dim Arr() As Long
    ReDim Arr(10)
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse");
    let mut sem = SemanticAnalyzer::new();
    let res = sem.analyze_file(&ast);

    assert!(res.is_ok(), "ReDim on declared array should not cause duplicate symbol error: {:?}", res.err());
}
