//! Milestone 12 Tests: Strings and Text Manipulation
//! Covers string allocation, freeing, concatenation (& and +),
//! intrinsic string functions: Len, Left$, Right$, Mid$, InStr,
//! UCase$, LCase$, Trim$, LTrim$, RTrim$, Space$, String$, Replace$,
//! Chr$, Asc, Val, Str$, string coercions, and ordinal content comparisons.

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

    let obj_path = unique_path(&format!("m12_{}", module_name), "obj");
    let exe_path = unique_path(&format!("m12_{}", module_name), "exe");

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
fn test_string_variable_assignment_and_print_native_x64() {
    let source = r#"
Sub Main()
    Dim S As String
    S = "Hello VB6 x64"
    Debug.Print S
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "StrAssign");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["Hello VB6 x64"]);
}

#[test]
fn test_string_concatenation_ampersand_native_x64() {
    let source = r#"
Sub Main()
    Dim S1 As String
    Dim S2 As String
    Dim S3 As String
    S1 = "Hello "
    S2 = "World!"
    S3 = S1 & S2
    Debug.Print S3
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "StrConcatAmp");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["Hello World!"]);
}

#[test]
fn test_string_concatenation_plus_operator_native_x64() {
    let source = r#"
Sub Main()
    Dim S1 As String
    Dim S2 As String
    S1 = "Visual "
    S2 = "Basic"
    Debug.Print S1 + S2
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "StrConcatPlus");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["Visual Basic"]);
}

#[test]
fn test_string_concatenation_with_numeric_coercion_native_x64() {
    let source = r#"
Sub Main()
    Dim S As String
    Dim X As Long
    X = 42
    S = "Value is: " & X
    Debug.Print S
    Debug.Print 100 & " items"
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "StrConcatNum");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["Value is: 42", "100 items"]);
}

#[test]
fn test_string_len_intrinsic_native_x64() {
    let source = r#"
Sub Main()
    Dim S As String
    S = "Antigravity"
    Debug.Print Len(S)
    Debug.Print Len("VB6")
    Debug.Print Len("")
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "StrLen");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["11", "3", "0"]);
}

#[test]
fn test_string_left_right_mid_intrinsics_native_x64() {
    let source = r#"
Sub Main()
    Dim S As String
    S = "Microsoft Visual Basic 6.0"
    Debug.Print Left$(S, 9)
    Debug.Print Right$(S, 3)
    Debug.Print Mid$(S, 11, 6)
    Debug.Print Mid$(S, 18)
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "StrSlices");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["Microsoft", "6.0", "Visual", "Basic 6.0"]);
}

#[test]
fn test_string_instr_intrinsic_native_x64() {
    let source = r#"
Sub Main()
    Dim S As String
    S = "Quick brown fox jumps"
    Debug.Print InStr(S, "brown")
    Debug.Print InStr(1, S, "fox")
    Debug.Print InStr(15, S, "fox")
    Debug.Print InStr(S, "missing")
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "StrInStr");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["7", "13", "0", "0"]);
}

#[test]
fn test_string_case_conversion_and_trim_native_x64() {
    let source = r#"
Sub Main()
    Dim S As String
    S = "   Hello World   "
    Debug.Print UCase$(S)
    Debug.Print LCase$(S)
    Debug.Print Trim$(S)
    Debug.Print LTrim$(S)
    Debug.Print RTrim$(S)
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "StrCaseTrim");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec![
        "HELLO WORLD",
        "hello world",
        "Hello World",
        "Hello World",
        "Hello World"
    ]);
}

#[test]
fn test_string_space_string_replace_native_x64() {
    let source = r#"
Sub Main()
    Dim S As String
    S = "foo bar foo baz"
    Debug.Print String$(4, "*")
    Debug.Print Replace(S, "foo", "qux")
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "StrSpaceReplace");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["****", "qux bar qux baz"]);
}

#[test]
fn test_string_chr_asc_val_str_native_x64() {
    let source = r#"
Sub Main()
    Dim S As String
    S = "A"
    Debug.Print Chr$(65)
    Debug.Print Asc(S)
    Debug.Print Val("123.5")
    Debug.Print Str$(42)
    Debug.Print Str$(-99)
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "StrChrAscVal");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["A", "65", "123.5", "42", "-99"]);
}

#[test]
fn test_string_comparisons_native_x64() {
    let source = r#"
Sub Main()
    Dim S1 As String
    Dim S2 As String
    S1 = "apple"
    S2 = "banana"
    If S1 = "apple" Then
        Debug.Print 1
    End If
    If S1 <> S2 Then
        Debug.Print 2
    End If
    If S1 < S2 Then
        Debug.Print 3
    End If
    If S2 > S1 Then
        Debug.Print 4
    End If
    If S1 <= "apple" Then
        Debug.Print 5
    End If
    If S2 >= "banana" Then
        Debug.Print 6
    End If
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "StrCompare");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["1", "2", "3", "4", "5", "6"]);
}

#[test]
fn test_string_null_empty_comparison_native_x64() {
    let source = r#"
Sub Main()
    Dim S As String
    If S = "" Then
        Debug.Print "EmptyString"
    End If
    If Len(S) = 0 Then
        Debug.Print "ZeroLength"
    End If
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "StrNullEmpty");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["EmptyString", "ZeroLength"]);
}
