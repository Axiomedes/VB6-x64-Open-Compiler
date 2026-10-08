use vb6_backend::Backend;
use vb6_compiler::ir::compile_to_ir;
use vb6_compiler::parse_source;

#[test]
fn test_emit_llvm_ir_conceptual_program() {
    let source = r#"
Sub Main()
    Dim X As Long
    X = 10
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse");
    let ir = compile_to_ir(&ast, "TestModule");
    let backend = Backend::new("x64");
    let llvm_ir = backend.emit_llvm_ir(&ir, true);

    assert!(llvm_ir.contains("target triple = \"x86_64-pc-windows-msvc\""));
    assert!(llvm_ir.contains("define void @\"Main\"()"));
    assert!(llvm_ir.contains("%\"local_X\" = alloca i32, align 4"));
    assert!(llvm_ir.contains("store i32 10, ptr %\"local_X\""));
    assert!(llvm_ir.contains("define i32 @main()"));
    assert!(llvm_ir.contains("call void @\"Main\"()"));
}

#[test]
fn test_emit_llvm_ir_arithmetic_and_comparison() {
    let source = r#"
Function AddNumbers(ByVal A As Long, ByVal B As Long) As Long
    AddNumbers = A + B
End Function
"#;

    let ast = parse_source(source).expect("Failed to parse");
    let ir = compile_to_ir(&ast, "MathModule");
    let backend = Backend::new("x64");
    let llvm_ir = backend.emit_llvm_ir(&ir, false);

    assert!(llvm_ir.contains("define i32 @\"AddNumbers\"(i32 %\"arg_A\", i32 %\"arg_B\")"));
    assert!(llvm_ir.contains("add i32"));
}

#[test]
fn test_compile_to_native_coff_obj() {
    let source = r#"
Sub Main()
    Dim X As Long
    X = 42
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse");
    let ir = compile_to_ir(&ast, "ProbeModule");
    let backend = Backend::new("x64");

    let temp_dir = std::env::temp_dir();
    let obj_path = temp_dir.join(format!("test_probe_{}.obj", std::process::id()));

    let res = backend.compile_to_object(&ir, &obj_path, true);
    assert!(res.is_ok(), "Expected compilation to object file to succeed: {:?}", res.err());
    assert!(obj_path.exists(), "Object file must exist on disk");

    // Read COFF header bytes to verify IMAGE_FILE_MACHINE_AMD64 (0x8664)
    let bytes = std::fs::read(&obj_path).expect("Failed to read obj");
    assert!(bytes.len() > 20, "Object file too small");
    let machine = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(
        machine, 0x8664,
        "Expected COFF Machine to be IMAGE_FILE_MACHINE_AMD64 (0x8664), got 0x{:04X}",
        machine
    );

    let _ = std::fs::remove_file(&obj_path);
}
