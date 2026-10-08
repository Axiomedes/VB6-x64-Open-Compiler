use std::path::Path;
use vb6_project::models::*;
use vb6_project::*;

#[test]
fn test_vbp_parser_complete() {
    let vbp_content = r#"
Type=Exe
Reference=*\G{00020430-0000-0000-C000-000000000046}#2.0#0#C:\Windows\System32\stdole2.tlb#OLE Automation
Object={831FDD16-0C5C-11D2-A9FC-0000F8754DA1}#2.0#0; MSCOMCTL.OCX
Module=Module1; Sources\Module1.bas
Class=Class1; Sources\Class1.cls
Form=Forms\Form1.frm
UserControl=Controls\UserControl1.ctl
PropertyPage=Pages\PropertyPage1.pag
Startup="Sub Main"
Command32=""
Name="ComplexApp"
HelpContextID="42"
CompatibleMode="0"
MajorVer=2
MinorVer=5
RevisionVer=1
AutoIncrementVer=1
ServerSupportFiles=0
VersionCompanyName="ACME Corp"
CompilationType=0
OptimizationType=0
FavorPentiumPro(tm)=1
CodeViewDebugInfo=0
NoAliasing=0
BoundsCheck=1
OverflowCheck=1
FlPointCheck=0
FDIVCheck=0
UnroundedFP=0
StartMode=0
Unattended=0
Retained=0
ThreadPerObject=0
MaxNumberOfThreads=1
"#;

    let base_dir = Path::new("C:/MyApp");
    let raw = VBPParser::parse(vbp_content, base_dir);

    assert_eq!(raw.settings.name, "ComplexApp");
    assert_eq!(raw.settings.project_type, "Exe");
    assert_eq!(raw.settings.startup, "Sub Main");
    assert_eq!(raw.settings.major_ver, 2);
    assert_eq!(raw.settings.minor_ver, 5);
    assert_eq!(raw.settings.revision_ver, 1);
    assert!(raw.settings.auto_increment_ver);
    assert!(raw.settings.favor_pentium_pro);
    assert!(raw.settings.bounds_check);
    assert!(raw.settings.overflow_check);
    assert_eq!(raw.settings.version_company_name, "ACME Corp");

    // References
    assert_eq!(raw.references.len(), 1);
    assert_eq!(raw.references[0].guid, "{00020430-0000-0000-C000-000000000046}");
    assert_eq!(raw.references[0].major_ver, 2);
    assert_eq!(raw.references[0].minor_ver, 0);
    assert_eq!(raw.references[0].name, "OLE Automation");

    // Components (OCX)
    assert_eq!(raw.components.len(), 1);
    assert_eq!(raw.components[0].guid, "{831FDD16-0C5C-11D2-A9FC-0000F8754DA1}");
    assert_eq!(raw.components[0].file, "MSCOMCTL.OCX");

    // Files
    assert_eq!(raw.module_files.len(), 1);
    assert_eq!(raw.class_files.len(), 1);
    assert_eq!(raw.form_files.len(), 1);
    assert_eq!(raw.user_control_files.len(), 1);
    assert_eq!(raw.property_page_files.len(), 1);
}

#[test]
fn test_frm_parser_nested_controls_and_code() {
    let frm_content = r#"
VERSION 5.00
Begin VB.Form MainForm 
   Caption         =   "Main Form"
   ClientHeight    =   4500
   ClientWidth     =   6000
   Icon            =   "MainForm.frx":0000
   Begin VB.Frame Frame1 
      Caption         =   "Group A"
      Height          =   2000
      Left            =   300
      Top             =   400
      Width           =   4000
      Begin VB.CommandButton CmdSave 
         Caption         =   "Save Now"
         Height          =   450
         Index           =   0
         Left            =   500
         Top             =   600
         Width           =   1200
      End
      Begin VB.CommandButton CmdSave 
         Caption         =   "Cancel"
         Height          =   450
         Index           =   1
         Left            =   1900
         Top             =   600
         Width           =   1200
      End
   End
End
Attribute VB_Name = "MainForm"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = True
Attribute VB_Exposed = False

Private Sub CmdSave_Click(Index As Integer)
    MsgBox "Clicked button " & Index
End Sub
"#;

    let path = Path::new("C:/MyApp/Forms/MainForm.frm");
    let form = FRMParser::parse(frm_content, path);

    assert_eq!(form.name, "MainForm");
    assert_eq!(form.attributes.get("VB_PredeclaredId").unwrap(), "True");

    let root = form.root_control.expect("Expected root Form control");
    assert_eq!(root.control_type, "VB.Form");
    assert_eq!(root.name, "MainForm");

    // Form properties
    assert_eq!(
        root.properties.get("Caption").unwrap(),
        &ControlPropertyValue::Literal("Main Form".to_string())
    );
    assert_eq!(
        root.properties.get("Icon").unwrap(),
        &ControlPropertyValue::FrxReference {
            frx_file: "MainForm.frx".to_string(),
            offset: 0x0000,
        }
    );

    // Frame child
    assert_eq!(root.children.len(), 1);
    let frame = &root.children[0];
    assert_eq!(frame.control_type, "VB.Frame");
    assert_eq!(frame.name, "Frame1");

    // Command buttons inside Frame
    assert_eq!(frame.children.len(), 2);
    let btn0 = &frame.children[0];
    assert_eq!(btn0.name, "CmdSave");
    assert_eq!(btn0.index, Some(0));
    assert_eq!(
        btn0.properties.get("Caption").unwrap(),
        &ControlPropertyValue::Literal("Save Now".to_string())
    );

    let btn1 = &frame.children[1];
    assert_eq!(btn1.name, "CmdSave");
    assert_eq!(btn1.index, Some(1));
    assert_eq!(
        btn1.properties.get("Caption").unwrap(),
        &ControlPropertyValue::Literal("Cancel".to_string())
    );

    // Code extraction
    assert!(form.code.contains("Private Sub CmdSave_Click"));
    assert!(form.code.contains("Clicked button"));
}

#[test]
fn test_cls_loader_properties_and_code() {
    let cls_content = r#"
VERSION 1.0 CLASS
BEGIN
  MultiUse = -1  'True
  Persistable = 0  'NotPersistable
  DataBindingBehavior = 0  'vbNone
  DataSourceBehavior = 0  'vbNone
  MTSTransactionMode = 0  'NotAnMTSObject
END
Attribute VB_Name = "Customer"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = True
Attribute VB_PredeclaredId = False
Attribute VB_Exposed = True

Private m_Name As String

Public Property Get Name() As String
    Name = m_Name
End Property
"#;

    let path = Path::new("Customer.cls");
    let cls = CLSLoader::parse(cls_content, path, None);

    assert_eq!(cls.name, "Customer");
    assert_eq!(cls.attributes.get("VB_Creatable").unwrap(), "True");
    assert_eq!(cls.attributes.get("VB_Exposed").unwrap(), "True");
    assert_eq!(cls.instancing.as_deref(), Some("-1"));
    assert!(cls.code.contains("Public Property Get Name()"));
}

#[test]
fn test_bas_loader_simple() {
    let bas_content = r#"
Attribute VB_Name = "ModGlobal"
Public Const AppTitle As String = "My VB6 App"

Public Sub Main()
    MsgBox AppTitle
End Sub
"#;

    let path = Path::new("ModGlobal.bas");
    let bmod = BASLoader::parse(bas_content, path, None);

    assert_eq!(bmod.name, "ModGlobal");
    assert!(bmod.code.contains("Public Sub Main()"));
}

#[test]
fn test_frx_loader_reading_offsets() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let frx_path = temp_dir.path().join("TestForm.frx");

    // Construct synthetic .frx:
    // Offset 0x00: length 4 (little endian: [0x04, 0x00, 0x00, 0x00]), bytes: [10, 20, 30, 40]
    // Offset 0x08: length 2 (little endian: [0x02, 0x00, 0x00, 0x00]), bytes: [99, 100]
    let mut frx_bytes = Vec::new();
    // Entry 1 at offset 0
    frx_bytes.extend_from_slice(&4u32.to_le_bytes());
    frx_bytes.extend_from_slice(&[10, 20, 30, 40]);
    // Entry 2 at offset 8
    frx_bytes.extend_from_slice(&2u32.to_le_bytes());
    frx_bytes.extend_from_slice(&[99, 100]);

    std::fs::write(&frx_path, &frx_bytes).expect("Failed to write test .frx");

    let res = FRXLoader::load_offsets(&frx_path, &[0x0000, 0x0008]).expect("Failed to load frx");
    assert_eq!(res.entries.get(&0x0000).unwrap(), &[10, 20, 30, 40]);
    assert_eq!(res.entries.get(&0x0008).unwrap(), &[99, 100]);
}

#[test]
fn test_project_loader_end_to_end() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let base = temp_dir.path();

    let vbp_path = base.join("FullProject.vbp");
    let bas_path = base.join("Module1.bas");
    let cls_path = base.join("Class1.cls");
    let frm_path = base.join("Form1.frm");
    let frx_path = base.join("Form1.frx");

    // Write .bas
    std::fs::write(
        &bas_path,
        "Attribute VB_Name = \"Module1\"\r\nSub Main()\r\n    Form1.Show\r\nEnd Sub\r\n",
    )
    .unwrap();

    // Write .cls
    std::fs::write(
        &cls_path,
        "VERSION 1.0 CLASS\r\nBEGIN\r\nMultiUse = -1\r\nEND\r\nAttribute VB_Name = \"Class1\"\r\nPublic X As Long\r\n",
    )
    .unwrap();

    // Write .frm with .frx reference
    std::fs::write(
        &frm_path,
        r#"VERSION 5.00
Begin VB.Form Form1 
   Caption = "My Form"
   Icon = "Form1.frx":0000
   Begin VB.CommandButton CmdOk 
      Caption = "OK"
   End
End
Attribute VB_Name = "Form1"

Private Sub CmdOk_Click()
    Unload Me
End Sub
"#,
    )
    .unwrap();

    // Write .frx
    let mut frx_bytes = Vec::new();
    frx_bytes.extend_from_slice(&3u32.to_le_bytes());
    frx_bytes.extend_from_slice(&[1, 2, 3]);
    std::fs::write(&frx_path, &frx_bytes).unwrap();

    // Write .vbp
    let vbp_content = r#"Type=Exe
Reference=*\G{00020430-0000-0000-C000-000000000046}#2.0#0#stdole2.tlb#OLE Automation
Module=Module1; Module1.bas
Class=Class1; Class1.cls
Form=Form1.frm
Startup="Sub Main"
Name="FullProject"
ExeName32="FullProject.exe"
MajorVer=1
MinorVer=2
RevisionVer=3
"#;
    std::fs::write(&vbp_path, vbp_content).unwrap();

    // Load full project
    let project = ProjectLoader::load_project(&vbp_path).expect("Failed to load project");

    assert_eq!(project.settings.name, "FullProject");
    assert_eq!(project.settings.startup, "Sub Main");
    assert_eq!(project.modules.len(), 1);
    assert_eq!(project.classes.len(), 1);
    assert_eq!(project.forms.len(), 1);
    assert_eq!(project.references.len(), 1);

    // Verify .frx resource was automatically loaded
    assert_eq!(project.resources.len(), 1);
    let frx_res = project.resources.get(&frx_path).expect("Expected .frx resource");
    assert_eq!(frx_res.entries.get(&0x0000).unwrap(), &[1, 2, 3]);

    // Verify form control tree
    let form = &project.forms[0];
    assert_eq!(form.name, "Form1");
    let root_ctrl = form.root_control.as_ref().unwrap();
    assert_eq!(root_ctrl.control_type, "VB.Form");
    assert_eq!(root_ctrl.children.len(), 1);
    assert_eq!(root_ctrl.children[0].name, "CmdOk");
    assert!(form.code.contains("CmdOk_Click"));
}
