//! Milestone 14 Tests: User-Defined Types (UDT) and Enums
//! Covers Type ... End Type, Enum ... End Enum, nested UDTs,
//! fixed-length strings (String * n), With ... End With (including nested With),
//! ByRef UDT passing, structure copy assignment, and arrays of UDTs.

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

    let obj_path = unique_path(&format!("m14_{}", module_name), "obj");
    let exe_path = unique_path(&format!("m14_{}", module_name), "exe");

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
fn test_udt_declaration_and_field_assignment() {
    let source = r#"
Type Point
    X As Long
    Y As Long
End Type

Sub Main()
    Dim pt As Point
    pt.X = 10
    pt.Y = 20
    Debug.Print pt.X
    Debug.Print pt.Y
    Debug.Print pt.X + pt.Y
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "UdtDecl");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["10", "20", "30"]);
}

#[test]
fn test_udt_fixed_length_strings() {
    let source = r#"
Type Employee
    ID As Long
    Name As String * 10
End Type

Sub Main()
    Dim emp As Employee
    emp.ID = 101
    emp.Name = "Alice"
    Debug.Print emp.ID
    Debug.Print emp.Name
    Debug.Print Len(emp.Name)
    emp.Name = "Constantinople"
    Debug.Print emp.Name
    Debug.Print Len(emp.Name)
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "UdtFixedStr");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().collect();
    // VB6 space-pads short strings to fixed length and truncates long strings
    assert_eq!(lines[0].trim(), "101");
    assert_eq!(lines[1], "Alice     ");
    assert_eq!(lines[2].trim(), "10");
    assert_eq!(lines[3], "Constantin");
    assert_eq!(lines[4].trim(), "10");
}

#[test]
fn test_udt_nested_structures() {
    let source = r#"
Type Address
    City As String
    ZipCode As Long
End Type

Type Person
    Name As String
    Addr As Address
    Age As Long
End Type

Sub Main()
    Dim p As Person
    p.Name = "Bob"
    p.Age = 30
    p.Addr.City = "Springfield"
    p.Addr.ZipCode = 12345
    Debug.Print p.Name
    Debug.Print p.Age
    Debug.Print p.Addr.City
    Debug.Print p.Addr.ZipCode
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "UdtNested");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["Bob", "30", "Springfield", "12345"]);
}

#[test]
fn test_udt_with_statement() {
    let source = r#"
Type Point
    X As Long
    Y As Long
End Type

Sub Main()
    Dim pt As Point
    With pt
        .X = 100
        .Y = 200
        Debug.Print .X
        Debug.Print .Y
    End With
    Debug.Print pt.X + pt.Y
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "UdtWith");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["100", "200", "300"]);
}

#[test]
fn test_udt_nested_with_statement() {
    let source = r#"
Type Address
    City As String
    ZipCode As Long
End Type

Type Person
    Name As String
    Addr As Address
End Type

Sub Main()
    Dim p As Person
    With p
        .Name = "Charlie"
        With .Addr
            .City = "Boston"
            .ZipCode = 2101
            Debug.Print .City
            Debug.Print .ZipCode
        End With
        Debug.Print .Name
    End With
    Debug.Print p.Addr.City
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "UdtNestedWith");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["Boston", "2101", "Charlie", "Boston"]);
}

#[test]
fn test_udt_pass_by_ref_to_sub() {
    let source = r#"
Type Point
    X As Long
    Y As Long
End Type

Sub MovePoint(ByRef pt As Point, ByVal dx As Long, ByVal dy As Long)
    pt.X = pt.X + dx
    pt.Y = pt.Y + dy
End Sub

Sub Main()
    Dim pt As Point
    pt.X = 15
    pt.Y = 25
    MovePoint pt, 10, 20
    Debug.Print pt.X
    Debug.Print pt.Y
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "UdtByRef");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["25", "45"]);
}

#[test]
fn test_udt_structure_copy_assignment() {
    let source = r#"
Type Point
    X As Long
    Y As Long
End Type

Sub Main()
    Dim p1 As Point
    Dim p2 As Point
    p1.X = 42
    p1.Y = 84
    p2 = p1
    Debug.Print p2.X
    Debug.Print p2.Y
    p2.X = 999
    Debug.Print p1.X
    Debug.Print p2.X
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "UdtCopy");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["42", "84", "42", "999"]);
}

#[test]
fn test_enum_auto_increment_and_explicit_values() {
    let source = r#"
Enum Days
    Sunday = 1
    Monday
    Tuesday
    Wednesday = 10
    Thursday
End Enum

Sub Main()
    Dim d1 As Long
    Dim d2 As Long
    Dim d3 As Long
    Dim d4 As Long
    Dim d5 As Long
    d1 = Sunday
    d2 = Monday
    d3 = Tuesday
    d4 = Wednesday
    d5 = Thursday
    Debug.Print d1
    Debug.Print d2
    Debug.Print d3
    Debug.Print d4
    Debug.Print d5
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "EnumAutoIncr");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["1", "2", "3", "10", "11"]);
}

#[test]
fn test_enum_in_select_case_and_expressions() {
    let source = r#"
Enum LogLevel
    DebugLevel = 1
    InfoLevel = 2
    WarningLevel = 3
    ErrorLevel = 4
End Enum

Sub PrintLevel(ByVal lvl As Long)
    Select Case lvl
        Case DebugLevel
            Debug.Print "DEBUG"
        Case InfoLevel
            Debug.Print "INFO"
        Case WarningLevel
            Debug.Print "WARN"
        Case ErrorLevel
            Debug.Print "ERROR"
        Case Else
            Debug.Print "UNKNOWN"
    End Select
End Sub

Sub Main()
    PrintLevel LogLevel.InfoLevel
    PrintLevel WarningLevel
    Debug.Print LogLevel.DebugLevel + LogLevel.ErrorLevel
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "EnumSelectCase");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["INFO", "WARN", "5"]);
}

#[test]
fn test_array_of_udts() {
    let source = r#"
Type Point
    X As Long
    Y As Long
End Type

Sub Main()
    Dim pts(2) As Point
    pts(0).X = 10
    pts(0).Y = 20
    pts(1).X = 30
    pts(1).Y = 40
    Debug.Print pts(0).X
    Debug.Print pts(0).Y
    Debug.Print pts(1).X
    Debug.Print pts(1).Y
    Debug.Print pts(0).X + pts(1).X
End Sub
"#;

    let (success, stdout) = compile_and_run(source, "UdtArray");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["10", "20", "30", "40", "40"]);
}
