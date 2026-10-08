//! Project loader and manager for VB6 project files (.vbp, .frm, .bas, .cls, .ctl, .pag, .frx).

pub mod bas_loader;
pub mod cls_loader;
pub mod ctl_loader;
pub mod encoding;
pub mod frm_parser;
pub mod frx_loader;
pub mod models;
pub mod pag_loader;
pub mod project_loader;
pub mod vbp_parser;

pub use bas_loader::BASLoader;
pub use cls_loader::CLSLoader;
pub use ctl_loader::CTLLoader;
pub use encoding::read_source_file;
pub use frm_parser::FRMParser;
pub use frx_loader::FRXLoader;
pub use models::*;
pub use pag_loader::PAGLoader;
pub use project_loader::{ProjectLoader, ProjectLoaderError};
pub use vbp_parser::VBPParser;

// Alias for convenience / backwards compatibility
pub type VbProject = VB6Project;

impl VB6Project {
    pub fn load_from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, ProjectLoaderError> {
        ProjectLoader::load_project(path)
    }
}
