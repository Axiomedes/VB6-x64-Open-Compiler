//! PE32+ (Windows x64) Linker interface.

use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LinkerError {
    #[error("Linker execution failed: {0}")]
    Execution(String),
}

pub struct Linker {
    output_path: std::path::PathBuf,
}

impl Linker {
    pub fn new<P: AsRef<Path>>(output_path: P) -> Self {
        Self {
            output_path: output_path.as_ref().to_path_buf(),
        }
    }

    pub fn output_path(&self) -> &Path {
        &self.output_path
    }
}
