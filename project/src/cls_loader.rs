//! Loader for class modules (.cls).
//! Extracts class metadata, instancing behavior, attributes, and class methods.

use crate::models::ClassModule;
use std::collections::HashMap;
use std::path::Path;

pub struct CLSLoader;

impl CLSLoader {
    /// Parse a .cls class module into a ClassModule struct.
    pub fn parse(content: &str, file_path: &Path, default_name: Option<String>) -> ClassModule {
        let lines: Vec<&str> = content.lines().collect();
        let mut attributes = HashMap::new();
        let mut instancing = None;
        let mut idx = 0;

        // Skip leading blank lines or VERSION lines
        while idx < lines.len() {
            let line = lines[idx].trim();
            if line.is_empty() || line.starts_with("VERSION ") {
                idx += 1;
            } else {
                break;
            }
        }

        // Parse optional BEGIN ... END block (class persistence/instancing properties)
        if idx < lines.len() && lines[idx].trim().eq_ignore_ascii_case("BEGIN") {
            idx += 1;
            while idx < lines.len() {
                let line = lines[idx].trim();
                if line.eq_ignore_ascii_case("END") {
                    idx += 1;
                    break;
                }
                if let Some((k, v)) = line.split_once('=') {
                    let key = k.trim().to_string();
                    let val = v.trim().split('\'').next().unwrap_or("").trim().to_string();
                    if key.eq_ignore_ascii_case("MultiUse") {
                        instancing = Some(val.clone());
                    }
                    attributes.insert(key, val);
                }
                idx += 1;
            }
        }

        // Parse Attribute lines
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

        // Remaining lines are the class methods and body
        let code_lines: Vec<&str> = if idx < lines.len() {
            lines[idx..].to_vec()
        } else {
            Vec::new()
        };
        let code = code_lines.join("\r\n");

        let name = attributes
            .get("VB_Name")
            .cloned()
            .or(default_name)
            .unwrap_or_else(|| {
                file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Class1")
                    .to_string()
            });

        ClassModule {
            name,
            path: file_path.to_path_buf(),
            attributes,
            instancing,
            code,
        }
    }
}
