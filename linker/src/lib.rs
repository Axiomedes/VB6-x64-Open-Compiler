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
    include_paths: Vec<PathBuf>,
    lib_paths: Vec<PathBuf>,
}

fn discover_windows_toolchain() -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut includes = Vec::new();
    let mut libs = Vec::new();

    // 1. MSVC Tools
    let vs_roots = [
        r"C:\Program Files (x86)\Microsoft Visual Studio",
        r"C:\Program Files\Microsoft Visual Studio",
    ];
    for root in &vs_roots {
        let p = Path::new(root);
        if let Ok(years) = std::fs::read_dir(p) {
            for year in years.flatten() {
                if let Ok(editions) = std::fs::read_dir(year.path()) {
                    for edition in editions.flatten() {
                        let msvc_dir = edition.path().join(r"VC\Tools\MSVC");
                        if let Ok(versions) = std::fs::read_dir(&msvc_dir) {
                            let mut ver_paths: Vec<_> = versions.flatten().map(|v| v.path()).collect();
                            ver_paths.sort();
                            if let Some(newest) = ver_paths.last() {
                                let inc = newest.join("include");
                                let lib = newest.join(r"lib\x64");
                                if inc.exists() {
                                    includes.push(inc);
                                }
                                if lib.exists() {
                                    libs.push(lib);
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Windows Kits (SDK)
    let sdk_roots = [
        r"C:\Program Files (x86)\Windows Kits\10",
        r"C:\Program Files\Windows Kits\10",
    ];
    for root in &sdk_roots {
        let inc_base = Path::new(root).join("Include");
        if let Ok(versions) = std::fs::read_dir(&inc_base) {
            let mut ver_dirs: Vec<_> = versions.flatten().map(|v| v.path()).collect();
            ver_dirs.sort();
            if let Some(newest) = ver_dirs.last() {
                for sub in &["ucrt", "shared", "um"] {
                    let p = newest.join(sub);
                    if p.exists() {
                        includes.push(p);
                    }
                }
                let ver_name = newest.file_name().unwrap_or_default();
                let lib_base = Path::new(root).join("Lib").join(ver_name);
                for sub in &[r"ucrt\x64", r"um\x64"] {
                    let p = lib_base.join(sub);
                    if p.exists() {
                        libs.push(p);
                    }
                }
                break;
            }
        }
    }

    (includes, libs)
}

impl Linker {
    pub fn new() -> Result<Self, LinkerError> {
        let candidates = [
            PathBuf::from(r"C:\Program Files\LLVM\bin\clang.exe"),
            PathBuf::from(r"C:\Program Files (x86)\LLVM\bin\clang.exe"),
        ];

        let mut found_clang = None;
        for path in &candidates {
            if path.exists() {
                found_clang = Some(path.clone());
                break;
            }
        }

        if found_clang.is_none() {
            if let Ok(output) = Command::new("where.exe").arg("clang.exe").output() {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if let Some(first_line) = stdout.lines().next() {
                        let path = PathBuf::from(first_line.trim());
                        if path.exists() {
                            found_clang = Some(path);
                        }
                    }
                }
            }
        }

        let clang_path = found_clang.ok_or(LinkerError::LinkerNotFound)?;
        let (include_paths, lib_paths) = discover_windows_toolchain();

        Ok(Self {
            clang_path,
            include_paths,
            lib_paths,
        })
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

        let mut compile_cmd = Command::new(&self.clang_path);
        compile_cmd
            .arg("-target")
            .arg("x86_64-pc-windows-msvc")
            .arg("-Wno-override-module");

        for inc in &self.include_paths {
            compile_cmd.arg("-isystem").arg(inc);
        }

        compile_cmd
            .arg("-c")
            .arg(&runtime_c)
            .arg("-o")
            .arg(&runtime_obj);

        let compile_rt = compile_cmd.output()?;

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

        for lib in &self.lib_paths {
            cmd.arg(format!("-L{}", lib.display()));
        }

        cmd.arg("-o").arg(output_exe);

        // Standard Windows system libraries
        cmd.arg("-luser32");
        cmd.arg("-lgdi32");
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
