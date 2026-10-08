//! Loader for UserControl modules (.ctl).

use crate::frm_parser::FRMParser;
use crate::models::UserControlModule;
use std::path::Path;

pub struct CTLLoader;

impl CTLLoader {
    /// Parse a .ctl UserControl file into a UserControlModule.
    pub fn parse(content: &str, file_path: &Path) -> UserControlModule {
        let form_mod = FRMParser::parse(content, file_path);
        UserControlModule {
            name: form_mod.name,
            path: form_mod.path,
            attributes: form_mod.attributes,
            root_control: form_mod.root_control,
            code: form_mod.code,
        }
    }
}
