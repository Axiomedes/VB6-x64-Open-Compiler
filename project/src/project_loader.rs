//! Master ProjectLoader orchestrator.
//! Loads full VB6 projects and all associated source and binary resource files.

use crate::bas_loader::BASLoader;
use crate::cls_loader::CLSLoader;
use crate::ctl_loader::CTLLoader;
use crate::encoding::read_source_file;
use crate::frm_parser::FRMParser;
use crate::frx_loader::FRXLoader;
use crate::models::*;
use crate::pag_loader::PAGLoader;
use crate::vbp_parser::VBPParser;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectLoaderError {
    #[error("I/O error loading project file '{0}': {1}")]
    Io(PathBuf, #[source] std::io::Error),
    #[error("Failed to parse project file '{0}': {1}")]
    ParseError(PathBuf, String),
    #[error("Referenced file does not exist: '{0}'")]
    FileNotFound(PathBuf),
}

pub struct ProjectLoader;

impl ProjectLoader {
    /// Load an entire VB6 project from a .vbp file on disk.
    pub fn load_project<P: AsRef<Path>>(vbp_path: P) -> Result<VB6Project, ProjectLoaderError> {
        let vbp_path_ref = vbp_path.as_ref();
        let base_dir = vbp_path_ref.parent().unwrap_or_else(|| Path::new("."));

        let content = read_source_file(vbp_path_ref)
            .map_err(|e| ProjectLoaderError::Io(vbp_path_ref.to_path_buf(), e))?;

        let raw_vbp = VBPParser::parse(&content, base_dir);

        let mut modules = Vec::new();
        for (default_name, mod_path) in raw_vbp.module_files {
            if mod_path.exists() {
                let mod_content = read_source_file(&mod_path)
                    .map_err(|e| ProjectLoaderError::Io(mod_path.clone(), e))?;
                let code_mod = BASLoader::parse(&mod_content, &mod_path, default_name);
                modules.push(code_mod);
            } else {
                return Err(ProjectLoaderError::FileNotFound(mod_path));
            }
        }

        let mut classes = Vec::new();
        for (default_name, cls_path) in raw_vbp.class_files {
            if cls_path.exists() {
                let cls_content = read_source_file(&cls_path)
                    .map_err(|e| ProjectLoaderError::Io(cls_path.clone(), e))?;
                let cls_mod = CLSLoader::parse(&cls_content, &cls_path, default_name);
                classes.push(cls_mod);
            } else {
                return Err(ProjectLoaderError::FileNotFound(cls_path));
            }
        }

        let mut forms = Vec::new();
        let mut resources = HashMap::new();

        for form_path in raw_vbp.form_files {
            if form_path.exists() {
                let form_content = read_source_file(&form_path)
                    .map_err(|e| ProjectLoaderError::Io(form_path.clone(), e))?;
                let form_mod = FRMParser::parse(&form_content, &form_path);

                // Collect .frx references from controls
                if let Some(ref root) = form_mod.root_control {
                    let mut frx_map: HashMap<String, Vec<u32>> = HashMap::new();
                    Self::collect_frx_offsets(root, &mut frx_map);

                    let form_dir = form_path.parent().unwrap_or_else(|| Path::new("."));
                    for (frx_filename, offsets) in frx_map {
                        let frx_path = form_dir.join(&frx_filename);
                        if frx_path.exists() {
                            if let Ok(res_file) = FRXLoader::load_offsets(&frx_path, &offsets) {
                                resources.insert(frx_path, res_file);
                            }
                        }
                    }
                }

                forms.push(form_mod);
            } else {
                return Err(ProjectLoaderError::FileNotFound(form_path));
            }
        }

        let mut user_controls = Vec::new();
        for ctl_path in raw_vbp.user_control_files {
            if ctl_path.exists() {
                let ctl_content = read_source_file(&ctl_path)
                    .map_err(|e| ProjectLoaderError::Io(ctl_path.clone(), e))?;
                let ctl_mod = CTLLoader::parse(&ctl_content, &ctl_path);
                user_controls.push(ctl_mod);
            } else {
                return Err(ProjectLoaderError::FileNotFound(ctl_path));
            }
        }

        let mut property_pages = Vec::new();
        for pag_path in raw_vbp.property_page_files {
            if pag_path.exists() {
                let pag_content = read_source_file(&pag_path)
                    .map_err(|e| ProjectLoaderError::Io(pag_path.clone(), e))?;
                let pag_mod = PAGLoader::parse(&pag_content, &pag_path);
                property_pages.push(pag_mod);
            } else {
                return Err(ProjectLoaderError::FileNotFound(pag_path));
            }
        }

        Ok(VB6Project {
            settings: raw_vbp.settings,
            references: raw_vbp.references,
            components: raw_vbp.components,
            forms,
            modules,
            classes,
            user_controls,
            property_pages,
            resources,
        })
    }

    /// Recursively collect all .frx resource references from a control hierarchy.
    fn collect_frx_offsets(node: &ControlNode, out_map: &mut HashMap<String, Vec<u32>>) {
        for prop_val in node.properties.values() {
            if let ControlPropertyValue::FrxReference { frx_file, offset } = prop_val {
                out_map
                    .entry(frx_file.clone())
                    .or_default()
                    .push(*offset);
            }
        }
        for child in &node.children {
            Self::collect_frx_offsets(child, out_map);
        }
    }
}
