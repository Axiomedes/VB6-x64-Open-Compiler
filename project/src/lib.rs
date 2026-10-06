//! Project loader and manager for VB6 project files (.vbp, .frm, .bas, .cls, .frx).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("Failed to read project file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid project file format: {0}")]
    InvalidFormat(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VbProject {
    pub name: String,
    pub project_type: String,
    pub startup: String,
    pub exe_name32: String,
    pub modules: Vec<PathBuf>,
    pub classes: Vec<PathBuf>,
    pub forms: Vec<PathBuf>,
    pub references: Vec<String>,
    pub major_ver: u32,
    pub minor_ver: u32,
    pub revision_ver: u32,
    pub properties: HashMap<String, String>,
}

impl VbProject {
    pub fn parse_vbp_str(content: &str, base_dir: &Path) -> Result<Self, ProjectError> {
        let mut name = String::from("Project1");
        let mut project_type = String::from("Exe");
        let mut startup = String::from("Sub Main");
        let mut exe_name32 = String::from("Project1.exe");
        let mut modules = Vec::new();
        let mut classes = Vec::new();
        let mut forms = Vec::new();
        let mut references = Vec::new();
        let mut major_ver = 1;
        let mut minor_ver = 0;
        let mut revision_ver = 0;
        let mut properties = HashMap::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('\'') || line.starts_with(';') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"');

                match key.to_ascii_uppercase().as_str() {
                    "NAME" => name = value.to_string(),
                    "TYPE" => project_type = value.to_string(),
                    "STARTUP" => startup = value.to_string(),
                    "EXENAME32" => exe_name32 = value.to_string(),
                    "MAJORVER" => major_ver = value.parse().unwrap_or(1),
                    "MINORVER" => minor_ver = value.parse().unwrap_or(0),
                    "REVISIONVER" => revision_ver = value.parse().unwrap_or(0),
                    "REFERENCE" => references.push(value.to_string()),
                    "MODULE" => {
                        // Format: Module=ModuleName; Path.bas
                        let path_str = if let Some((_, p)) = value.split_once(';') {
                            p.trim()
                        } else {
                            value
                        };
                        modules.push(base_dir.join(path_str));
                    }
                    "CLASS" => {
                        // Format: Class=ClassName; Path.cls
                        let path_str = if let Some((_, p)) = value.split_once(';') {
                            p.trim()
                        } else {
                            value
                        };
                        classes.push(base_dir.join(path_str));
                    }
                    "FORM" => {
                        forms.push(base_dir.join(value));
                    }
                    _ => {
                        properties.insert(key.to_string(), value.to_string());
                    }
                }
            }
        }

        Ok(Self {
            name,
            project_type,
            startup,
            exe_name32,
            modules,
            classes,
            forms,
            references,
            major_ver,
            minor_ver,
            revision_ver,
            properties,
        })
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ProjectError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;
        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
        Self::parse_vbp_str(&content, base_dir)
    }
}
