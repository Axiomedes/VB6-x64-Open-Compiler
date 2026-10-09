//! Milestone 10 Tests: Control Structures
//! Covers If..Then..ElseIf..Else, Select Case (Exact, Range, Is comparison, Else, no fallthrough),
//! For..Next (default +1, custom step, negative step, Exit For),
//! Do..Loop (While/Until at top and bottom, infinite Do..Loop with Exit Do),
//! While..Wend, and nested loops with proper Exit scoping.

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

    let obj_path = unique_path(&format!("m10_{}", module_name), "obj");
    let exe_path = unique_path(&format!("m10_{}", module_name), "exe");

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
fn test_if_then_elseif_else_native_x64() {
    let source = r#"
Sub Main()
    Dim Score As Long
    Score = 85

    If Score >= 90 Then
        Debug.Print 1
    ElseIf Score >= 80 Then
        Debug.Print 2
    ElseIf Score >= 70 Then
        Debug.Print 3
    Else
        Debug.Print 4
    End If
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "IfElseTest");
    assert!(success, "Execution failed");
    assert!(stdout.contains("2"), "Expected branch 2 to execute, got: {}", stdout);
}

#[test]
fn test_for_next_positive_and_negative_step_native_x64() {
    let source = r#"
Sub Main()
    Dim I As Long
    Dim Sum1 As Long
    Sum1 = 0
    For I = 1 To 5
        Sum1 = Sum1 + I
    Next I
    Debug.Print Sum1

    Dim J As Long
    Dim Sum2 As Long
    Sum2 = 0
    For J = 10 To 2 Step -2
        Sum2 = Sum2 + J
    Next J
    Debug.Print Sum2

    Dim K As Long
    Dim Sum3 As Long
    Sum3 = 0
    For K = 1 To 10 Step 3
        Sum3 = Sum3 + K
    Next K
    Debug.Print Sum3
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "ForStepTest");
    assert!(success, "Execution failed");
    // Sum1: 1+2+3+4+5 = 15
    assert!(stdout.contains("15"), "Expected Sum1=15, got: {}", stdout);
    // Sum2: 10+8+6+4+2 = 30
    assert!(stdout.contains("30"), "Expected Sum2=30, got: {}", stdout);
    // Sum3: 1+4+7+10 = 22
    assert!(stdout.contains("22"), "Expected Sum3=22, got: {}", stdout);
}

#[test]
fn test_for_loop_exit_for_native_x64() {
    let source = r#"
Sub Main()
    Dim I As Long
    Dim Total As Long
    Total = 0

    For I = 1 To 100
        Total = Total + I
        If I = 5 Then
            Exit For
        End If
    Next I

    Debug.Print Total
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "ExitForTest");
    assert!(success, "Execution failed");
    // 1+2+3+4+5 = 15
    assert!(stdout.contains("15"), "Expected 15 after Exit For, got: {}", stdout);
}

#[test]
fn test_do_while_and_do_until_entry_condition_native_x64() {
    let source = r#"
Sub Main()
    Dim A As Long
    A = 0
    Do While A < 5
        A = A + 1
    Loop
    Debug.Print A

    Dim B As Long
    B = 0
    Do Until B >= 5
        B = B + 1
    Loop
    Debug.Print B
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "DoEntryTest");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(lines.iter().any(|l| l.contains('5')), "Expected 5, got: {}", stdout);
}

#[test]
fn test_do_loop_while_and_until_exit_condition_native_x64() {
    let source = r#"
Sub Main()
    Dim X As Long
    X = 100
    ' Condition is immediately false at bottom, but body must execute once
    Do
        X = X + 1
    Loop While X < 10

    Debug.Print X

    Dim Y As Long
    Y = 200
    ' Condition is immediately true at bottom, but body must execute once
    Do
        Y = Y + 1
    Loop Until Y > 10

    Debug.Print Y
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "DoExitCondTest");
    assert!(success, "Execution failed");
    assert!(stdout.contains("101"), "Expected 101 for Loop While, got: {}", stdout);
    assert!(stdout.contains("201"), "Expected 201 for Loop Until, got: {}", stdout);
}

#[test]
fn test_infinite_do_loop_with_exit_do_native_x64() {
    let source = r#"
Sub Main()
    Dim Count As Long
    Count = 0

    Do
        Count = Count + 1
        If Count = 7 Then
            Exit Do
        End If
    Loop

    Debug.Print Count
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "ExitDoTest");
    assert!(success, "Execution failed");
    assert!(stdout.contains("7"), "Expected 7 after Exit Do, got: {}", stdout);
}

#[test]
fn test_while_wend_native_x64() {
    let source = r#"
Sub Main()
    Dim N As Long
    Dim Sum As Long
    N = 1
    Sum = 0

    While N <= 4
        Sum = Sum + N
        N = N + 1
    Wend

    Debug.Print Sum
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "WhileWendTest");
    assert!(success, "Execution failed");
    // 1+2+3+4 = 10
    assert!(stdout.contains("10"), "Expected 10 from While..Wend, got: {}", stdout);
}

#[test]
fn test_select_case_all_clauses_and_no_fallthrough_native_x64() {
    let source = r#"
Sub TestSelect(ByVal Val As Long)
    Select Case Val
        Case 1
            Debug.Print 100
        Case 2, 3
            Debug.Print 200
        Case 4 To 10
            Debug.Print 300
        Case Is > 50
            Debug.Print 400
        Case Else
            Debug.Print 500
    End Select
End Sub

Sub Main()
    TestSelect 1
    TestSelect 3
    TestSelect 7
    TestSelect 75
    TestSelect 25
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "SelectCaseTest");
    assert!(success, "Execution failed");
    assert!(stdout.contains("100"), "Expected 100 for exact match");
    assert!(stdout.contains("200"), "Expected 200 for multiple match");
    assert!(stdout.contains("300"), "Expected 300 for range match");
    assert!(stdout.contains("400"), "Expected 400 for comparison match");
    assert!(stdout.contains("500"), "Expected 500 for else match");
}

#[test]
fn test_nested_loops_and_selective_exit_native_x64() {
    let source = r#"
Sub Main()
    Dim I As Long
    Dim J As Long
    Dim OuterHits As Long
    Dim InnerHits As Long
    OuterHits = 0
    InnerHits = 0

    For I = 1 To 3
        OuterHits = OuterHits + 1
        J = 0
        Do
            J = J + 1
            InnerHits = InnerHits + 1
            If J = 2 Then
                Exit Do
            End If
        Loop
    Next I

    Debug.Print OuterHits
    Debug.Print InnerHits
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "NestedLoopsTest");
    assert!(success, "Execution failed");
    // OuterHits = 3, InnerHits = 3 * 2 = 6
    assert!(stdout.contains("3"), "Expected 3 outer iterations, got: {}", stdout);
    assert!(stdout.contains("6"), "Expected 6 inner iterations, got: {}", stdout);
}

#[test]
fn test_semantic_invalid_exit_statements() {
    let invalid_exit_for = r#"
Sub Main()
    Exit For
End Sub
"#;
    let ast1 = parse_source(invalid_exit_for).expect("Failed to parse");
    let mut sem1 = SemanticAnalyzer::new();
    let res1 = sem1.analyze_file(&ast1);
    assert!(res1.is_err(), "Expected error for Exit For outside For loop");
    let errs1 = res1.unwrap_err();
    assert!(
        errs1.iter().any(|e| matches!(e, SemanticError::InvalidExitStatement { target, .. } if target == "For")),
        "Expected InvalidExitStatement for For, got: {:?}",
        errs1
    );

    let invalid_exit_do = r#"
Sub Main()
    Exit Do
End Sub
"#;
    let ast2 = parse_source(invalid_exit_do).expect("Failed to parse");
    let mut sem2 = SemanticAnalyzer::new();
    let res2 = sem2.analyze_file(&ast2);
    assert!(res2.is_err(), "Expected error for Exit Do outside Do loop");
    let errs2 = res2.unwrap_err();
    assert!(
        errs2.iter().any(|e| matches!(e, SemanticError::InvalidExitStatement { target, .. } if target == "Do")),
        "Expected InvalidExitStatement for Do, got: {:?}",
        errs2
    );
}
