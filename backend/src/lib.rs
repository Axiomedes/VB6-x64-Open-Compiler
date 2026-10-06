//! Backend code generator for VB6 Open Compiler (x64 / PE32+).

use vb6_compiler::ir::IrModule;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BackendError {
    #[error("Code generation failed: {0}")]
    CodeGen(String),
}

pub struct Backend {
    target: String,
}

impl Backend {
    pub fn new(target: &str) -> Self {
        Self {
            target: target.to_string(),
        }
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn compile_module(&self, module: &IrModule) -> Result<Vec<u8>, BackendError> {
        let _ = module;
        Ok(Vec::new())
    }
}
