//! Milestone 16 Tests: Forms (.frm), Controls, and Win32 UI Model in x64
//! Covers:
//! 1. test_frm_desugaring_to_form_class_and_controls
//! 2. test_form_initial_properties_and_control_tree
//! 3. test_form_load_lifecycle_event
//! 4. test_control_click_event_dispatch
//! 5. test_textbox_change_and_text_manipulation
//! 6. test_checkbox_and_optionbutton_value_events
//! 7. test_timer_interval_and_tick_event
//! 8. test_form_unload_and_queryunload_lifecycle
//! 9. test_vbp_startup_form_project_compilation
//! 10. test_native_win32_ui_runtime_and_do_events

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use vb6_backend::Backend;
use vb6_compiler::ir::compile_to_ir;
use vb6_compiler::parse_source;
use vb6_compiler::semantic::SemanticAnalyzer;
use vb6_linker::Linker;
use vb6_project::{FRMDesugarer, FRMParser, ProjectLoader};

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

    let obj_path = unique_path(&format!("m16_{}", module_name), "obj");
    let exe_path = unique_path(&format!("m16_{}", module_name), "exe");

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
fn test_frm_desugaring_to_form_class_and_controls() {
    let frm_content = r#"VERSION 5.00
Begin VB.Form Form1 
   Caption         =   "Test Form"
   ClientHeight    =   3195
   ClientLeft      =   60
   ClientTop       =   345
   ClientWidth     =   4680
   Begin VB.CommandButton Command1 
      Caption         =   "Click Me"
      Height          =   495
      Left            =   1200
      Top             =   1080
      Width           =   1215
   End
   Begin VB.TextBox Text1 
      Height          =   495
      Left            =   1200
      Top             =   1800
      Width           =   1215
      Text            =   "Initial Text"
   End
   Begin VB.Label Label1 
      Caption         =   "Status"
      Height          =   255
      Left            =   1200
      Top             =   2500
      Width           =   1215
   End
End
Attribute VB_Name = "Form1"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = True
Attribute VB_Exposed = False

Private Sub Command1_Click()
    Text1.Text = "Clicked"
End Sub
"#;

    let dummy_path = Path::new("Form1.frm");
    let form_module = FRMParser::parse(frm_content, dummy_path);
    assert_eq!(form_module.name, "Form1");
    assert!(form_module.root_control.is_some());

    let desugared_vb = FRMDesugarer::desugar(&form_module);
    assert!(desugared_vb.contains("Class Form1"));
    assert!(desugared_vb.contains("Public Command1 As CommandButton"));
    assert!(desugared_vb.contains("Public Text1 As TextBox"));
    assert!(desugared_vb.contains("Public Label1 As Label"));
    assert!(desugared_vb.contains("Public Sub Command1_Click()"));

    // Verify it parses cleanly and semantically verifies
    let ast = parse_source(&desugared_vb).expect("Desugared code must parse cleanly");
    let mut sem = SemanticAnalyzer::new();
    sem.analyze_file(&ast).expect("Desugared code must pass semantic analysis");
}

#[test]
fn test_form_initial_properties_and_control_tree() {
    let frm_content = r#"VERSION 5.00
Begin VB.Form FormMain 
   Caption         =   "Main Application Window"
   ClientHeight    =   4800
   ClientLeft      =   1500
   ClientTop       =   1200
   ClientWidth     =   6400
   Begin VB.CommandButton Command1 
      Caption         =   "Submit"
      Height          =   300
      Left            =   120
      Top             =   240
      Width           =   800
   End
   Begin VB.TextBox Text1 
      Height          =   250
      Left            =   100
      Top             =   100
      Width           =   1000
      Text            =   "Hello VB6"
   End
   Begin VB.Label Label1 
      Caption         =   "Username"
      Height          =   200
      Left            =   50
      Top             =   50
      Width           =   500
   End
   Begin VB.CheckBox Check1 
      Caption         =   "Remember Me"
      Height          =   200
      Left            =   100
      Top             =   300
      Width           =   1000
      Value           =   1
   End
   Begin VB.OptionButton Option1 
      Caption         =   "Active Mode"
      Height          =   200
      Left            =   100
      Top             =   500
      Width           =   1000
      Value           =   1
   End
   Begin VB.Frame Frame1 
      Caption         =   "Preferences"
      Height          =   1500
      Left            =   50
      Top             =   250
      Width           =   2000
   End
   Begin VB.Timer Timer1 
      Interval        =   1000
      Left            =   0
      Top             =   0
   End
End
Attribute VB_Name = "FormMain"
"#;

    let dummy_path = Path::new("FormMain.frm");
    let form_module = FRMParser::parse(frm_content, dummy_path);
    let desugared_vb = FRMDesugarer::desugar(&form_module);

    let full_source = format!(
        "{}\nSub Main()\n    Dim f As FormMain\n    Set f = New FormMain\n    Debug.Print f.Name\n    Debug.Print f.Caption\n    Debug.Print f.Command1.Caption\n    Debug.Print f.Text1.Text\n    Debug.Print f.Label1.Caption\n    Debug.Print f.Check1.Value\n    Debug.Print f.Option1.Value\n    Debug.Print f.Frame1.Caption\n    Debug.Print f.Timer1.Interval\nEnd Sub\n",
        desugared_vb
    );

    let (success, stdout) = compile_and_run(&full_source, "FormInitialProps");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(
        lines,
        vec![
            "FormMain",
            "Main Application Window",
            "Submit",
            "Hello VB6",
            "Username",
            "1",
            "-1",
            "Preferences",
            "1000"
        ]
    );
}

#[test]
fn test_form_load_lifecycle_event() {
    let frm_content = r#"VERSION 5.00
Begin VB.Form Form1 
   Caption         =   "Lifecycle Form"
   ClientHeight    =   3000
   ClientLeft      =   1000
   ClientTop       =   1000
   ClientWidth     =   4000
   Begin VB.TextBox Text1 
      Text            =   "Initial"
   End
End
Attribute VB_Name = "Form1"

Private Sub Form_Load()
    Text1.Text = "Initialized by Form_Load"
End Sub
"#;

    let dummy_path = Path::new("Form1.frm");
    let form_module = FRMParser::parse(frm_content, dummy_path);
    let desugared_vb = FRMDesugarer::desugar(&form_module);

    let full_source = format!(
        "{}\nSub Main()\n    Dim f As Form1\n    Set f = New Form1\n    f.Show\n    Debug.Print f.Text1.Text\n    Debug.Print f.Visible\nEnd Sub\n",
        desugared_vb
    );

    let (success, stdout) = compile_and_run(&full_source, "FormLoadEvent");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["Initialized by Form_Load", "-1"]);
}

#[test]
fn test_control_click_event_dispatch() {
    let frm_content = r#"VERSION 5.00
Begin VB.Form Form1 
   Caption         =   "Click Test"
   Begin VB.CommandButton Command1 
      Caption         =   "Click Me"
   End
   Begin VB.Label Label1 
      Caption         =   "Initial Label"
   End
End
Attribute VB_Name = "Form1"

Private Sub Command1_Click()
    Label1.Caption = "Clicked!"
End Sub
"#;

    let dummy_path = Path::new("Form1.frm");
    let form_module = FRMParser::parse(frm_content, dummy_path);
    let desugared_vb = FRMDesugarer::desugar(&form_module);

    let full_source = format!(
        "{}\nSub Main()\n    Dim f As Form1\n    Set f = New Form1\n    Debug.Print f.Label1.Caption\n    f.Command1_Click\n    Debug.Print f.Label1.Caption\nEnd Sub\n",
        desugared_vb
    );

    let (success, stdout) = compile_and_run(&full_source, "ControlClick");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["Initial Label", "Clicked!"]);
}

#[test]
fn test_textbox_change_and_text_manipulation() {
    let frm_content = r#"VERSION 5.00
Begin VB.Form Form1 
   Caption         =   "Text Test"
   Begin VB.TextBox Text1 
      Text            =   ""
   End
   Begin VB.TextBox Text2 
      Text            =   ""
   End
End
Attribute VB_Name = "Form1"

Public Sub ProcessText()
    Text2.Text = Text1.Text & " [PROCESSED]"
End Sub
"#;

    let dummy_path = Path::new("Form1.frm");
    let form_module = FRMParser::parse(frm_content, dummy_path);
    let desugared_vb = FRMDesugarer::desugar(&form_module);

    let full_source = format!(
        "{}\nSub Main()\n    Dim f As Form1\n    Set f = New Form1\n    f.Text1.Text = \"RawInput\"\n    f.ProcessText\n    Debug.Print f.Text2.Text\nEnd Sub\n",
        desugared_vb
    );

    let (success, stdout) = compile_and_run(&full_source, "TextManipulation");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["RawInput [PROCESSED]"]);
}

#[test]
fn test_checkbox_and_optionbutton_value_events() {
    let frm_content = r#"VERSION 5.00
Begin VB.Form Form1 
   Caption         =   "Value Test"
   Begin VB.CheckBox Check1 
      Caption         =   "Check Me"
      Value           =   0
   End
   Begin VB.OptionButton Option1 
      Caption         =   "Pick Me"
      Value           =   0
   End
End
Attribute VB_Name = "Form1"

Public Sub SetValues()
    Check1.Value = 1
    Option1.Value = True
End Sub
"#;

    let dummy_path = Path::new("Form1.frm");
    let form_module = FRMParser::parse(frm_content, dummy_path);
    let desugared_vb = FRMDesugarer::desugar(&form_module);

    let full_source = format!(
        "{}\nSub Main()\n    Dim f As Form1\n    Set f = New Form1\n    Debug.Print f.Check1.Value\n    Debug.Print f.Option1.Value\n    f.SetValues\n    Debug.Print f.Check1.Value\n    Debug.Print f.Option1.Value\n    If f.Check1.Value = 1 Then\n        Debug.Print \"Check is checked\"\n    End If\n    If f.Option1.Value Then\n        Debug.Print \"Option is selected\"\n    End If\nEnd Sub\n",
        desugared_vb
    );

    let (success, stdout) = compile_and_run(&full_source, "CheckOptionValues");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(
        lines,
        vec![
            "0",
            "0",
            "1",
            "-1",
            "Check is checked",
            "Option is selected"
        ]
    );
}

#[test]
fn test_timer_interval_and_tick_event() {
    let frm_content = r#"VERSION 5.00
Begin VB.Form Form1 
   Caption         =   "Timer Test"
   Begin VB.Timer Timer1 
      Interval        =   500
   End
End
Attribute VB_Name = "Form1"

Public TickCount As Long

Public Sub Timer1_Timer()
    TickCount = TickCount + 10
End Sub
"#;

    let dummy_path = Path::new("Form1.frm");
    let form_module = FRMParser::parse(frm_content, dummy_path);
    let desugared_vb = FRMDesugarer::desugar(&form_module);

    let full_source = format!(
        "{}\nSub Main()\n    Dim f As Form1\n    Set f = New Form1\n    Debug.Print f.Timer1.Interval\n    f.Timer1.Interval = 250\n    Debug.Print f.Timer1.Interval\n    Debug.Print f.TickCount\n    f.Timer1_Timer\n    f.Timer1_Timer\n    Debug.Print f.TickCount\nEnd Sub\n",
        desugared_vb
    );

    let (success, stdout) = compile_and_run(&full_source, "TimerTick");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["500", "250", "0", "20"]);
}

#[test]
fn test_form_unload_and_queryunload_lifecycle() {
    let frm_content = r#"VERSION 5.00
Begin VB.Form Form1 
   Caption         =   "Unload Test"
End
Attribute VB_Name = "Form1"

Public UnloadState As Long

Private Sub Form_Unload(Cancel As Integer)
    UnloadState = 99
End Sub
"#;

    let dummy_path = Path::new("Form1.frm");
    let form_module = FRMParser::parse(frm_content, dummy_path);
    let desugared_vb = FRMDesugarer::desugar(&form_module);

    let full_source = format!(
        "{}\nSub Main()\n    Dim f As Form1\n    Set f = New Form1\n    f.Show\n    Debug.Print f.Visible\n    Debug.Print f.UnloadState\n    Unload f\n    Debug.Print f.Visible\n    Debug.Print f.UnloadState\nEnd Sub\n",
        desugared_vb
    );

    let (success, stdout) = compile_and_run(&full_source, "FormUnloadLifecycle");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["-1", "0", "0", "99"]);
}

#[test]
fn test_vbp_startup_form_project_compilation() {
    let temp_dir = unique_path("vbp_proj_test", "");
    std::fs::create_dir_all(&temp_dir).expect("Failed to create temp project directory");

    let frm_content = r#"VERSION 5.00
Begin VB.Form Form1 
   Caption         =   "Startup Form"
   ClientHeight    =   3000
   ClientLeft      =   1000
   ClientTop       =   1000
   ClientWidth     =   4000
   Begin VB.TextBox Text1 
      Text            =   "Hello from VBP"
   End
End
Attribute VB_Name = "Form1"

Private Sub Form_Load()
    Debug.Print "Startup Form Loaded: " & Text1.Text
End Sub
"#;

    let vbp_content = r#"Type=Exe
Form=Form1.frm
Startup="Form1"
Name="Project1"
"#;

    let frm_path = temp_dir.join("Form1.frm");
    let vbp_path = temp_dir.join("Project1.vbp");

    std::fs::write(&frm_path, frm_content).expect("Failed to write Form1.frm");
    std::fs::write(&vbp_path, vbp_content).expect("Failed to write Project1.vbp");

    let project = ProjectLoader::load_project(&vbp_path).expect("Failed to load project");
    assert_eq!(project.settings.name, "Project1");
    assert_eq!(project.settings.startup, "Form1");
    assert_eq!(project.forms.len(), 1);

    // Desugar form and synthesize startup wrapper
    let form = &project.forms[0];
    let desugared = FRMDesugarer::desugar(form);
    let startup_code = format!(
        "{}\nPublic Sub Main()\n    Set Form1 = New Form1\n    Form1.Show\n    vb6_do_events\nEnd Sub\n",
        desugared
    );

    let (success, stdout) = compile_and_run(&startup_code, "VbpStartupForm");

    let _ = std::fs::remove_dir_all(&temp_dir);

    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["Startup Form Loaded: Hello from VBP"]);
}

#[test]
fn test_native_win32_ui_runtime_and_do_events() {
    let frm_content = r#"VERSION 5.00
Begin VB.Form Form1 
   Caption         =   "Win32 Runtime Form"
   ClientHeight    =   3000
   ClientLeft      =   1000
   ClientTop       =   1000
   ClientWidth     =   4000
   Begin VB.CommandButton Command1 
      Caption         =   "Button1"
   End
   Begin VB.TextBox Text1 
      Text            =   "Box1"
   End
End
Attribute VB_Name = "Form1"
"#;

    let dummy_path = Path::new("Form1.frm");
    let form_module = FRMParser::parse(frm_content, dummy_path);
    let desugared_vb = FRMDesugarer::desugar(&form_module);

    let full_source = format!(
        "{}\nSub Main()\n    Dim f As Form1\n    Set f = New Form1\n    f.Show\n    Debug.Print f.hWnd <> 0\n    Debug.Print f.Command1.hWnd <> 0\n    Debug.Print f.Text1.hWnd <> 0\n    DoEvents\n    f.Hide\n    Debug.Print f.Visible\nEnd Sub\n",
        desugared_vb
    );

    let (success, stdout) = compile_and_run(&full_source, "NativeWin32DoEvents");
    assert!(success, "Execution failed");
    let lines: Vec<&str> = stdout.lines().map(|s| s.trim()).collect();
    assert_eq!(lines, vec!["-1", "-1", "-1", "0"]);
}
