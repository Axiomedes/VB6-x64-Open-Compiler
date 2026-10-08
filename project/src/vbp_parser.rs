//! Parser for Visual Basic 6.0 Project files (.vbp).

use crate::models::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct RawVbpData {
    pub settings: ProjectSettings,
    pub references: Vec<ProjectReference>,
    pub components: Vec<ProjectComponent>,
    pub module_files: Vec<(Option<String>, PathBuf)>,
    pub class_files: Vec<(Option<String>, PathBuf)>,
    pub form_files: Vec<PathBuf>,
    pub user_control_files: Vec<PathBuf>,
    pub property_page_files: Vec<PathBuf>,
}

pub struct VBPParser;

impl VBPParser {
    /// Parse a .vbp content string into structured metadata and file paths.
    pub fn parse(content: &str, base_dir: &Path) -> RawVbpData {
        let mut data = RawVbpData::default();
        let mut extra = HashMap::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('\'') || line.starts_with(';') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"');

                match key.to_ascii_uppercase().as_str() {
                    "NAME" => data.settings.name = value.to_string(),
                    "TYPE" => data.settings.project_type = value.to_string(),
                    "STARTUP" => data.settings.startup = value.to_string(),
                    "EXENAME32" => data.settings.exe_name32 = value.to_string(),
                    "TITLE" => data.settings.title = value.to_string(),
                    "DESCRIPTION" => data.settings.description = value.to_string(),
                    "HELPFILE" => data.settings.help_file = value.to_string(),
                    "HELPCONTEXTID" => {
                        data.settings.help_context_id = value.parse().unwrap_or(0);
                    }
                    "COMPATIBLEMODE" => data.settings.compatible_mode = value.to_string(),
                    "MAJORVER" => data.settings.major_ver = value.parse().unwrap_or(1),
                    "MINORVER" => data.settings.minor_ver = value.parse().unwrap_or(0),
                    "REVISIONVER" => data.settings.revision_ver = value.parse().unwrap_or(0),
                    "AUTOINCREMENTVER" => {
                        data.settings.auto_increment_ver = value == "1" || value.eq_ignore_ascii_case("true");
                    }
                    "SERVERSUPPORTFILES" => {
                        data.settings.server_support_files = value == "1" || value.eq_ignore_ascii_case("true");
                    }
                    "VERSIONCOMPANYNAME" => data.settings.version_company_name = value.to_string(),
                    "COMPILATIONTYPE" => {
                        data.settings.compilation_type = value.parse().unwrap_or(0);
                    }
                    "OPTIMIZATIONTYPE" => {
                        data.settings.optimization_type = value.parse().unwrap_or(0);
                    }
                    "FAVORPENTIUMPRO(TM)" => {
                        data.settings.favor_pentium_pro = value == "1" || value.eq_ignore_ascii_case("true");
                    }
                    "CODEVIEWDEBUGINFO" => {
                        data.settings.codeview_debug_info = value == "1" || value.eq_ignore_ascii_case("true");
                    }
                    "NOALIASING" => {
                        data.settings.no_aliasing = value == "1" || value.eq_ignore_ascii_case("true");
                    }
                    "BOUNDSCHECK" => {
                        data.settings.bounds_check = value == "1" || value.eq_ignore_ascii_case("true");
                    }
                    "OVERFLOWCHECK" => {
                        data.settings.overflow_check = value == "1" || value.eq_ignore_ascii_case("true");
                    }
                    "FLPOINTCHECK" => {
                        data.settings.flpoint_check = value == "1" || value.eq_ignore_ascii_case("true");
                    }
                    "FDIVCHECK" => {
                        data.settings.fdiv_check = value == "1" || value.eq_ignore_ascii_case("true");
                    }
                    "UNROUNDEDFP" => {
                        data.settings.unrounded_fp = value == "1" || value.eq_ignore_ascii_case("true");
                    }
                    "STARTMODE" => {
                        data.settings.start_mode = value.parse().unwrap_or(0);
                    }
                    "UNATTENDED" => {
                        data.settings.unattended = value == "1" || value.eq_ignore_ascii_case("true");
                    }
                    "RETAINED" => {
                        data.settings.retained = value == "1" || value.eq_ignore_ascii_case("true");
                    }
                    "THREADPEROBJECT" => {
                        data.settings.thread_per_object = value == "1" || value.eq_ignore_ascii_case("true");
                    }
                    "MAXNUMBEROFTHREADS" => {
                        data.settings.max_number_of_threads = value.parse().unwrap_or(1);
                    }
                    "REFERENCE" => {
                        if let Some(reference) = Self::parse_reference(value) {
                            data.references.push(reference);
                        }
                    }
                    "OBJECT" => {
                        if let Some(comp) = Self::parse_component(value) {
                            data.components.push(comp);
                        }
                    }
                    "MODULE" => {
                        // Format: Module=ModuleName; RelativePath.bas
                        let (mod_name, path) = if let Some((n, p)) = value.split_once(';') {
                            (Some(n.trim().to_string()), p.trim())
                        } else {
                            (None, value)
                        };
                        data.module_files.push((mod_name, base_dir.join(path)));
                    }
                    "CLASS" => {
                        // Format: Class=ClassName; RelativePath.cls
                        let (cls_name, path) = if let Some((n, p)) = value.split_once(';') {
                            (Some(n.trim().to_string()), p.trim())
                        } else {
                            (None, value)
                        };
                        data.class_files.push((cls_name, base_dir.join(path)));
                    }
                    "FORM" => {
                        data.form_files.push(base_dir.join(value));
                    }
                    "USERCONTROL" => {
                        data.user_control_files.push(base_dir.join(value));
                    }
                    "PROPERTYPAGE" => {
                        data.property_page_files.push(base_dir.join(value));
                    }
                    _ => {
                        extra.insert(key.to_string(), value.to_string());
                    }
                }
            }
        }

        data.settings.extra_properties = extra;
        data
    }

    /// Parse Reference entry:
    /// e.g. `*\G{00020430-0000-0000-C000-000000000046}#2.0#0#C:\Windows\System32\stdole2.tlb#OLE Automation`
    fn parse_reference(raw: &str) -> Option<ProjectReference> {
        let parts: Vec<&str> = raw.split('#').collect();
        if parts.len() >= 5 {
            let guid_part = parts[0];
            let guid = if let Some(idx) = guid_part.find('{') {
                if let Some(end_idx) = guid_part.find('}') {
                    guid_part[idx..=end_idx].to_string()
                } else {
                    guid_part.to_string()
                }
            } else {
                guid_part.to_string()
            };

            let ver_parts: Vec<&str> = parts[1].split('.').collect();
            let major_ver = ver_parts.first().and_then(|v| v.parse().ok()).unwrap_or(1);
            let minor_ver = ver_parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(0);
            let lcid = parts[2].parse().unwrap_or(0);
            let path = parts[3].to_string();
            let name = parts[4].to_string();

            Some(ProjectReference {
                raw: raw.to_string(),
                guid,
                major_ver,
                minor_ver,
                lcid,
                path,
                name,
            })
        } else {
            // Fallback for non-standard references
            Some(ProjectReference {
                raw: raw.to_string(),
                guid: String::new(),
                major_ver: 0,
                minor_ver: 0,
                lcid: 0,
                path: raw.to_string(),
                name: String::new(),
            })
        }
    }

    /// Parse Object component entry:
    /// e.g. `{831FDD16-0C5C-11D2-A9FC-0000F8754DA1}#2.0#0; MSCOMCTL.OCX`
    fn parse_component(raw: &str) -> Option<ProjectComponent> {
        let (left, right) = raw.split_once(';')?;
        let file = right.trim().to_string();
        let parts: Vec<&str> = left.trim().split('#').collect();

        let guid = parts.first().unwrap_or(&"").trim().to_string();
        let ver_str = parts.get(1).unwrap_or(&"1.0");
        let ver_parts: Vec<&str> = ver_str.split('.').collect();
        let major_ver = ver_parts.first().and_then(|v| v.parse().ok()).unwrap_or(1);
        let minor_ver = ver_parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(0);
        let lcid = parts.get(2).and_then(|l| l.parse().ok()).unwrap_or(0);

        Some(ProjectComponent {
            raw: raw.to_string(),
            guid,
            major_ver,
            minor_ver,
            lcid,
            file,
        })
    }
}
