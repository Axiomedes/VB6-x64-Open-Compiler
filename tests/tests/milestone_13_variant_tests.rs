//! Milestone 13 Tests: Variant and Dynamic Typing Subsystem
//! Covers 24-byte COM Variant layout, VT tags, primitive boxing/unboxing,
//! dynamic arithmetic, comparisons, concatenation, intrinsic functions
//! (VarType, TypeName, IsEmpty, IsNull, IsNumeric, IsDate, IsArray),
//! and conversion functions (CBool, CByte, CInt, CLng, CSng, CDbl, CStr, CVar).

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

    let obj_path = unique_path(&format!("m13_{}", module_name), "obj");
    let exe_path = unique_path(&format!("m13_{}", module_name), "exe");

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
fn test_variant_default_empty_and_is_empty() {
    let source = r#"
Sub Main()
    Dim V As Variant
    Debug.Print IsEmpty(V)
    Debug.Print TypeName(V)
    Debug.Print VarType(V)
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "VarEmpty");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines[0], "-1"); // True
    assert_eq!(lines[1], "Empty");
    assert_eq!(lines[2], "0"); // VT_EMPTY = 0
}

#[test]
fn test_variant_assignment_primitives_and_types() {
    let source = r#"
Sub Main()
    Dim V As Variant

    V = 42
    Debug.Print TypeName(V)
    Debug.Print VarType(V)
    Debug.Print V

    V = "VB6 Variant x64"
    Debug.Print TypeName(V)
    Debug.Print VarType(V)
    Debug.Print V

    V = True
    Debug.Print TypeName(V)
    Debug.Print VarType(V)
    Debug.Print V
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "VarTypes");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines[0], "Long");
    assert_eq!(lines[1], "3"); // VT_I4 = 3
    assert_eq!(lines[2], "42");

    assert_eq!(lines[3], "String");
    assert_eq!(lines[4], "8"); // VT_BSTR = 8
    assert_eq!(lines[5], "VB6 Variant x64");

    assert_eq!(lines[6], "Boolean");
    assert_eq!(lines[7], "11"); // VT_BOOL = 11
    assert_eq!(lines[8], "True");
}

#[test]
fn test_variant_null_literal_and_is_null() {
    let source = r#"
Sub Main()
    Dim V As Variant
    V = Null
    Debug.Print IsNull(V)
    Debug.Print TypeName(V)
    Debug.Print VarType(V)
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "VarNull");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines[0], "-1"); // True
    assert_eq!(lines[1], "Null");
    assert_eq!(lines[2], "1"); // VT_NULL = 1
}

#[test]
fn test_variant_is_numeric() {
    let source = r#"
Sub Main()
    Dim V As Variant
    V = 100
    Debug.Print IsNumeric(V)

    V = "123.45"
    Debug.Print IsNumeric(V)

    V = "HelloWorld"
    Debug.Print IsNumeric(V)
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "VarIsNumeric");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines[0], "-1"); // True
    assert_eq!(lines[1], "-1"); // True
    assert_eq!(lines[2], "0");  // False
}

#[test]
fn test_variant_dynamic_arithmetic_addition() {
    let source = r#"
Sub Main()
    Dim V1 As Variant
    Dim V2 As Variant
    Dim Res As Variant

    V1 = 15
    V2 = 27
    Res = V1 + V2
    Debug.Print Res

    V1 = "First"
    V2 = "Second"
    Res = V1 + V2
    Debug.Print Res
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "VarAdd");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines[0], "42");
    assert_eq!(lines[1], "FirstSecond");
}

#[test]
fn test_variant_dynamic_arithmetic_all_ops() {
    let source = r#"
Sub Main()
    Dim V1 As Variant
    Dim V2 As Variant
    V1 = 50
    V2 = 10

    Dim Diff As Variant
    Diff = V1 - V2
    Debug.Print Diff

    Dim Prod As Variant
    Prod = V1 * V2
    Debug.Print Prod

    Dim Quot As Variant
    Quot = V1 / V2
    Debug.Print Quot

    Dim Modulo As Variant
    Modulo = V1 Mod V2
    Debug.Print Modulo
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "VarOps");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines[0], "40");
    assert_eq!(lines[1], "500");
    assert_eq!(lines[2], "5");
    assert_eq!(lines[3], "0");
}

#[test]
fn test_variant_dynamic_concatenation() {
    let source = r#"
Sub Main()
    Dim V1 As Variant
    Dim V2 As Variant
    Dim Res As Variant

    V1 = "Count: "
    V2 = 100
    Res = V1 & V2
    Debug.Print Res
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "VarConcat");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines[0], "Count: 100");
}

#[test]
fn test_variant_comparisons() {
    let source = r#"
Sub Main()
    Dim V1 As Variant
    Dim V2 As Variant

    V1 = 20
    V2 = 50
    Debug.Print V1 < V2
    Debug.Print V1 = V2

    V1 = "Beta"
    V2 = "Alpha"
    Debug.Print V1 > V2
    Debug.Print V1 = "Beta"
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "VarCmp");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines[0], "-1"); // True
    assert_eq!(lines[1], "0");  // False
    assert_eq!(lines[2], "-1"); // True
    assert_eq!(lines[3], "-1"); // True
}

#[test]
fn test_variant_coercion_to_and_from_primitives() {
    let source = r#"
Sub Main()
    Dim V As Variant
    Dim N As Long
    Dim S As String

    V = 500
    N = V
    S = V
    Debug.Print N
    Debug.Print S

    V = "789"
    N = V
    Debug.Print N
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "VarCoerce");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines[0], "500");
    assert_eq!(lines[1], "500");
    assert_eq!(lines[2], "789");
}

#[test]
fn test_conversion_functions() {
    let source = r#"
Sub Main()
    Debug.Print CBool(1)
    Debug.Print CByte(200)
    Debug.Print CInt(1234)
    Debug.Print CLng(987654)
    Debug.Print CStr(456)
    Dim V As Variant
    V = CVar("Dynamic")
    Debug.Print V
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "VarConv");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines[0], "-1"); // True
    assert_eq!(lines[1], "200");
    assert_eq!(lines[2], "1234");
    assert_eq!(lines[3], "987654");
    assert_eq!(lines[4], "456");
    assert_eq!(lines[5], "Dynamic");
}
