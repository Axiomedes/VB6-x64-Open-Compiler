//! Discovery and invocation of the LLVM compiler toolchain (clang / llc).

use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolchainError {
    #[error("LLVM / Clang compiler not found. Checked default locations and PATH.")]
    CompilerNotFound,
    #[error("Compilation command failed: {0}")]
    ExecutionFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct Toolchain {
    clang_path: PathBuf,
}

use std::sync::atomic::{AtomicU64, Ordering};
static TOOLCHAIN_COUNTER: AtomicU64 = AtomicU64::new(0);

impl Toolchain {
    pub fn discover() -> Result<Self, ToolchainError> {
        let candidates = [
            PathBuf::from(r"C:\Program Files\LLVM\bin\clang.exe"),
            PathBuf::from(r"C:\Program Files (x86)\LLVM\bin\clang.exe"),
        ];

        for path in &candidates {
            if path.exists() {
                return Ok(Self {
                    clang_path: path.clone(),
                });
            }
        }

        // Try discovering in PATH
        if let Ok(output) = Command::new("where.exe").arg("clang.exe").output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(first_line) = stdout.lines().next() {
                    let path = PathBuf::from(first_line.trim());
                    if path.exists() {
                        return Ok(Self { clang_path: path });
                    }
                }
            }
        }

        Err(ToolchainError::CompilerNotFound)
    }

    pub fn clang_path(&self) -> &Path {
        &self.clang_path
    }

    /// Compile textual LLVM IR to a native Windows x64 COFF object file (.obj).
    pub fn compile_llvm_ir_to_obj(
        &self,
        llvm_ir: &str,
        output_obj: &Path,
    ) -> Result<(), ToolchainError> {
        let count = TOOLCHAIN_COUNTER.fetch_add(1, Ordering::Relaxed);
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let temp_dir = std::env::temp_dir();
        let temp_ll = temp_dir.join(format!("vb6_tmp_{}_{}_{}.ll", std::process::id(), time, count));
        std::fs::write(&temp_ll, llvm_ir)?;

        let output = Command::new(&self.clang_path)
            .arg("-target")
            .arg("x86_64-pc-windows-msvc")
            .arg("-Wno-override-module")
            .arg("-c")
            .arg(&temp_ll)
            .arg("-o")
            .arg(output_obj)
            .output()?;

        let _ = std::fs::remove_file(&temp_ll);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ToolchainError::ExecutionFailed(format!(
                "clang error: {}",
                stderr
            )));
        }

        if !output_obj.exists() {
            return Err(ToolchainError::ExecutionFailed(
                "Output object file was not generated".to_string(),
            ));
        }

        Ok(())
    }
}
