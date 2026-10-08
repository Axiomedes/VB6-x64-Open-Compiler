//! Semantic Analysis, Type Resolution, and Validation for VB6 AST.

use crate::ast::decl::*;
use crate::ast::expr::*;
use crate::ast::span::Span;
use crate::ast::stmt::*;
use crate::ast::SourceFile;
use crate::symbols::*;
use crate::types::VbType;
use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq)]
pub enum SemanticError {
    #[error("Type mismatch: cannot assign/coerce '{found:?}' to expected type '{expected:?}' at line {}, col {}", span.start_line, span.start_col)]
    TypeMismatch {
        expected: VbType,
        found: VbType,
        span: Span,
    },
    #[error("Variable or identifier '{name}' is not declared at line {}, col {}", span.start_line, span.start_col)]
    UndeclaredIdentifier { name: String, span: Span },
    #[error("Duplicate declaration of '{name}' at line {}, col {}", duplicate_span.start_line, duplicate_span.start_col)]
    DuplicateSymbol {
        name: String,
        original_span: Span,
        duplicate_span: Span,
    },
    #[error("Argument count mismatch in call to '{proc_name}': expected {expected}, found {found} at line {}, col {}", span.start_line, span.start_col)]
    ArgumentCountMismatch {
        proc_name: String,
        expected: usize,
        found: usize,
        span: Span,
    },
    #[error("Invalid assignment target at line {}, col {}", span.start_line, span.start_col)]
    InvalidAssignmentTarget { span: Span },
}

pub struct SemanticAnalyzer {
    pub symbol_table: SymbolTable,
    pub errors: Vec<SemanticError>,
    pub option_explicit: bool,
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {
            symbol_table: SymbolTable::new(),
            errors: Vec::new(),
            option_explicit: false,
        }
    }

    pub fn analyze_file(&mut self, file: &SourceFile) -> Result<(), Vec<SemanticError>> {
        self.symbol_table.enter_scope(ScopeKind::Module);

        // First pass: register all procedure headers and module-level symbols
        for decl in &file.declarations {
            self.declare_symbol(decl);
        }

        // Second pass: analyze bodies, statements, and expressions
        for decl in &file.declarations {
            self.analyze_declaration(decl);
        }

        self.symbol_table.exit_scope();

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn declare_symbol(&mut self, decl: &Declaration) {
        match decl {
            Declaration::Sub(s) => {
                let params = s
                    .params
                    .iter()
                    .map(|p| {
                        let ty = p
                            .param_type
                            .as_ref()
                            .map(|t| VbType::from_name(&t.name))
                            .unwrap_or(VbType::Variant);
                        (p.name.clone(), ty)
                    })
                    .collect();

                let sym = Symbol {
                    name: s.name.clone(),
                    kind: SymbolKind::Procedure {
                        kind: ProcKind::Sub,
                        params,
                        return_type: None,
                    },
                    vb_type: VbType::Unknown,
                    visibility: s.visibility,
                    span: s.span,
                };
                if let Err(orig) = self.symbol_table.define(sym) {
                    self.errors.push(SemanticError::DuplicateSymbol {
                        name: s.name.clone(),
                        original_span: orig.span,
                        duplicate_span: s.span,
                    });
                }
            }
            Declaration::Function(f) => {
                let params = f
                    .params
                    .iter()
                    .map(|p| {
                        let ty = p
                            .param_type
                            .as_ref()
                            .map(|t| VbType::from_name(&t.name))
                            .unwrap_or(VbType::Variant);
                        (p.name.clone(), ty)
                    })
                    .collect();

                let ret_ty = f
                    .return_type
                    .as_ref()
                    .map(|t| VbType::from_name(&t.name))
                    .unwrap_or(VbType::Variant);

                let sym = Symbol {
                    name: f.name.clone(),
                    kind: SymbolKind::Procedure {
                        kind: ProcKind::Function,
                        params,
                        return_type: Some(ret_ty.clone()),
                    },
                    vb_type: ret_ty,
                    visibility: f.visibility,
                    span: f.span,
                };
                if let Err(orig) = self.symbol_table.define(sym) {
                    self.errors.push(SemanticError::DuplicateSymbol {
                        name: f.name.clone(),
                        original_span: orig.span,
                        duplicate_span: f.span,
                    });
                }
            }
            Declaration::ModuleVariables(v) => {
                for var in &v.variables {
                    let vb_type = var
                        .type_spec
                        .as_ref()
                        .map(|t| VbType::from_name(&t.name))
                        .unwrap_or(VbType::Variant);

                    let sym = Symbol {
                        name: var.name.clone(),
                        kind: SymbolKind::Variable {
                            is_const: v.is_const,
                            is_static: false,
                        },
                        vb_type,
                        visibility: v.visibility,
                        span: v.span,
                    };
                    if let Err(orig) = self.symbol_table.define(sym) {
                        self.errors.push(SemanticError::DuplicateSymbol {
                            name: var.name.clone(),
                            original_span: orig.span,
                            duplicate_span: v.span,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    fn analyze_declaration(&mut self, decl: &Declaration) {
        match decl {
            Declaration::Sub(s) => {
                self.symbol_table.enter_scope(ScopeKind::Procedure);
                // Define parameters in procedure scope
                for p in &s.params {
                    let vb_type = p
                        .param_type
                        .as_ref()
                        .map(|t| VbType::from_name(&t.name))
                        .unwrap_or(VbType::Variant);
                    let sym = Symbol {
                        name: p.name.clone(),
                        kind: SymbolKind::Parameter {
                            is_by_val: p.is_by_val,
                            is_optional: p.is_optional,
                        },
                        vb_type,
                        visibility: Visibility::Default,
                        span: s.span,
                    };
                    let _ = self.symbol_table.define(sym);
                }

                for stmt in &s.body {
                    self.analyze_statement(stmt);
                }
                self.symbol_table.exit_scope();
            }
            Declaration::Function(f) => {
                self.symbol_table.enter_scope(ScopeKind::Procedure);
                for p in &f.params {
                    let vb_type = p
                        .param_type
                        .as_ref()
                        .map(|t| VbType::from_name(&t.name))
                        .unwrap_or(VbType::Variant);
                    let sym = Symbol {
                        name: p.name.clone(),
                        kind: SymbolKind::Parameter {
                            is_by_val: p.is_by_val,
                            is_optional: p.is_optional,
                        },
                        vb_type,
                        visibility: Visibility::Default,
                        span: f.span,
                    };
                    let _ = self.symbol_table.define(sym);
                }

                for stmt in &f.body {
                    self.analyze_statement(stmt);
                }
                self.symbol_table.exit_scope();
            }
            _ => {}
        }
    }

    fn analyze_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration {
                kind,
                variables,
                span,
            } => {
                for v in variables {
                    let vb_type = v
                        .type_spec
                        .as_ref()
                        .map(|t| VbType::from_name(&t.name))
                        .unwrap_or(VbType::Variant);

                    let sym = Symbol {
                        name: v.name.clone(),
                        kind: SymbolKind::Variable {
                            is_const: *kind == VarDeclKind::Const,
                            is_static: *kind == VarDeclKind::Static,
                        },
                        vb_type,
                        visibility: Visibility::Default,
                        span: *span,
                    };
                    if let Err(orig) = self.symbol_table.define(sym) {
                        self.errors.push(SemanticError::DuplicateSymbol {
                            name: v.name.clone(),
                            original_span: orig.span,
                            duplicate_span: *span,
                        });
                    }

                    if let Some(init) = &v.init_value {
                        let init_ty = self.infer_expression_type(init);
                        if let Some(ts) = &v.type_spec {
                            let expected = VbType::from_name(&ts.name);
                            if !init_ty.can_coerce_to(&expected) {
                                self.errors.push(SemanticError::TypeMismatch {
                                    expected,
                                    found: init_ty,
                                    span: *span,
                                });
                            }
                        }
                    }
                }
            }
            Statement::Assignment {
                target,
                value,
                span,
                ..
            } => {
                let target_ty = self.infer_expression_type(target);
                let value_ty = self.infer_expression_type(value);

                if target_ty != VbType::Unknown
                    && value_ty != VbType::Unknown
                    && !value_ty.can_coerce_to(&target_ty)
                {
                    self.errors.push(SemanticError::TypeMismatch {
                        expected: target_ty,
                        found: value_ty,
                        span: *span,
                    });
                }
            }
            Statement::If {
                branches,
                else_branch,
                ..
            } => {
                for b in branches {
                    let _ = self.infer_expression_type(&b.condition);
                    for s in &b.body {
                        self.analyze_statement(s);
                    }
                }
                if let Some(el) = else_branch {
                    for s in el {
                        self.analyze_statement(s);
                    }
                }
            }
            Statement::For {
                counter,
                start,
                end,
                step,
                body,
                span,
            } => {
                if let Some(sym) = self.symbol_table.lookup(counter) {
                    if !sym.vb_type.is_numeric() && sym.vb_type != VbType::Variant {
                        self.errors.push(SemanticError::TypeMismatch {
                            expected: VbType::Long,
                            found: sym.vb_type.clone(),
                            span: *span,
                        });
                    }
                } else if self.option_explicit {
                    self.errors.push(SemanticError::UndeclaredIdentifier {
                        name: counter.clone(),
                        span: *span,
                    });
                }

                let _ = self.infer_expression_type(start);
                let _ = self.infer_expression_type(end);
                if let Some(st) = step {
                    let _ = self.infer_expression_type(st);
                }

                for s in body {
                    self.analyze_statement(s);
                }
            }
            Statement::Call {
                name,
                arguments,
                span,
                ..
            } => {
                if let Some(sym) = self.symbol_table.lookup(name) {
                    if let SymbolKind::Procedure { params, .. } = &sym.kind {
                        if arguments.len() != params.len() {
                            self.errors.push(SemanticError::ArgumentCountMismatch {
                                proc_name: name.clone(),
                                expected: params.len(),
                                found: arguments.len(),
                                span: *span,
                            });
                        }
                    }
                } else if self.option_explicit {
                    self.errors.push(SemanticError::UndeclaredIdentifier {
                        name: name.clone(),
                        span: *span,
                    });
                }

                for arg in arguments {
                    let _ = self.infer_expression_type(arg);
                }
            }
            Statement::DebugPrint { arguments, .. } => {
                for arg in arguments {
                    let _ = self.infer_expression_type(arg);
                }
            }
            _ => {}
        }
    }

    pub fn infer_expression_type(&mut self, expr: &Expression) -> VbType {
        match expr {
            Expression::Literal(lit) => match lit {
                Literal::Integer(_) => VbType::Integer,
                Literal::Double(_) => VbType::Double,
                Literal::String(_) => VbType::String,
                Literal::Boolean(_) => VbType::Boolean,
                Literal::Date(_) => VbType::Date,
                Literal::Null | Literal::Empty => VbType::Variant,
                Literal::Nothing => VbType::Object,
            },
            Expression::Variable(name) => {
                if let Some(sym) = self.symbol_table.lookup(name) {
                    sym.vb_type.clone()
                } else {
                    if self.option_explicit {
                        self.errors.push(SemanticError::UndeclaredIdentifier {
                            name: name.clone(),
                            span: Span::default(),
                        });
                        VbType::Unknown
                    } else {
                        // In classic VB6 without Option Explicit, undeclared variables default to Variant!
                        VbType::Variant
                    }
                }
            }
            Expression::BinaryOp { op, left, right } => {
                let left_ty = self.infer_expression_type(left);
                let right_ty = self.infer_expression_type(right);

                match op {
                    BinaryOperator::Concat => VbType::String,
                    BinaryOperator::Equal
                    | BinaryOperator::NotEqual
                    | BinaryOperator::LessThan
                    | BinaryOperator::LessThanOrEqual
                    | BinaryOperator::GreaterThan
                    | BinaryOperator::GreaterThanOrEqual
                    | BinaryOperator::Is
                    | BinaryOperator::Like => VbType::Boolean,
                    BinaryOperator::Divide => VbType::Double, // VB6 '/' always produces Double
                    BinaryOperator::IntegerDivide => VbType::Long,
                    BinaryOperator::Modulo => VbType::Long,
                    BinaryOperator::Add => {
                        if left_ty == VbType::String && right_ty == VbType::String {
                            VbType::String
                        } else if left_ty == VbType::Double || right_ty == VbType::Double {
                            VbType::Double
                        } else if left_ty == VbType::Single || right_ty == VbType::Single {
                            VbType::Single
                        } else if left_ty == VbType::Long || right_ty == VbType::Long {
                            VbType::Long
                        } else if left_ty.is_integer() && right_ty.is_integer() {
                            VbType::Integer
                        } else {
                            VbType::Variant
                        }
                    }
                    BinaryOperator::Subtract | BinaryOperator::Multiply => {
                        if left_ty == VbType::Double || right_ty == VbType::Double {
                            VbType::Double
                        } else if left_ty == VbType::Single || right_ty == VbType::Single {
                            VbType::Single
                        } else if left_ty == VbType::Long || right_ty == VbType::Long {
                            VbType::Long
                        } else {
                            VbType::Integer
                        }
                    }
                    BinaryOperator::And
                    | BinaryOperator::Or
                    | BinaryOperator::Xor
                    | BinaryOperator::Eqv
                    | BinaryOperator::Imp => {
                        if left_ty == VbType::Boolean && right_ty == VbType::Boolean {
                            VbType::Boolean
                        } else if left_ty.is_integer() && right_ty.is_integer() {
                            VbType::Long
                        } else {
                            VbType::Variant
                        }
                    }
                    BinaryOperator::Exponent => VbType::Double,
                }
            }
            Expression::UnaryOp { op, operand } => {
                let ty = self.infer_expression_type(operand);
                match op {
                    UnaryOperator::Not => {
                        if ty == VbType::Boolean {
                            VbType::Boolean
                        } else {
                            ty
                        }
                    }
                    UnaryOperator::Negate | UnaryOperator::Plus => ty,
                }
            }
            Expression::Paren(inner) => self.infer_expression_type(inner),
            _ => VbType::Variant,
        }
    }
}
