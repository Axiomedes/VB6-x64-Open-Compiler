//! IR Optimization passes.

use crate::ir::IrModule;

pub struct Optimizer {}

impl Optimizer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn optimize(&self, module: &mut IrModule) {
        // Optimization passes to be implemented in Milestone 2/3
        let _ = module;
    }
}
