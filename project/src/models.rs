//! Strongly-typed domain model for Microsoft Visual Basic 6.0 projects.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Complete internal representation of a VB6 Project (.vbp and referenced files).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VB6Project {
    pub settings: ProjectSettings,
    pub references: Vec<ProjectReference>,
    pub components: Vec<ProjectComponent>,
    pub forms: Vec<FormModule>,
    pub modules: Vec<CodeModule>,
    pub classes: Vec<ClassModule>,
    pub user_controls: Vec<UserControlModule>,
    pub property_pages: Vec<PropertyPageModule>,
    pub resources: HashMap<PathBuf, FrxResourceFile>,
}

/// Project-wide compilation, version, and runtime settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub name: String,
    pub project_type: String, // "Exe", "OleDll", "Control", etc.
    pub startup: String,      // "Sub Main" or "Form1"
    pub exe_name32: String,
    pub title: String,
    pub description: String,
    pub help_file: String,
    pub help_context_id: u32,
    pub compatible_mode: String,
    pub major_ver: u32,
    pub minor_ver: u32,
    pub revision_ver: u32,
    pub auto_increment_ver: bool,
    pub server_support_files: bool,
    pub version_company_name: String,
    pub compilation_type: i32,
    pub optimization_type: i32,
    pub favor_pentium_pro: bool,
    pub codeview_debug_info: bool,
    pub no_aliasing: bool,
    pub bounds_check: bool,
    pub overflow_check: bool,
    pub flpoint_check: bool,
    pub fdiv_check: bool,
    pub unrounded_fp: bool,
    pub start_mode: i32,
    pub unattended: bool,
    pub retained: bool,
    pub thread_per_object: bool,
    pub max_number_of_threads: u32,
    pub extra_properties: HashMap<String, String>,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            name: String::from("Project1"),
            project_type: String::from("Exe"),
            startup: String::from("Sub Main"),
            exe_name32: String::from("Project1.exe"),
            title: String::from("Project1"),
            description: String::new(),
            help_file: String::new(),
            help_context_id: 0,
            compatible_mode: String::from("0"),
            major_ver: 1,
            minor_ver: 0,
            revision_ver: 0,
            auto_increment_ver: false,
            server_support_files: false,
            version_company_name: String::new(),
            compilation_type: 0,
            optimization_type: 0,
            favor_pentium_pro: false,
            codeview_debug_info: false,
            no_aliasing: false,
            bounds_check: false,
            overflow_check: false,
            flpoint_check: false,
            fdiv_check: false,
            unrounded_fp: false,
            start_mode: 0,
            unattended: false,
            retained: false,
            thread_per_object: false,
            max_number_of_threads: 1,
            extra_properties: HashMap::new(),
        }
    }
}

/// A COM Type Library Reference defined in a .vbp.
/// Example: Reference=*\G{00020430-0000-0000-C000-000000000046}#2.0#0#C:\Windows\System32\stdole2.tlb#OLE Automation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectReference {
    pub raw: String,
    pub guid: String,
    pub major_ver: u32,
    pub minor_ver: u32,
    pub lcid: u32,
    pub path: String,
    pub name: String,
}

/// An ActiveX / OCX Component defined in a .vbp.
/// Example: Object={831FDD16-0C5C-11D2-A9FC-0000F8754DA1}#2.0#0; MSCOMCTL.OCX
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectComponent {
    pub raw: String,
    pub guid: String,
    pub major_ver: u32,
    pub minor_ver: u32,
    pub lcid: u32,
    pub file: String,
}

/// Form module (.frm).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormModule {
    pub name: String,
    pub path: PathBuf,
    pub guid: Option<String>,
    pub attributes: HashMap<String, String>,
    pub root_control: Option<ControlNode>,
    pub code: String,
}

/// Standard code module (.bas).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeModule {
    pub name: String,
    pub path: PathBuf,
    pub attributes: HashMap<String, String>,
    pub code: String,
}

/// Class module (.cls).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassModule {
    pub name: String,
    pub path: PathBuf,
    pub attributes: HashMap<String, String>,
    pub instancing: Option<String>,
    pub code: String,
}

/// UserControl module (.ctl).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserControlModule {
    pub name: String,
    pub path: PathBuf,
    pub attributes: HashMap<String, String>,
    pub root_control: Option<ControlNode>,
    pub code: String,
}

/// PropertyPage module (.pag).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyPageModule {
    pub name: String,
    pub path: PathBuf,
    pub attributes: HashMap<String, String>,
    pub root_control: Option<ControlNode>,
    pub code: String,
}

/// Visual control node within a .frm or .ctl visual tree.
/// Supports arbitrary nesting (e.g. Forms containing Frames containing Buttons/TextBoxes).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlNode {
    pub control_type: String, // e.g., "VB.Form", "VB.CommandButton", "VB.TextBox", "VB.Frame"
    pub name: String,         // e.g., "Command1"
    pub index: Option<i32>,   // For control arrays
    pub properties: HashMap<String, ControlPropertyValue>,
    pub children: Vec<ControlNode>,
}

/// Value of a property defined on a visual control.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ControlPropertyValue {
    /// Literal string / numeric value: Caption = "Click Me"
    Literal(String),
    /// Binary resource reference in .frx: Icon = "Form1.frx":0000
    FrxReference { frx_file: String, offset: u32 },
}

/// Binary resource file (.frx).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrxResourceFile {
    pub path: PathBuf,
    pub entries: HashMap<u32, Vec<u8>>,
}
