use vb6_compiler::ir::*;
use vb6_compiler::parse_source;
use vb6_compiler::types::VbType;

#[test]
fn test_milestone_4_conceptual_example() {
    let source = r#"
Sub Main()
    Dim X As Long
    X = 10
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse source");
    let ir = compile_to_ir(&ast, "TestModule");

    assert_eq!(ir.name, "TestModule");
    assert_eq!(ir.functions.len(), 1);

    let func = &ir.functions[0];
    assert_eq!(func.name, "Main");
    assert!(!func.blocks.is_empty());

    let entry = &func.blocks[0];
    assert_eq!(entry.label, "entry");

    // 1. ALLOC_LOCAL X : Long
    let has_alloc = entry.instructions.iter().any(|inst| {
        matches!(
            inst,
            IrInstruction::AllocLocal {
                name,
                vb_type: VbType::Long,
                ..
            } if name == "X"
        )
    });
    assert!(has_alloc, "Expected AllocLocal for X : Long");

    // 2. STORE Local("X"), ConstInt(10, Long)
    let has_store = entry.instructions.iter().any(|inst| {
        matches!(
            inst,
            IrInstruction::Store {
                dest: Operand::Local(ref name),
                src: Operand::Constant(IrConstant::Int(10, VbType::Long)),
            } if name == "X"
        )
    });
    assert!(has_store, "Expected Store for X = 10");

    // 3. RETURN
    let has_return = entry.instructions.iter().any(|inst| {
        matches!(inst, IrInstruction::Return { value: None })
    });
    assert!(has_return, "Expected terminating Return");

    // Test IR Pretty Printer
    let printed = IrPrinter::print_module(&ir);
    assert!(printed.contains("module \"TestModule\""));
    assert!(printed.contains("function Main()"));
    assert!(printed.contains("ALLOC_LOCAL \"X\" : Long"));
    assert!(printed.contains("STORE Local(\"X\"), ConstInt(10, Long)"));
    assert!(printed.contains("RETURN"));
}

#[test]
fn test_arithmetic_expression_lowering() {
    let source = r#"
Sub Compute()
    Dim Res As Long
    Res = 2 + 3 * 4
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse source");
    let ir = compile_to_ir(&ast, "ComputeModule");
    let entry = &ir.functions[0].blocks[0];

    // Verify Mul is emitted before Add
    let mut saw_mul = false;
    let mut saw_add = false;

    for inst in &entry.instructions {
        if let IrInstruction::BinaryOp { op, .. } = inst {
            if *op == IrBinaryOp::Mul {
                saw_mul = true;
            } else if *op == IrBinaryOp::Add && saw_mul {
                saw_add = true;
            }
        }
    }

    assert!(saw_mul, "Expected multiplication operation");
    assert!(saw_add, "Expected addition operation after multiplication");
}

#[test]
fn test_string_concat_lowering() {
    let source = r#"
Sub ConcatTest()
    Dim S As String
    S = "Hello " & "World"
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse source");
    let ir = compile_to_ir(&ast, "StringModule");
    let entry = &ir.functions[0].blocks[0];

    let has_concat = entry.instructions.iter().any(|inst| {
        matches!(
            inst,
            IrInstruction::StringConcat {
                left: Operand::Constant(IrConstant::String(ref l)),
                right: Operand::Constant(IrConstant::String(ref r)),
                ..
            } if l == "Hello " && r == "World"
        )
    });
    assert!(has_concat, "Expected StringConcat instruction");
}

#[test]
fn test_if_branching_basic_blocks() {
    let source = r#"
Sub BranchTest()
    Dim X As Long
    X = 10
    If X > 5 Then
        MsgBox "High"
    Else
        MsgBox "Low"
    End If
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse source");
    let ir = compile_to_ir(&ast, "BranchModule");
    let func = &ir.functions[0];

    // Entry should have Compare and BranchIf
    let entry = &func.blocks[0];
    let has_branch_if = entry.instructions.iter().any(|inst| {
        matches!(inst, IrInstruction::BranchIf { .. })
    });
    assert!(has_branch_if, "Expected BranchIf instruction in entry block");

    // Multiple blocks must exist: entry, then, else, merge
    assert!(func.blocks.len() >= 4, "Expected at least 4 basic blocks for If/Else");
}

#[test]
fn test_for_loop_basic_blocks() {
    let source = r#"
Sub LoopTest()
    Dim I As Long
    For I = 1 To 10 Step 2
        Debug.Print I
    Next I
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse source");
    let ir = compile_to_ir(&ast, "LoopModule");
    let func = &ir.functions[0];

    // Must have header, body, exit blocks
    let has_header = func.blocks.iter().any(|b| b.label.contains("for_header"));
    let has_body = func.blocks.iter().any(|b| b.label.contains("for_body"));
    let has_exit = func.blocks.iter().any(|b| b.label.contains("for_exit"));

    assert!(has_header, "Expected for_header basic block");
    assert!(has_body, "Expected for_body basic block");
    assert!(has_exit, "Expected for_exit basic block");
}

#[test]
fn test_error_handling_ir_lowering() {
    let source = r#"
Sub ErrorTest()
    On Error GoTo CatchBlock
    Exit Sub
CatchBlock:
    Resume Next
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse source");
    let ir = compile_to_ir(&ast, "ErrorModule");
    let func = &ir.functions[0];

    let mut saw_push = false;
    let mut saw_resume = false;

    for b in &func.blocks {
        for inst in &b.instructions {
            if let IrInstruction::PushErrorHandler { target } = inst {
                if target == "CatchBlock" {
                    saw_push = true;
                }
            }
            if matches!(inst, IrInstruction::Resume { .. }) {
                saw_resume = true;
            }
        }
    }

    assert!(saw_push, "Expected PushErrorHandler instruction");
    assert!(saw_resume, "Expected Resume instruction");
}
