//! Command Line Interface (CLI) for VB6 Open Compiler (`vb6c`).

use clap::Parser;
use std::path::PathBuf;
use vb6_compiler::parse_source;
use vb6_project::VbProject;

#[derive(Parser, Debug)]
#[command(name = "vb6c")]
#[command(author = "VB6 Open Compiler Contributors")]
#[command(version = "0.1.0")]
#[command(about = "Open source Microsoft Visual Basic 6.0 compiler for Windows x64", long_about = None)]
struct Args {
    /// Path to the VB6 project (.vbp) or source file (.bas, .frm, .cls)
    #[arg(required = true)]
    input: PathBuf,

    /// Target architecture (x64, x86, arm64)
    #[arg(long, default_value = "x64")]
    target: String,

    /// Output binary path
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Build with release optimizations
    #[arg(long, default_value_t = false)]
    release: bool,

    /// Build with debug symbols
    #[arg(long, default_value_t = false)]
    debug: bool,

    /// Enable verbose diagnostic output
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    println!("VB6 Open Compiler x64 v0.1.0");
    println!("Compiling: {}", args.input.display());
    println!("Target Architecture: {}", args.target);

    let ext = args
        .input
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "vbp" => {
            println!("Loading VB6 Project...");
            let project = VbProject::load_from_file(&args.input)?;
            println!("Project Name: {}", project.name);
            println!("Startup: {}", project.startup);
            println!("Modules count: {}", project.modules.len());
            println!("Classes count: {}", project.classes.len());
            println!("Forms count: {}", project.forms.len());
        }
        "bas" | "frm" | "cls" => {
            let content = std::fs::read_to_string(&args.input)?;
            let ast = parse_source(&content)?;
            println!("Successfully parsed {} declarations.", ast.declarations.len());
            if args.verbose {
                println!("{:#?}", ast);
            }
        }
        _ => {
            anyhow::bail!("Unrecognized file format: .{}", ext);
        }
    }

    Ok(())
}
