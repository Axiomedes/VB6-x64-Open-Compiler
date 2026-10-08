use vb6_compiler::ast::*;
use vb6_compiler::parse_source;

#[test]
fn test_milestone_2_conceptual_example() {
    let source = r#"
Sub Main()
    Dim X As Long
    X = 10
    If X > 5 Then
        Debug.Print X
    End If
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse conceptual example");
    assert_eq!(ast.declarations.len(), 1);

    if let Declaration::Sub(s) = &ast.declarations[0] {
        assert_eq!(s.name, "Main");
        assert_eq!(s.body.len(), 3);

        // 1. Dim X As Long
        match &s.body[0] {
            Statement::VariableDeclaration {
                kind, variables, ..
            } => {
                assert_eq!(*kind, VarDeclKind::Dim);
                assert_eq!(variables.len(), 1);
                assert_eq!(variables[0].name, "X");
                assert_eq!(variables[0].type_spec.as_ref().unwrap().name, "Long");
            }
            other => panic!("Expected VariableDeclaration, got {:?}", other),
        }

        // 2. X = 10
        match &s.body[1] {
            Statement::Assignment {
                target,
                value,
                is_set,
                ..
            } => {
                assert!(!is_set);
                assert_eq!(*target, Expression::Variable("X".to_string()));
                assert_eq!(*value, Expression::Literal(Literal::Integer(10)));
            }
            other => panic!("Expected Assignment, got {:?}", other),
        }

        // 3. If X > 5 Then Debug.Print X End If
        match &s.body[2] {
            Statement::If {
                branches,
                else_branch,
                ..
            } => {
                assert_eq!(branches.len(), 1);
                assert!(else_branch.is_none());
                let b = &branches[0];
                match &b.condition {
                    Expression::BinaryOp { op, left, right } => {
                        assert_eq!(*op, BinaryOperator::GreaterThan);
                        assert_eq!(**left, Expression::Variable("X".to_string()));
                        assert_eq!(**right, Expression::Literal(Literal::Integer(5)));
                    }
                    other => panic!("Expected binary op condition, got {:?}", other),
                }

                assert_eq!(b.body.len(), 1);
                match &b.body[0] {
                    Statement::DebugPrint { arguments, .. } => {
                        assert_eq!(arguments.len(), 1);
                        assert_eq!(arguments[0], Expression::Variable("X".to_string()));
                    }
                    other => panic!("Expected DebugPrint, got {:?}", other),
                }
            }
            other => panic!("Expected If statement, got {:?}", other),
        }
    } else {
        panic!("Expected Sub declaration");
    }

    // Verify ASTPrinter output
    let printed = ASTPrinter::print(&ast);
    assert!(printed.contains("Program"));
    assert!(printed.contains("SubDeclaration: Main"));
    assert!(printed.contains("VariableDeclaration: Dim X As Long"));
    assert!(printed.contains("IfStatement"));
    assert!(printed.contains("DebugPrint"));
}

#[test]
fn test_operator_precedence_arithmetic_and_logical() {
    let source = r#"
Sub TestPrecedence()
    Dim Res As Boolean
    Res = 2 + 3 * 4 > 10 And 5 < 20
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse precedence test");
    if let Declaration::Sub(s) = &ast.declarations[0] {
        if let Statement::Assignment { value, .. } = &s.body[1] {
            // Should be parsed as: ((2 + (3 * 4)) > 10) And (5 < 20)
            match value {
                Expression::BinaryOp { op, left, right } => {
                    assert_eq!(*op, BinaryOperator::And);
                    // Left is comparison >
                    match &**left {
                        Expression::BinaryOp {
                            op: op_comp,
                            left: left_add,
                            right: right_10,
                        } => {
                            assert_eq!(*op_comp, BinaryOperator::GreaterThan);
                            assert_eq!(**right_10, Expression::Literal(Literal::Integer(10)));
                            // left_add is 2 + (3 * 4)
                            match &**left_add {
                                Expression::BinaryOp {
                                    op: op_add,
                                    left: left_2,
                                    right: right_mul,
                                } => {
                                    assert_eq!(*op_add, BinaryOperator::Add);
                                    assert_eq!(**left_2, Expression::Literal(Literal::Integer(2)));
                                    match &**right_mul {
                                        Expression::BinaryOp {
                                            op: op_mul,
                                            left: l3,
                                            right: r4,
                                        } => {
                                            assert_eq!(*op_mul, BinaryOperator::Multiply);
                                            assert_eq!(**l3, Expression::Literal(Literal::Integer(3)));
                                            assert_eq!(**r4, Expression::Literal(Literal::Integer(4)));
                                        }
                                        other => panic!("Expected Multiply, got {:?}", other),
                                    }
                                }
                                other => panic!("Expected Add, got {:?}", other),
                            }
                        }
                        other => panic!("Expected Comparison, got {:?}", other),
                    }

                    // Right is comparison <
                    match &**right {
                        Expression::BinaryOp { op: op_comp2, .. } => {
                            assert_eq!(*op_comp2, BinaryOperator::LessThan);
                        }
                        other => panic!("Expected Comparison, got {:?}", other),
                    }
                }
                other => panic!("Expected BinaryOp And, got {:?}", other),
            }
        }
    }
}

#[test]
fn test_control_flow_loops_and_select() {
    let source = r#"
Sub TestLoops()
    Dim I As Long
    For I = 1 To 10 Step 2
        If I = 5 Then Exit For
    Next I

    Do While I > 0
        I = I - 1
    Loop

    Select Case I
        Case 0
            Debug.Print "Zero"
        Case 1 To 5
            Debug.Print "Small"
        Case Is > 10
            Debug.Print "Big"
        Case Else
            Debug.Print "Other"
    End Select
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse loops and select case");
    if let Declaration::Sub(s) = &ast.declarations[0] {
        assert_eq!(s.body.len(), 4);

        // 1. Dim
        assert!(matches!(s.body[0], Statement::VariableDeclaration { .. }));

        // 2. For loop
        if let Statement::For { counter, step, body, .. } = &s.body[1] {
            assert_eq!(counter, "I");
            assert!(step.is_some());
            assert_eq!(body.len(), 1);
            assert!(matches!(body[0], Statement::If { .. }));
        } else {
            panic!("Expected For loop");
        }

        // 3. Do loop
        if let Statement::DoLoop { condition, body, .. } = &s.body[2] {
            assert!(matches!(condition, Some(LoopCondition::While(_))));
            assert_eq!(body.len(), 1);
        } else {
            panic!("Expected DoLoop");
        }

        // 4. Select Case
        if let Statement::SelectCase { cases, else_case, .. } = &s.body[3] {
            assert_eq!(cases.len(), 3);
            assert!(else_case.is_some());
        } else {
            panic!("Expected SelectCase");
        }
    }
}

#[test]
fn test_error_handling_statements() {
    let source = r#"
Sub TestErrors()
    On Error GoTo ErrHandler
    Dim X As Long
    X = 1 / 0
    Exit Sub

ErrHandler:
    On Error Resume Next
    Resume Next
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse error handling statements");
    if let Declaration::Sub(s) = &ast.declarations[0] {
        assert!(matches!(s.body[0], Statement::OnError { action: OnErrorAction::GoTo(ref lbl), .. } if lbl == "ErrHandler"));
        assert!(matches!(s.body[4], Statement::LineLabel { ref name, .. } if name == "ErrHandler"));
        assert!(matches!(s.body[5], Statement::OnError { action: OnErrorAction::ResumeNext, .. }));
        assert!(matches!(s.body[6], Statement::Resume { action: ResumeAction::Next, .. }));
    }
}

#[test]
fn test_ast_visitor_traversal() {
    let source = r#"
Sub ProcessData()
    Dim Total As Double
    Total = 100.5
    If Total > 50 Then
        Debug.Print Total
    End If
End Sub
"#;

    let ast = parse_source(source).expect("Failed to parse source");

    #[derive(Default)]
    struct CounterVisitor {
        sub_count: usize,
        var_count: usize,
        expr_count: usize,
    }

    impl ASTVisitor for CounterVisitor {
        fn visit_sub_decl(&mut self, sub: &SubDeclaration) {
            self.sub_count += 1;
            for s in &sub.body {
                self.visit_statement(s);
            }
        }

        fn visit_statement(&mut self, stmt: &Statement) {
            if let Statement::VariableDeclaration { variables, .. } = stmt {
                self.var_count += variables.len();
            }
            // Chain to default implementation
            match stmt {
                Statement::Assignment { target, value, .. } => {
                    self.visit_expression(target);
                    self.visit_expression(value);
                }
                Statement::If { branches, .. } => {
                    for b in branches {
                        self.visit_expression(&b.condition);
                        for s in &b.body {
                            self.visit_statement(s);
                        }
                    }
                }
                Statement::DebugPrint { arguments, .. } => {
                    for a in arguments {
                        self.visit_expression(a);
                    }
                }
                _ => {}
            }
        }

        fn visit_expression(&mut self, expr: &Expression) {
            self.expr_count += 1;
            if let Expression::BinaryOp { left, right, .. } = expr {
                self.visit_expression(left);
                self.visit_expression(right);
            }
        }
    }

    let mut visitor = CounterVisitor::default();
    visitor.visit_source_file(&ast);

    assert_eq!(visitor.sub_count, 1);
    assert_eq!(visitor.var_count, 1);
    assert!(visitor.expr_count >= 3);
}
