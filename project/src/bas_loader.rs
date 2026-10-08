//! Loader for standard code modules (.bas).

use crate::models::CodeModule;
use std::collections::HashMap;
use std::path::Path;

pub struct BASLoader;

impl BASLoader {
    /// Parse a .bas module string into a CodeModule struct.
    pub fn parse(content: &str, file_path: &Path, default_name: Option<String>) -> CodeModule {
        let lines: Vec<&str> = content.lines().collect();
        let mut attributes = HashMap::new();
        let mut idx = 0;

        // Skip leading blank lines
        while idx < lines.len() && lines[idx].trim().is_empty() {
            idx += 1;
        }

        // Parse attributes (e.g. Attribute VB_Name = "Module1")
        while idx < lines.len() {
            let line = lines[idx].trim();
            if line.starts_with("Attribute ") {
                let attr_content = &line["Attribute ".len()..];
                if let Some((k, v)) = attr_content.split_once('=') {
                    let key = k.trim().to_string();
                    let val = v.trim().trim_matches('"').to_string();
                    attributes.insert(key, val);
                }
                idx += 1;
            } else if line.is_empty() {
                idx += 1;
            } else {
                break;
            }
        }

        // Remaining lines are the VB6 module code
        let code_lines: Vec<&str> = if idx < lines.len() {
            lines[idx..].to_vec()
        } else {
            Vec::new()
        };
        let code = code_lines.join("\r\n");

        // Determine name
        let name = attributes
            .get("VB_Name")
            .cloned()
            .or(default_name)
            .unwrap_or_else(|| {
                file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Module1")
                    .to_string()
            });

        CodeModule {
            name,
            path: file_path.to_path_buf(),
            attributes,
            code,
        }
    }
}
