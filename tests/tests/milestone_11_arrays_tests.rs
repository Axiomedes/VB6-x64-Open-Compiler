//! Milestone 11 Tests: Arrays and Basic Collections
//! Covers fixed-size arrays (default lower bound 0, explicit bounds with To),
//! multidimensional arrays (2D matrix indexing and bounds),
//! dynamic arrays (Dim Arr() As Type, ReDim, ReDim Preserve),
//! LBound and UBound intrinsic functions, and array indexing read/write.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use vb6_backend::Backend;
use vb6_compiler::ir::compile_to_ir;
use vb6_compiler::parse_source;
use vb6_compiler::semantic::SemanticAnalyzer;
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

    let obj_path = unique_path(&format!("m11_{}", module_name), "obj");
    let exe_path = unique_path(&format!("m11_{}", module_name), "exe");

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
fn test_fixed_1d_array_default_bounds_native_x64() {
    let source = r#"
Sub Main()
    Dim Arr(4) As Long
    Arr(0) = 10
    Arr(1) = 20
    Arr(4) = 50
    Debug.Print Arr(0)
    Debug.Print Arr(1)
    Debug.Print Arr(4)
    Debug.Print LBound(Arr)
    Debug.Print UBound(Arr)
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "Fixed1DDefault");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["10", "20", "50", "0", "4"]);
}

#[test]
fn test_fixed_1d_array_explicit_bounds_native_x64() {
    let source = r#"
Sub Main()
    Dim B(1 To 5) As Long
    Dim I As Long
    For I = 1 To 5
        B(I) = I * 10
    Next I
    Debug.Print B(1)
    Debug.Print B(3)
    Debug.Print B(5)
    Debug.Print LBound(B)
    Debug.Print UBound(B)
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "Fixed1DExplicit");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["10", "30", "50", "1", "5"]);
}

#[test]
fn test_multidimensional_2d_array_native_x64() {
    let source = r#"
Sub Main()
    Dim M(2, 3) As Long
    M(0, 0) = 111
    M(1, 2) = 222
    M(2, 3) = 333
    Debug.Print M(0, 0)
    Debug.Print M(1, 2)
    Debug.Print M(2, 3)
    Debug.Print LBound(M, 1)
    Debug.Print UBound(M, 1)
    Debug.Print LBound(M, 2)
    Debug.Print UBound(M, 2)
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "Matrix2D");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["111", "222", "333", "0", "2", "0", "3"]);
}

#[test]
fn test_dynamic_array_redim_native_x64() {
    let source = r#"
Sub Main()
    Dim D() As Long
    ReDim D(5)
    D(0) = 100
    D(5) = 500
    Debug.Print D(0)
    Debug.Print D(5)
    Debug.Print LBound(D)
    Debug.Print UBound(D)
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "DynamicReDim");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["100", "500", "0", "5"]);
}

#[test]
fn test_dynamic_array_redim_preserve_native_x64() {
    let source = r#"
Sub Main()
    Dim D() As Long
    ReDim D(3)
    D(0) = 10
    D(1) = 20
    D(2) = 30
    D(3) = 40
    ReDim Preserve D(6)
    Debug.Print D(0)
    Debug.Print D(1)
    Debug.Print D(2)
    Debug.Print D(3)
    D(6) = 70
    Debug.Print D(6)
    Debug.Print UBound(D)
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "DynamicReDimPreserve");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["10", "20", "30", "40", "70", "6"]);
}

#[test]
fn test_array_passed_to_function_native_x64() {
    let source = r#"
Function SumElements(A() As Long) As Long
    SumElements = A(0) + A(1) + A(2)
End Function

Sub Main()
    Dim Arr(2) As Long
    Arr(0) = 100
    Arr(1) = 200
    Arr(2) = 300
    Debug.Print SumElements(Arr)
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "ArrayParamTest");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["600"]);
}

#[test]
fn test_negative_bounds_array_native_x64() {
    let source = r#"
Sub Main()
    Dim Arr(-5 To 5) As Long
    Arr(-5) = 111
    Arr(0) = 222
    Arr(5) = 333
    Debug.Print Arr(-5)
    Debug.Print Arr(0)
    Debug.Print Arr(5)
    Debug.Print LBound(Arr)
    Debug.Print UBound(Arr)
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "NegativeBounds");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["111", "222", "333", "-5", "5"]);
}

#[test]
fn test_3d_cube_array_native_x64() {
    let source = r#"
Sub Main()
    Dim Cube(1 To 2, 1 To 2, 1 To 2) As Long
    Cube(1, 1, 1) = 10
    Cube(2, 2, 2) = 99
    Debug.Print Cube(1, 1, 1)
    Debug.Print Cube(2, 2, 2)
    Debug.Print LBound(Cube, 3)
    Debug.Print UBound(Cube, 3)
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "Cube3D");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["10", "99", "1", "2"]);
}

#[test]
fn test_double_array_native_x64() {
    let source = r#"
Sub Main()
    Dim Floats(2) As Double
    Floats(0) = 1.5
    Floats(1) = 2.75
    Floats(2) = Floats(0) + Floats(1)
    Debug.Print Floats(0)
    Debug.Print Floats(1)
    Debug.Print Floats(2)
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "DoubleArray");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["1.5", "2.75", "4.25"]);
}

#[test]
fn test_uninitialized_dynamic_array_bounds_native_x64() {
    let source = r#"
Sub Main()
    Dim EmptyArr() As Long
    Debug.Print LBound(EmptyArr)
    Debug.Print UBound(EmptyArr)
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "UninitializedBounds");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["0", "0"]);
}
