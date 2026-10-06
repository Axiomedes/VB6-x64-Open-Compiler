//! Target code generation from VB6 IR.

use crate::ir::IrModule;

pub struct CodeGenerator {}

impl CodeGenerator {
    pub fn new() -> Self {
        Self {}
    }

    pub fn generate(&self, module: &IrModule) -> Result<String, String> {
        let _ = module;
        Ok(String::new())
    }
}
