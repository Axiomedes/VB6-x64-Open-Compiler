//! Milestone 15 Tests: Classes (.cls) and VB6 Object Model
//! Covers:
//! 1. Class instantiation with New and field access
//! 2. Property Get and Let
//! 3. Property Set for object references
//! 4. Class methods and Me keyword
//! 5. Class_Initialize lifecycle constructor
//! 6. Class_Terminate lifecycle destructor on Set = Nothing
//! 7. Reference counting with multiple references
//! 8. Object identity: "Is Nothing" and "Is" operator
//! 9. Dim ... As New auto-instantiation
//! 10. Passing class instances as arguments to procedures

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

    let obj_path = unique_path(&format!("m15_{}", module_name), "obj");
    let exe_path = unique_path(&format!("m15_{}", module_name), "exe");

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
fn test_class_instantiation_new_and_field_access() {
    let source = r#"
Class Person
    Public Age As Long
    Public Score As Long
End Class

Sub Main()
    Dim p As Person
    Set p = New Person
    p.Age = 30
    p.Score = 95
    Debug.Print p.Age
    Debug.Print p.Score
    Debug.Print p.Age + p.Score
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "ClassNewField");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["30", "95", "125"]);
}

#[test]
fn test_class_property_get_and_let() {
    let source = r#"
Class Counter
    Private m_val As Long

    Public Property Get Value() As Long
        Value = m_val
    End Property

    Public Property Let Value(v As Long)
        m_val = v * 2
    End Property
End Class

Sub Main()
    Dim c As New Counter
    c.Value = 21
    Debug.Print c.Value
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "ClassPropGetLet");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["42"]);
}

#[test]
fn test_class_property_set_object_reference() {
    let source = r#"
Class Node
    Public Value As Long
End Class

Class Wrapper
    Private m_node As Node

    Public Property Get Target() As Node
        Set Target = m_node
    End Property

    Public Property Set Target(n As Node)
        Set m_node = n
    End Property
End Class

Sub Main()
    Dim w As New Wrapper
    Dim n As New Node
    n.Value = 777
    Set w.Target = n
    Dim res As Node
    Set res = w.Target
    Debug.Print res.Value
    If res Is n Then
        Debug.Print "SameNode"
    End If
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "ClassPropSet");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["777", "SameNode"]);
}

#[test]
fn test_class_methods_and_me_keyword() {
    let source = r#"
Class Calculator
    Public Base As Long

    Public Function Add(x As Long) As Long
        Add = Me.Base + x
    End Function

    Public Function Multiply(x As Long, y As Long) As Long
        Multiply = x * y
    End Function
End Class

Sub Main()
    Dim calc As New Calculator
    calc.Base = 10
    Debug.Print calc.Add(5)
    Debug.Print calc.Multiply(6, 7)
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "ClassMethodsMe");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["15", "42"]);
}

#[test]
fn test_class_initialize_lifecycle() {
    let source = r#"
Class Config
    Public Status As String

    Private Sub Class_Initialize()
        Status = "Ready"
    End Sub
End Class

Sub Main()
    Dim cfg As New Config
    Debug.Print cfg.Status
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "ClassInit");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["Ready"]);
}

#[test]
fn test_class_terminate_lifecycle_on_set_nothing() {
    let source = r#"
Class Resource
    Private Sub Class_Terminate()
        Debug.Print "Resource Terminated"
    End Sub
End Class

Sub Main()
    Dim r As Resource
    Set r = New Resource
    Debug.Print "Before Nothing"
    Set r = Nothing
    Debug.Print "After Nothing"
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "ClassTerm");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["Before Nothing", "Resource Terminated", "After Nothing"]);
}

#[test]
fn test_class_reference_counting_multiple_references() {
    let source = r#"
Class SharedObj
    Private Sub Class_Terminate()
        Debug.Print "SharedObj Destroyed"
    End Sub
End Class

Sub Main()
    Dim o1 As SharedObj
    Dim o2 As SharedObj
    Set o1 = New SharedObj
    Set o2 = o1
    Debug.Print "Release o1"
    Set o1 = Nothing
    Debug.Print "o1 Released"
    Debug.Print "Release o2"
    Set o2 = Nothing
    Debug.Print "o2 Released"
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "ClassRefCount");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec![
        "Release o1",
        "o1 Released",
        "Release o2",
        "SharedObj Destroyed",
        "o2 Released",
    ]);
}

#[test]
fn test_class_is_nothing_and_is_operator() {
    let source = r#"
Class Item
    Public ID As Long
End Class

Sub Main()
    Dim a As Item
    If a Is Nothing Then
        Debug.Print "a is Nothing"
    End If
    Set a = New Item
    If Not (a Is Nothing) Then
        Debug.Print "a is not Nothing"
    End If
    Dim b As Item
    Set b = a
    If a Is b Then
        Debug.Print "a Is b"
    End If
    Dim c As Item
    Set c = New Item
    If Not (a Is c) Then
        Debug.Print "a Is Not c"
    End If
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "ClassIsOp");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec![
        "a is Nothing",
        "a is not Nothing",
        "a Is b",
        "a Is Not c",
    ]);
}

#[test]
fn test_class_dim_as_new_auto_instantiation() {
    let source = r#"
Class Greeter
    Public Message As String

    Private Sub Class_Initialize()
        Message = "Hello from Dim As New"
    End Sub
End Class

Sub Main()
    Dim g As New Greeter
    Debug.Print g.Message
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "ClassDimAsNew");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["Hello from Dim As New"]);
}

#[test]
fn test_class_passed_as_parameter_to_sub() {
    let source = r#"
Class Box
    Public Width As Long
    Public Height As Long
End Class

Sub ResizeBox(b As Box, ByVal newW As Long, ByVal newH As Long)
    b.Width = newW
    b.Height = newH
End Sub

Sub Main()
    Dim myBox As New Box
    myBox.Width = 10
    myBox.Height = 20
    ResizeBox myBox, 50, 80
    Debug.Print myBox.Width
    Debug.Print myBox.Height
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "ClassParam");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["50", "80"]);
}
