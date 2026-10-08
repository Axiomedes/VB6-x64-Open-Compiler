//! Loader for PropertyPage modules (.pag).

use crate::frm_parser::FRMParser;
use crate::models::PropertyPageModule;
use std::path::Path;

pub struct PAGLoader;

impl PAGLoader {
    /// Parse a .pag PropertyPage file into a PropertyPageModule.
    pub fn parse(content: &str, file_path: &Path) -> PropertyPageModule {
        let form_mod = FRMParser::parse(content, file_path);
        PropertyPageModule {
            name: form_mod.name,
            path: form_mod.path,
            attributes: form_mod.attributes,
            root_control: form_mod.root_control,
            code: form_mod.code,
        }
    }
}
