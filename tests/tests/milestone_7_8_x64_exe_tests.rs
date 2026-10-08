use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use vb6_backend::Backend;
use vb6_compiler::ir::compile_to_ir;
use vb6_compiler::parse_source;
use vb6_linker::{Linker, PeInfo};
use vb6_project::VbProject;

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_path(prefix: &str, ext: &str) -> PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("{}_{}_{}_{}.{}", prefix, std::process::id(), time, id, ext))
}

#[test]
fn test_compile_empty_sub_main_to_pe32plus_and_run() {
    let source = r#"
Sub Main()
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse source");
    let ir = compile_to_ir(&ast, "EmptyMain");

    let obj_path = unique_path("test_m7_empty", "obj");
    let exe_path = unique_path("test_m7_empty", "exe");

    let backend = Backend::new("x64");
    backend
        .compile_to_object(&ir, &obj_path, true)
        .expect("Backend failed to emit object");

    let linker = Linker::new().expect("Failed to create linker");
    let pe_info = linker
        .link_executable(&[&obj_path], &exe_path)
        .expect("Linker failed to produce executable");

    // Validate PE32+ (x64) headers
    assert_eq!(
        pe_info.machine, 0x8664,
        "Expected IMAGE_FILE_MACHINE_AMD64 (0x8664)"
    );
    assert_eq!(pe_info.magic, 0x020B, "Expected PE32+ 64-bit magic (0x020B)");
    assert!(pe_info.entry_point > 0, "Entry point RVA must be non-zero");

    // Execute the generated native Windows x64 binary
    let status = Command::new(&exe_path)
        .status()
        .expect("Failed to run generated executable");
    assert!(status.success(), "Executable exited with non-zero status");

    let _ = std::fs::remove_file(&obj_path);
    let _ = std::fs::remove_file(&exe_path);
}

#[test]
fn test_compile_sub_main_with_assignment_and_run() {
    let source = r#"
Sub Main()
    Dim X As Long
    X = 10
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse source");
    let ir = compile_to_ir(&ast, "AssignMain");

    let obj_path = unique_path("test_m7_assign", "obj");
    let exe_path = unique_path("test_m7_assign", "exe");

    let backend = Backend::new("x64");
    backend
        .compile_to_object(&ir, &obj_path, true)
        .expect("Backend failed to emit object");

    let linker = Linker::new().expect("Failed to create linker");
    let pe_info = linker
        .link_executable(&[&obj_path], &exe_path)
        .expect("Linker failed to produce executable");

    assert_eq!(pe_info.machine, 0x8664);
    assert_eq!(pe_info.magic, 0x020B);

    let status = Command::new(&exe_path)
        .status()
        .expect("Failed to execute generated program");
    assert!(status.success(), "Program failed with exit code: {:?}", status.code());

    let _ = std::fs::remove_file(&obj_path);
    let _ = std::fs::remove_file(&exe_path);
}

#[test]
fn test_compile_golden_vbp_project_and_validate_pe32plus() {
    let vbp_path = Path::new("tests/golden/001_hello/001_hello.vbp");
    let path = if vbp_path.exists() {
        vbp_path.to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("golden/001_hello/001_hello.vbp")
    };

    let project = VbProject::load_from_file(&path).expect("Failed to load golden .vbp project");
    assert_eq!(project.settings.name, "HelloVB6");
    assert_eq!(project.modules.len(), 1);

    let ast = parse_source(&project.modules[0].code).expect("Failed to parse golden module");
    let ir = compile_to_ir(&ast, &project.settings.name);

    let obj_path = unique_path("test_m7_golden", "obj");
    let exe_path = unique_path("test_m7_golden", "exe");

    let backend = Backend::new("x64");
    backend
        .compile_to_object(&ir, &obj_path, true)
        .expect("Backend failed");

    let linker = Linker::new().expect("Linker discovery failed");
    let pe_info = linker
        .link_executable(&[&obj_path], &exe_path)
        .expect("Linking failed");

    assert_eq!(pe_info.machine, 0x8664);
    assert_eq!(pe_info.magic, 0x020B);
    assert!(exe_path.exists());

    // Inspect the generated binary directly with PeInfo
    let inspected = PeInfo::inspect(&exe_path).expect("PE inspection failed");
    assert_eq!(inspected.machine, 0x8664);
    assert_eq!(inspected.magic, 0x020B);

    let _ = std::fs::remove_file(&obj_path);
    let _ = std::fs::remove_file(&exe_path);
}

#[test]
fn test_runtime_minimal_lifecycle() {
    // Test Rust runtime functions
    vb6_runtime::vb6_init();
    vb6_runtime::vb6_debug_print(b"Test debug print from runtime\0".as_ptr());
    vb6_runtime::vb6_shutdown();

    // Verify embedded C runtime source is non-empty
    let c_src = vb6_runtime::get_runtime_c_source();
    assert!(c_src.contains("vb6_init"));
    assert!(c_src.contains("vb6_shutdown"));
    assert!(c_src.contains("vb6_msgbox"));
    assert!(c_src.contains("vb6_debug_print"));
}
