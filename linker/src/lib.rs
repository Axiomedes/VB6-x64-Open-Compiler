//! PE32+ (Windows x64) Linker interface for VB6 Open Compiler.

pub mod pe;
pub use pe::{PeError, PeInfo};

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;

static LINKER_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Error)]
pub enum LinkerError {
    #[error("Linker tool not found. Checked default locations and PATH.")]
    LinkerNotFound,
    #[error("Linker execution failed: {0}")]
    Execution(String),
    #[error("PE validation failed: {0}")]
    PeValidation(#[from] PeError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct Linker {
    clang_path: PathBuf,
}

impl Linker {
    pub fn new() -> Result<Self, LinkerError> {
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

        Err(LinkerError::LinkerNotFound)
    }

    /// Link one or more object files into a native Windows x64 PE32+ executable.
    pub fn link_executable(
        &self,
        obj_files: &[&Path],
        output_exe: &Path,
    ) -> Result<PeInfo, LinkerError> {
        // Compile the embedded standalone C runtime
        let count = LINKER_COUNTER.fetch_add(1, Ordering::Relaxed);
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let temp_dir = std::env::temp_dir();
        let runtime_c = temp_dir.join(format!("vb6_rt_{}_{}_{}.c", std::process::id(), time, count));
        let runtime_obj = temp_dir.join(format!("vb6_rt_{}_{}_{}.obj", std::process::id(), time, count));

        std::fs::write(&runtime_c, vb6_runtime::get_runtime_c_source())?;

        let compile_rt = Command::new(&self.clang_path)
            .arg("-target")
            .arg("x86_64-pc-windows-msvc")
            .arg("-Wno-override-module")
            .arg("-c")
            .arg(&runtime_c)
            .arg("-o")
            .arg(&runtime_obj)
            .output()?;

        let _ = std::fs::remove_file(&runtime_c);

        if !compile_rt.status.success() {
            let stderr = String::from_utf8_lossy(&compile_rt.stderr);
            return Err(LinkerError::Execution(format!(
                "Failed to compile runtime support library: {}",
                stderr
            )));
        }

        // Link with Clang (driver mode invokes lld-link with MSVC libraries)
        let mut cmd = Command::new(&self.clang_path);
        cmd.arg("-target").arg("x86_64-pc-windows-msvc");
        cmd.arg("-Wno-override-module");

        for obj in obj_files {
            cmd.arg(obj);
        }
        cmd.arg(&runtime_obj);

        cmd.arg("-o").arg(output_exe);

        // Standard Windows system libraries
        cmd.arg("-luser32");
        cmd.arg("-lole32");
        cmd.arg("-loleaut32");
        cmd.arg("-lkernel32");

        let link_output = cmd.output()?;
        let _ = std::fs::remove_file(&runtime_obj);

        if !link_output.status.success() {
            let stderr = String::from_utf8_lossy(&link_output.stderr);
            return Err(LinkerError::Execution(format!(
                "Linker failed: {}",
                stderr
            )));
        }

        if !output_exe.exists() {
            return Err(LinkerError::Execution(
                "Executable file was not created by linker".to_string(),
            ));
        }

        // Inspect and validate the generated PE32+ file
        let pe_info = PeInfo::inspect(output_exe)?;
        Ok(pe_info)
    }
}
