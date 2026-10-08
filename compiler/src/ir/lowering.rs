//! Lowering from AST + Semantic Model to VB6 Intermediate Representation (IR).

use crate::ast::decl::*;
use crate::ast::expr::*;
use crate::ast::stmt::*;
use crate::ast::SourceFile;
use crate::ir::model::*;
use crate::types::VbType;

pub struct AstToIrLowering {
    temp_counter: usize,
    block_counter: usize,
    current_blocks: Vec<BasicBlock>,
    current_block_idx: usize,
}

impl Default for AstToIrLowering {
    fn default() -> Self {
        Self::new()
    }
}

impl AstToIrLowering {
    pub fn new() -> Self {
        Self {
            temp_counter: 0,
            block_counter: 0,
            current_blocks: Vec::new(),
            current_block_idx: 0,
        }
    }

    fn next_temp(&mut self) -> usize {
        let t = self.temp_counter;
        self.temp_counter += 1;
        t
    }

    fn next_label(&mut self, prefix: &str) -> String {
        self.block_counter += 1;
        format!("{}_{}", prefix, self.block_counter)
    }

    fn emit(&mut self, inst: IrInstruction) {
        if self.current_block_idx < self.current_blocks.len() {
            self.current_blocks[self.current_block_idx]
                .instructions
                .push(inst);
        }
    }

    fn create_block(&mut self, label: &str) -> usize {
        let block = BasicBlock::new(label);
        self.current_blocks.push(block);
        self.current_blocks.len() - 1
    }

    fn switch_to_block(&mut self, idx: usize) {
        self.current_block_idx = idx;
    }

    /// Lower an entire SourceFile into an IrModule.
    pub fn lower_source_file(&mut self, file: &SourceFile, module_name: &str) -> IrModule {
        let mut functions = Vec::new();
        let mut globals = Vec::new();

        for decl in &file.declarations {
            match decl {
                Declaration::Sub(s) => {
                    let func = self.lower_sub(s);
                    functions.push(func);
                }
                Declaration::Function(f) => {
                    let func = self.lower_function(f);
                    functions.push(func);
                }
                Declaration::ModuleVariables(v) => {
                    for var in &v.variables {
                        let vb_type = var
                            .type_spec
                            .as_ref()
                            .map(|ts| VbType::from_name(&ts.name))
                            .unwrap_or(VbType::Variant);
                        globals.push((var.name.clone(), vb_type));
                    }
                }
                _ => {}
            }
        }

        IrModule {
            name: module_name.to_string(),
            globals,
            functions,
        }
    }

    fn lower_sub(&mut self, sub: &SubDeclaration) -> IrFunction {
        self.temp_counter = 0;
        self.block_counter = 0;
        self.current_blocks.clear();

        let entry_idx = self.create_block("entry");
        self.switch_to_block(entry_idx);

        let params: Vec<(String, VbType)> = sub
            .params
            .iter()
            .map(|p| {
                let ty = p
                    .param_type
                    .as_ref()
                    .map(|ts| VbType::from_name(&ts.name))
                    .unwrap_or(VbType::Variant);
                (p.name.clone(), ty)
            })
            .collect();

        // Parameter copies into locals
        for (name, ty) in &params {
            let temp = self.next_temp();
            self.emit(IrInstruction::AllocLocal {
                dest: temp,
                name: name.clone(),
                vb_type: ty.clone(),
            });
        }

        // Lower body statements
        for stmt in &sub.body {
            self.lower_statement(stmt);
        }

        // Ensure terminating return in the last block
        let last_idx = self.current_blocks.len() - 1;
        let needs_return = if let Some(last_inst) = self.current_blocks[last_idx].instructions.last() {
            !matches!(last_inst, IrInstruction::Return { .. } | IrInstruction::Branch { .. })
        } else {
            true
        };

        if needs_return {
            self.current_blocks[last_idx]
                .instructions
                .push(IrInstruction::Return { value: None });
        }

        IrFunction {
            name: sub.name.clone(),
            params,
            return_type: None,
            blocks: self.current_blocks.clone(),
        }
    }

    fn lower_function(&mut self, func: &FunctionDeclaration) -> IrFunction {
        self.temp_counter = 0;
        self.block_counter = 0;
        self.current_blocks.clear();

        let entry_idx = self.create_block("entry");
        self.switch_to_block(entry_idx);

        let params: Vec<(String, VbType)> = func
            .params
            .iter()
            .map(|p| {
                let ty = p
                    .param_type
                    .as_ref()
                    .map(|ts| VbType::from_name(&ts.name))
                    .unwrap_or(VbType::Variant);
                (p.name.clone(), ty)
            })
            .collect();

        let ret_ty = func
            .return_type
            .as_ref()
            .map(|ts| VbType::from_name(&ts.name))
            .unwrap_or(VbType::Variant);

        for (name, ty) in &params {
            let temp = self.next_temp();
            self.emit(IrInstruction::AllocLocal {
                dest: temp,
                name: name.clone(),
                vb_type: ty.clone(),
            });
        }

        // Allocate return value variable (named after function in VB6)
        let ret_temp = self.next_temp();
        self.emit(IrInstruction::AllocLocal {
            dest: ret_temp,
            name: func.name.clone(),
            vb_type: ret_ty.clone(),
        });

        for stmt in &func.body {
            self.lower_statement(stmt);
        }

        let last_idx = self.current_blocks.len() - 1;
        let needs_return = if let Some(last_inst) = self.current_blocks[last_idx].instructions.last() {
            !matches!(last_inst, IrInstruction::Return { .. } | IrInstruction::Branch { .. })
        } else {
            true
        };

        if needs_return {
            let ret_val_temp = self.next_temp();
            self.current_blocks[last_idx].instructions.push(IrInstruction::Load {
                dest: ret_val_temp,
                src: Operand::Local(func.name.clone()),
            });
            self.current_blocks[last_idx].instructions.push(IrInstruction::Return {
                value: Some(Operand::Temp(ret_val_temp)),
            });
        }

        IrFunction {
            name: func.name.clone(),
            params,
            return_type: Some(ret_ty),
            blocks: self.current_blocks.clone(),
        }
    }

    fn lower_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration { variables, .. } => {
                for v in variables {
                    let vb_type = v
                        .type_spec
                        .as_ref()
                        .map(|ts| VbType::from_name(&ts.name))
                        .unwrap_or(VbType::Variant);

                    let dest = self.next_temp();
                    self.emit(IrInstruction::AllocLocal {
                        dest,
                        name: v.name.clone(),
                        vb_type,
                    });

                    if let Some(init) = &v.init_value {
                        let init_op = self.lower_expression(init);
                        self.emit(IrInstruction::Store {
                            dest: Operand::Local(v.name.clone()),
                            src: init_op,
                        });
                    }
                }
            }
            Statement::Assignment { target, value, .. } => {
                let val_op = self.lower_expression(value);
                let target_op = match target {
                    Expression::Variable(name) => Operand::Local(name.clone()),
                    _ => Operand::Local("Unknown".to_string()),
                };
                self.emit(IrInstruction::Store {
                    dest: target_op,
                    src: val_op,
                });
            }
            Statement::If {
                branches,
                else_branch,
                ..
            } => {
                let merge_label = self.next_label("if_merge");

                for (i, branch) in branches.iter().enumerate() {
                    let cond_op = self.lower_expression(&branch.condition);
                    let then_label = self.next_label("if_then");
                    let next_test_label = if i + 1 < branches.len() || else_branch.is_some() {
                        self.next_label("if_next")
                    } else {
                        merge_label.clone()
                    };

                    self.emit(IrInstruction::BranchIf {
                        condition: cond_op,
                        true_target: then_label.clone(),
                        false_target: next_test_label.clone(),
                    });

                    // Then block
                    let then_idx = self.create_block(&then_label);
                    self.switch_to_block(then_idx);
                    for s in &branch.body {
                        self.lower_statement(s);
                    }
                    self.emit(IrInstruction::Branch {
                        target: merge_label.clone(),
                    });

                    // Next test block
                    if next_test_label != merge_label {
                        let next_idx = self.create_block(&next_test_label);
                        self.switch_to_block(next_idx);
                    }
                }

                if let Some(else_stmts) = else_branch {
                    for s in else_stmts {
                        self.lower_statement(s);
                    }
                    self.emit(IrInstruction::Branch {
                        target: merge_label.clone(),
                    });
                }

                let merge_idx = self.create_block(&merge_label);
                self.switch_to_block(merge_idx);
            }
            Statement::For {
                counter,
                start,
                end,
                step,
                body,
                ..
            } => {
                // Initialize counter
                let start_op = self.lower_expression(start);
                self.emit(IrInstruction::Store {
                    dest: Operand::Local(counter.clone()),
                    src: start_op,
                });

                let header_label = self.next_label("for_header");
                let body_label = self.next_label("for_body");
                let exit_label = self.next_label("for_exit");

                self.emit(IrInstruction::Branch {
                    target: header_label.clone(),
                });

                // Header block (condition test)
                let header_idx = self.create_block(&header_label);
                self.switch_to_block(header_idx);

                let counter_load_temp = self.next_temp();
                self.emit(IrInstruction::Load {
                    dest: counter_load_temp,
                    src: Operand::Local(counter.clone()),
                });

                let end_op = self.lower_expression(end);
                let cmp_temp = self.next_temp();
                self.emit(IrInstruction::Compare {
                    dest: cmp_temp,
                    op: IrCompareOp::LessThanOrEqual,
                    left: Operand::Temp(counter_load_temp),
                    right: end_op,
                });

                self.emit(IrInstruction::BranchIf {
                    condition: Operand::Temp(cmp_temp),
                    true_target: body_label.clone(),
                    false_target: exit_label.clone(),
                });

                // Body block
                let body_idx = self.create_block(&body_label);
                self.switch_to_block(body_idx);
                for s in body {
                    self.lower_statement(s);
                }

                // Step increment
                let step_op = if let Some(st) = step {
                    self.lower_expression(st)
                } else {
                    Operand::Constant(IrConstant::Int(1, VbType::Long))
                };

                let curr_val_temp = self.next_temp();
                self.emit(IrInstruction::Load {
                    dest: curr_val_temp,
                    src: Operand::Local(counter.clone()),
                });

                let add_temp = self.next_temp();
                self.emit(IrInstruction::BinaryOp {
                    dest: add_temp,
                    op: IrBinaryOp::Add,
                    left: Operand::Temp(curr_val_temp),
                    right: step_op,
                    result_type: VbType::Long,
                });

                self.emit(IrInstruction::Store {
                    dest: Operand::Local(counter.clone()),
                    src: Operand::Temp(add_temp),
                });

                self.emit(IrInstruction::Branch {
                    target: header_label,
                });

                // Exit block
                let exit_idx = self.create_block(&exit_label);
                self.switch_to_block(exit_idx);
            }
            Statement::DoLoop {
                condition,
                condition_at_start,
                body,
                ..
            } => {
                let header_label = self.next_label("loop_header");
                let body_label = self.next_label("loop_body");
                let exit_label = self.next_label("loop_exit");

                self.emit(IrInstruction::Branch {
                    target: header_label.clone(),
                });

                let header_idx = self.create_block(&header_label);
                self.switch_to_block(header_idx);

                if *condition_at_start {
                    if let Some(cond) = condition {
                        let (cond_op, is_until) = match cond {
                            LoopCondition::While(e) => (self.lower_expression(e), false),
                            LoopCondition::Until(e) => (self.lower_expression(e), true),
                        };

                        let (true_t, false_t) = if is_until {
                            (exit_label.clone(), body_label.clone())
                        } else {
                            (body_label.clone(), exit_label.clone())
                        };

                        self.emit(IrInstruction::BranchIf {
                            condition: cond_op,
                            true_target: true_t,
                            false_target: false_t,
                        });
                    }
                } else {
                    self.emit(IrInstruction::Branch {
                        target: body_label.clone(),
                    });
                }

                let body_idx = self.create_block(&body_label);
                self.switch_to_block(body_idx);
                for s in body {
                    self.lower_statement(s);
                }

                if !condition_at_start {
                    if let Some(cond) = condition {
                        let (cond_op, is_until) = match cond {
                            LoopCondition::While(e) => (self.lower_expression(e), false),
                            LoopCondition::Until(e) => (self.lower_expression(e), true),
                        };

                        let (true_t, false_t) = if is_until {
                            (exit_label.clone(), header_label.clone())
                        } else {
                            (header_label.clone(), exit_label.clone())
                        };

                        self.emit(IrInstruction::BranchIf {
                            condition: cond_op,
                            true_target: true_t,
                            false_target: false_t,
                        });
                    } else {
                        self.emit(IrInstruction::Branch {
                            target: header_label,
                        });
                    }
                } else {
                    self.emit(IrInstruction::Branch {
                        target: header_label,
                    });
                }

                let exit_idx = self.create_block(&exit_label);
                self.switch_to_block(exit_idx);
            }
            Statement::Call {
                name, arguments, ..
            } => {
                let args_ops = arguments
                    .iter()
                    .map(|arg| self.lower_expression(arg))
                    .collect();
                self.emit(IrInstruction::Call {
                    dest: None,
                    name: name.clone(),
                    args: args_ops,
                });
            }
            Statement::DebugPrint { arguments, .. } => {
                let args_ops = arguments
                    .iter()
                    .map(|arg| self.lower_expression(arg))
                    .collect();
                self.emit(IrInstruction::DebugPrint { args: args_ops });
            }
            Statement::OnError { action, .. } => match action {
                OnErrorAction::GoTo(label) => {
                    self.emit(IrInstruction::PushErrorHandler {
                        target: label.clone(),
                    });
                }
                OnErrorAction::ResumeNext => {
                    self.emit(IrInstruction::PushErrorHandler {
                        target: "__resume_next".to_string(),
                    });
                }
                OnErrorAction::GoToZero => {
                    self.emit(IrInstruction::PopErrorHandler);
                }
            },
            Statement::Resume { action, .. } => match action {
                ResumeAction::Label(lbl) => {
                    self.emit(IrInstruction::Resume {
                        target: Some(lbl.clone()),
                    });
                }
                ResumeAction::Next => {
                    self.emit(IrInstruction::Resume {
                        target: Some("__resume_next".to_string()),
                    });
                }
                ResumeAction::Current => {
                    self.emit(IrInstruction::Resume { target: None });
                }
            },
            Statement::Exit { target, .. } => match target {
                ExitTarget::Sub | ExitTarget::Function | ExitTarget::Property => {
                    self.emit(IrInstruction::Return { value: None });
                }
                _ => {}
            },
            Statement::GoTo { label, .. } => {
                self.emit(IrInstruction::Branch {
                    target: label.clone(),
                });
            }
            Statement::LineLabel { name, .. } => {
                self.emit(IrInstruction::Branch {
                    target: name.clone(),
                });
                let new_idx = self.create_block(name);
                self.switch_to_block(new_idx);
            }
            _ => {}
        }
    }

    pub fn lower_expression(&mut self, expr: &Expression) -> Operand {
        match expr {
            Expression::Literal(lit) => match lit {
                Literal::Integer(i) => Operand::Constant(IrConstant::Int(*i as i64, VbType::Long)),
                Literal::Double(d) => Operand::Constant(IrConstant::Float(*d)),
                Literal::String(s) => Operand::Constant(IrConstant::String(s.clone())),
                Literal::Boolean(b) => Operand::Constant(IrConstant::Bool(*b)),
                Literal::Date(d) => Operand::Constant(IrConstant::Date(d.clone())),
                Literal::Null => Operand::Constant(IrConstant::Null),
                Literal::Empty => Operand::Constant(IrConstant::Empty),
                Literal::Nothing => Operand::Constant(IrConstant::Nothing),
            },
            Expression::Variable(name) => {
                let temp = self.next_temp();
                self.emit(IrInstruction::Load {
                    dest: temp,
                    src: Operand::Local(name.clone()),
                });
                Operand::Temp(temp)
            }
            Expression::BinaryOp { op, left, right } => {
                let left_op = self.lower_expression(left);
                let right_op = self.lower_expression(right);
                let dest = self.next_temp();

                match op {
                    BinaryOperator::Concat => {
                        self.emit(IrInstruction::StringConcat {
                            dest,
                            left: left_op,
                            right: right_op,
                        });
                    }
                    BinaryOperator::Equal => {
                        self.emit(IrInstruction::Compare {
                            dest,
                            op: IrCompareOp::Equal,
                            left: left_op,
                            right: right_op,
                        });
                    }
                    BinaryOperator::NotEqual => {
                        self.emit(IrInstruction::Compare {
                            dest,
                            op: IrCompareOp::NotEqual,
                            left: left_op,
                            right: right_op,
                        });
                    }
                    BinaryOperator::LessThan => {
                        self.emit(IrInstruction::Compare {
                            dest,
                            op: IrCompareOp::LessThan,
                            left: left_op,
                            right: right_op,
                        });
                    }
                    BinaryOperator::LessThanOrEqual => {
                        self.emit(IrInstruction::Compare {
                            dest,
                            op: IrCompareOp::LessThanOrEqual,
                            left: left_op,
                            right: right_op,
                        });
                    }
                    BinaryOperator::GreaterThan => {
                        self.emit(IrInstruction::Compare {
                            dest,
                            op: IrCompareOp::GreaterThan,
                            left: left_op,
                            right: right_op,
                        });
                    }
                    BinaryOperator::GreaterThanOrEqual => {
                        self.emit(IrInstruction::Compare {
                            dest,
                            op: IrCompareOp::GreaterThanOrEqual,
                            left: left_op,
                            right: right_op,
                        });
                    }
                    BinaryOperator::Add => {
                        self.emit(IrInstruction::BinaryOp {
                            dest,
                            op: IrBinaryOp::Add,
                            left: left_op,
                            right: right_op,
                            result_type: VbType::Long,
                        });
                    }
                    BinaryOperator::Subtract => {
                        self.emit(IrInstruction::BinaryOp {
                            dest,
                            op: IrBinaryOp::Sub,
                            left: left_op,
                            right: right_op,
                            result_type: VbType::Long,
                        });
                    }
                    BinaryOperator::Multiply => {
                        self.emit(IrInstruction::BinaryOp {
                            dest,
                            op: IrBinaryOp::Mul,
                            left: left_op,
                            right: right_op,
                            result_type: VbType::Long,
                        });
                    }
                    BinaryOperator::Divide => {
                        self.emit(IrInstruction::BinaryOp {
                            dest,
                            op: IrBinaryOp::Div,
                            left: left_op,
                            right: right_op,
                            result_type: VbType::Double,
                        });
                    }
                    BinaryOperator::IntegerDivide => {
                        self.emit(IrInstruction::BinaryOp {
                            dest,
                            op: IrBinaryOp::IntDiv,
                            left: left_op,
                            right: right_op,
                            result_type: VbType::Long,
                        });
                    }
                    BinaryOperator::Modulo => {
                        self.emit(IrInstruction::BinaryOp {
                            dest,
                            op: IrBinaryOp::Mod,
                            left: left_op,
                            right: right_op,
                            result_type: VbType::Long,
                        });
                    }
                    BinaryOperator::And => {
                        self.emit(IrInstruction::BinaryOp {
                            dest,
                            op: IrBinaryOp::BitAnd,
                            left: left_op,
                            right: right_op,
                            result_type: VbType::Boolean,
                        });
                    }
                    BinaryOperator::Or => {
                        self.emit(IrInstruction::BinaryOp {
                            dest,
                            op: IrBinaryOp::BitOr,
                            left: left_op,
                            right: right_op,
                            result_type: VbType::Boolean,
                        });
                    }
                    BinaryOperator::Xor => {
                        self.emit(IrInstruction::BinaryOp {
                            dest,
                            op: IrBinaryOp::BitXor,
                            left: left_op,
                            right: right_op,
                            result_type: VbType::Boolean,
                        });
                    }
                    _ => {
                        self.emit(IrInstruction::BinaryOp {
                            dest,
                            op: IrBinaryOp::Add,
                            left: left_op,
                            right: right_op,
                            result_type: VbType::Variant,
                        });
                    }
                }

                Operand::Temp(dest)
            }
            Expression::UnaryOp { op, operand } => {
                let opnd = self.lower_expression(operand);
                let dest = self.next_temp();
                let ir_op = match op {
                    UnaryOperator::Negate => IrUnaryOp::Negate,
                    UnaryOperator::Not => IrUnaryOp::BitNot,
                    UnaryOperator::Plus => return opnd,
                };
                self.emit(IrInstruction::UnaryOp {
                    dest,
                    op: ir_op,
                    operand: opnd,
                    result_type: VbType::Long,
                });
                Operand::Temp(dest)
            }
            Expression::Paren(inner) => self.lower_expression(inner),
            _ => Operand::Constant(IrConstant::Null),
        }
    }
}
