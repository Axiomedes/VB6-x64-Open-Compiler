//! Backend code generator for VB6 Open Compiler (x64 / PE32+).

pub mod emitter;
pub mod toolchain;

pub use emitter::LlvmEmitter;
pub use toolchain::{Toolchain, ToolchainError};

use std::path::Path;
use thiserror::Error;
use vb6_compiler::ir::IrModule;

#[derive(Debug, Error)]
pub enum BackendError {
    #[error("Toolchain error: {0}")]
    Toolchain(#[from] ToolchainError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Code generation error: {0}")]
    CodeGen(String),
}

pub struct Backend {
    target: String,
    emitter: LlvmEmitter,
    toolchain: Option<Toolchain>,
}

impl Backend {
    pub fn new(target: &str) -> Self {
        let emitter = LlvmEmitter::new("x86_64-pc-windows-msvc");
        let toolchain = Toolchain::discover().ok();
        Self {
            target: target.to_string(),
            emitter,
            toolchain,
        }
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    /// Emit textual LLVM IR for an IrModule.
    pub fn emit_llvm_ir(&self, module: &IrModule, emit_main_wrapper: bool) -> String {
        self.emitter.emit_module(module, emit_main_wrapper)
    }

    /// Compile IrModule directly to a native Windows x64 object file (.obj).
    pub fn compile_to_object(
        &self,
        module: &IrModule,
        output_obj: &Path,
        emit_main_wrapper: bool,
    ) -> Result<(), BackendError> {
        let toolchain = self
            .toolchain
            .as_ref()
            .ok_or(ToolchainError::CompilerNotFound)?;

        let llvm_ir = self.emit_llvm_ir(module, emit_main_wrapper);
        toolchain.compile_llvm_ir_to_obj(&llvm_ir, output_obj)?;
        Ok(())
    }

    /// Compile IrModule to object bytes in memory.
    pub fn compile_module(&self, module: &IrModule) -> Result<Vec<u8>, BackendError> {
        let temp_dir = std::env::temp_dir();
        let temp_obj = temp_dir.join(format!("vb6_tmp_{}.obj", std::process::id()));
        self.compile_to_object(module, &temp_obj, false)?;
        let bytes = std::fs::read(&temp_obj)?;
        let _ = std::fs::remove_file(&temp_obj);
        Ok(bytes)
    }
}
