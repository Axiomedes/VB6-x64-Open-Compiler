//! Parser for VB6 Form files (.frm).
//! Separates visual hierarchy, control properties, attributes, and source code.

use crate::models::*;
use std::collections::HashMap;
use std::path::Path;

pub struct FRMParser;

impl FRMParser {
    /// Parse a .frm file content into a FormModule.
    pub fn parse(content: &str, file_path: &Path) -> FormModule {
        let lines: Vec<&str> = content.lines().collect();
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

        // Parse visual control hierarchy (Begin VB.Form ... End)
        let root_control = if idx < lines.len() && lines[idx].trim_start().starts_with("Begin ") {
            Self::parse_control_node(&lines, &mut idx)
        } else {
            None
        };

        // Parse Attribute lines
        let mut attributes = HashMap::new();
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

        // Remaining lines are the VB6 code behind the form
        let code_lines: Vec<&str> = if idx < lines.len() {
            lines[idx..].to_vec()
        } else {
            Vec::new()
        };
        let code = code_lines.join("\r\n");

        // Determine form name from attributes, root control, or file stem
        let form_name = attributes
            .get("VB_Name")
            .cloned()
            .or_else(|| root_control.as_ref().map(|c| c.name.clone()))
            .unwrap_or_else(|| {
                file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Form1")
                    .to_string()
            });

        FormModule {
            name: form_name,
            path: file_path.to_path_buf(),
            guid: None,
            attributes,
            root_control,
            code,
        }
    }

    /// Recursively parse a `Begin <Type> <Name>` ... `End` block.
    pub fn parse_control_node(lines: &[&str], idx: &mut usize) -> Option<ControlNode> {
        if *idx >= lines.len() {
            return None;
        }

        let header_line = lines[*idx].trim();
        if !header_line.starts_with("Begin ") {
            return None;
        }

        // Header format: "Begin VB.CommandButton Command1" or "Begin {GUID} ControlName"
        let parts: Vec<&str> = header_line.split_whitespace().collect();
        let (control_type, name) = if parts.len() >= 3 {
            (parts[1].to_string(), parts[2].to_string())
        } else if parts.len() == 2 {
            (parts[1].to_string(), String::from("Unnamed"))
        } else {
            (String::from("Unknown"), String::from("Unnamed"))
        };

        *idx += 1; // Consume Begin line

        let mut properties = HashMap::new();
        let mut children = Vec::new();
        let mut index_val = None;

        while *idx < lines.len() {
            let line = lines[*idx].trim();

            if line.is_empty() {
                *idx += 1;
                continue;
            }

            if line == "End" {
                *idx += 1; // Consume End line
                break;
            }

            if line.starts_with("Begin ") {
                // Nested control
                if let Some(child) = Self::parse_control_node(lines, idx) {
                    children.push(child);
                }
            } else if let Some((key, val_str)) = line.split_once('=') {
                let key = key.trim().to_string();
                let val_str = val_str.trim();

                // Check for control array Index
                if key.eq_ignore_ascii_case("Index") {
                    index_val = val_str.parse::<i32>().ok();
                }

                // Check for .frx reference: "Form1.frx":0000
                let prop_val = if let Some(frx_ref) = Self::parse_frx_reference(val_str) {
                    frx_ref
                } else {
                    ControlPropertyValue::Literal(val_str.trim_matches('"').to_string())
                };

                properties.insert(key, prop_val);
                *idx += 1;
            } else {
                *idx += 1;
            }
        }

        Some(ControlNode {
            control_type,
            name,
            index: index_val,
            properties,
            children,
        })
    }

    /// Parse property values referencing .frx resource files:
    /// e.g. `"Form1.frx":0000` or `"Form1.frx":031A`
    fn parse_frx_reference(val: &str) -> Option<ControlPropertyValue> {
        if let Some((file_part, offset_part)) = val.split_once(':') {
            let file = file_part.trim().trim_matches('"').to_string();
            if file.to_ascii_lowercase().ends_with(".frx") {
                let offset_str = offset_part.trim();
                let offset = u32::from_str_radix(offset_str, 16).ok()?;
                return Some(ControlPropertyValue::FrxReference {
                    frx_file: file,
                    offset,
                });
            }
        }
        None
    }
}
