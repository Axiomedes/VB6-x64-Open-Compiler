//! Command Line Interface (CLI) for VB6 Open Compiler (`vb6c`).

use anyhow::Context;
use clap::Parser;
use std::path::PathBuf;
use vb6_backend::Backend;
use vb6_compiler::ir::compile_to_ir;
use vb6_compiler::parse_source;
use vb6_compiler::semantic::SemanticAnalyzer;
use vb6_linker::Linker;
use vb6_project::VbProject;

#[derive(Parser, Debug)]
#[command(name = "vb6c")]
#[command(author = "VB6 Open Compiler Contributors")]
#[command(version = "0.1.0")]
#[command(about = "Open source Microsoft Visual Basic 6.0 compiler for Windows x64 (PE32+)", long_about = None)]
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

    /// Emit textual VB6 IR to console
    #[arg(long, default_value_t = false)]
    emit_ir: bool,

    /// Emit textual LLVM IR to console
    #[arg(long, default_value_t = false)]
    emit_llvm: bool,

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
    println!("Source: {}", args.input.display());
    println!("Target: Windows {}", args.target);

    let ext = args
        .input
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let output_exe = args.output.clone().unwrap_or_else(|| {
        let mut p = args.input.clone();
        p.set_extension("exe");
        p
    });

    match ext.as_str() {
        "bas" | "frm" | "cls" => {
            compile_single_source_file(&args.input, &output_exe, &args)?;
        }
        "vbp" => {
            compile_vbp_project(&args.input, &output_exe, &args)?;
        }
        _ => {
            anyhow::bail!("Unrecognized file format: .{}", ext);
        }
    }

    Ok(())
}

fn compile_single_source_file(
    source_path: &PathBuf,
    output_exe: &PathBuf,
    args: &Args,
) -> anyhow::Result<()> {
    let module_name = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Module1");

    println!("Parsing {}...", source_path.display());
    let content = std::fs::read_to_string(source_path)
        .with_context(|| format!("Failed to read source file: {}", source_path.display()))?;

    let ext = source_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let (content, effective_module_name) = if ext == "cls" {
        let cls_mod = vb6_project::CLSLoader::parse(&content, source_path, Some(module_name.to_string()));
        let class_name = cls_mod.name.clone();
        (format!("Class {}\n{}\nEnd Class\n", class_name, cls_mod.code), class_name)
    } else if ext == "frm" {
        let form_mod = vb6_project::FRMParser::parse(&content, source_path);
        let desugared = vb6_project::FRMDesugarer::desugar(&form_mod);
        let form_name = form_mod.name.clone();
        let full_code = if !form_mod.code.to_ascii_lowercase().contains("sub main") {
            format!(
                "{}\nPublic Sub Main()\n    Set {} = New {}\n    {}.Show\n    vb6_do_events\nEnd Sub\n",
                desugared, form_name, form_name, form_name
            )
        } else {
            desugared
        };
        (full_code, form_name)
    } else {
        (content, module_name.to_string())
    };

    let ast = parse_source(&content)
        .map_err(|e| anyhow::anyhow!("Parser error: {}", e))?;

    println!("Semantic analysis...");
    let mut analyzer = SemanticAnalyzer::new();
    if let Err(errors) = analyzer.analyze_file(&ast) {
        eprintln!("Semantic verification failed with {} error(s):", errors.len());
        for err in errors {
            eprintln!("  - {}", err);
        }
        anyhow::bail!("Compilation aborted due to semantic errors.");
    }

    println!("Generating VB6 IR...");
    let ir = compile_to_ir(&ast, &effective_module_name);

    if args.emit_ir {
        let ir_text = vb6_compiler::ir::IrPrinter::print_module(&ir);
        println!("\n=== VB6 IR ===\n{}\n==============\n", ir_text);
    }

    let backend = Backend::new(&args.target);
    if args.emit_llvm {
        let llvm_ir = backend.emit_llvm_ir(&ir, true);
        println!("\n=== LLVM IR ===\n{}\n===============\n", llvm_ir);
    }

    println!("Generating native COFF x64 object file...");
    let temp_dir = std::env::temp_dir();
    let temp_obj = temp_dir.join(format!("vb6_build_{}.obj", std::process::id()));

    backend
        .compile_to_object(&ir, &temp_obj, true)
        .map_err(|e| anyhow::anyhow!("Backend code generation failed: {}", e))?;

    println!("Linking native PE32+ executable...");
    let linker = Linker::new().map_err(|e| anyhow::anyhow!("Linker discovery failed: {}", e))?;
    let pe_info = linker
        .link_executable(&[&temp_obj], output_exe)
        .map_err(|e| anyhow::anyhow!("Linking failed: {}", e))?;

    let _ = std::fs::remove_file(&temp_obj);

    println!("\n[SUCCESS] Built native Windows x64 executable: {}", output_exe.display());
    println!("  Machine:       IMAGE_FILE_MACHINE_AMD64 (0x{:04X})", pe_info.machine);
    println!("  Format:        PE32+ 64-bit (0x{:04X})", pe_info.magic);
    println!("  Subsystem:     {}", pe_info.subsystem);
    println!("  Sections:      {}", pe_info.section_count);
    println!("  Entry Point:   0x{:08X}", pe_info.entry_point);

    Ok(())
}

fn compile_vbp_project(
    project_path: &PathBuf,
    output_exe: &PathBuf,
    args: &Args,
) -> anyhow::Result<()> {
    println!("Loading project: {}...", project_path.display());
    let project = VbProject::load_from_file(project_path)
        .map_err(|e| anyhow::anyhow!("Failed to load project: {}", e))?;

    println!("Project: {}", project.settings.name);
    println!("Startup: {}", project.settings.startup);

    // If there is a standard module or startup file, compile it
    let mut combined_declarations = Vec::new();

    for m in &project.modules {
        println!("Reading module {}: {}", m.name, m.path.display());
        let mut ast = parse_source(&m.code)
            .map_err(|e| anyhow::anyhow!("Failed parsing {}: {}", m.name, e))?;
        combined_declarations.append(&mut ast.declarations);
    }

    for c in &project.classes {
        println!("Reading class module {}: {}", c.name, c.path.display());
        let wrapped_code = format!("Class {}\n{}\nEnd Class\n", c.name, c.code);
        let mut ast = parse_source(&wrapped_code)
            .map_err(|e| anyhow::anyhow!("Failed parsing class {}: {}", c.name, e))?;
        combined_declarations.append(&mut ast.declarations);
    }

    for f in &project.forms {
        println!("Reading form module {}: {}", f.name, f.path.display());
        let desugared = vb6_project::FRMDesugarer::desugar(f);
        let mut ast = parse_source(&desugared)
            .map_err(|e| anyhow::anyhow!("Failed parsing form {}: {}", f.name, e))?;
        combined_declarations.append(&mut ast.declarations);
    }

    // Check if Sub Main is already defined
    let has_main = combined_declarations.iter().any(|d| match d {
        vb6_compiler::ast::Declaration::Function(f) => f.name.eq_ignore_ascii_case("Main"),
        _ => false,
    });

    if !has_main {
        let startup_form = if !project.settings.startup.is_empty() && !project.settings.startup.eq_ignore_ascii_case("Sub Main") {
            project.settings.startup.clone()
        } else if let Some(first_form) = project.forms.first() {
            first_form.name.clone()
        } else {
            String::new()
        };

        if !startup_form.is_empty() {
            println!("Synthesizing startup entry point for form: {}", startup_form);
            let startup_code = format!(
                "Public Sub Main()\n    Set {} = New {}\n    {}.Show\n    vb6_do_events\nEnd Sub\n",
                startup_form, startup_form, startup_form
            );
            let mut ast = parse_source(&startup_code)
                .map_err(|e| anyhow::anyhow!("Failed generating startup wrapper: {}", e))?;
            combined_declarations.append(&mut ast.declarations);
        }
    }

    if combined_declarations.is_empty() {
        anyhow::bail!("No modules, classes, or forms found in project.");
    }

    let combined_ast = vb6_compiler::ast::SourceFile {
        declarations: combined_declarations,
    };

    println!("Semantic analysis for project...");
    let mut analyzer = SemanticAnalyzer::new();
    if let Err(errors) = analyzer.analyze_file(&combined_ast) {
        eprintln!("Semantic errors in project:");
        for err in errors {
            eprintln!("  - {}", err);
        }
        anyhow::bail!("Compilation halted due to semantic errors.");
    }

    println!("Generating IR for project...");
    let ir = compile_to_ir(&combined_ast, &project.settings.name);

    let backend = Backend::new(&args.target);
    let temp_dir = std::env::temp_dir();
    let temp_obj = temp_dir.join(format!("vb6_proj_{}.obj", std::process::id()));

    backend
        .compile_to_object(&ir, &temp_obj, true)
        .map_err(|e| anyhow::anyhow!("Backend failed: {}", e))?;

    println!("Linking PE32+ executable...");
    let linker = Linker::new().map_err(|e| anyhow::anyhow!("Linker error: {}", e))?;
    let pe_info = linker
        .link_executable(&[&temp_obj], output_exe)
        .map_err(|e| anyhow::anyhow!("Linker failed: {}", e))?;

    let _ = std::fs::remove_file(&temp_obj);

    println!("\n[SUCCESS] Project successfully compiled: {}", output_exe.display());
    println!("  Machine:       IMAGE_FILE_MACHINE_AMD64 (0x{:04X})", pe_info.machine);
    println!("  Format:        PE32+ 64-bit (0x{:04X})", pe_info.magic);
    println!("  Sections:      {}", pe_info.section_count);

    Ok(())
}
