//! Visitor pattern for traversing and analyzing VB6 AST nodes.

use crate::ast::decl::*;
use crate::ast::expr::*;
use crate::ast::stmt::*;
use crate::ast::SourceFile;

pub trait ASTVisitor {
    fn visit_source_file(&mut self, file: &SourceFile) {
        for decl in &file.declarations {
            self.visit_declaration(decl);
        }
    }

    fn visit_declaration(&mut self, decl: &Declaration) {
        match decl {
            Declaration::Sub(s) => self.visit_sub_decl(s),
            Declaration::Function(f) => self.visit_func_decl(f),
            Declaration::Property(p) => self.visit_prop_decl(p),
            Declaration::ModuleVariables(v) => self.visit_module_vars(v),
            Declaration::Type(t) => self.visit_type_decl(t),
            Declaration::Enum(e) => self.visit_enum_decl(e),
            Declaration::Class(c) => self.visit_class_decl(c),
            Declaration::Event(ev) => self.visit_event_decl(ev),
            Declaration::Comment(_) => {}
        }
    }

    fn visit_class_decl(&mut self, class: &ClassDeclaration) {
        for decl in &class.declarations {
            self.visit_declaration(decl);
        }
    }

    fn visit_sub_decl(&mut self, sub: &SubDeclaration) {
        for stmt in &sub.body {
            self.visit_statement(stmt);
        }
    }

    fn visit_func_decl(&mut self, func: &FunctionDeclaration) {
        for stmt in &func.body {
            self.visit_statement(stmt);
        }
    }

    fn visit_prop_decl(&mut self, prop: &PropertyDeclaration) {
        for stmt in &prop.body {
            self.visit_statement(stmt);
        }
    }

    fn visit_module_vars(&mut self, _vars: &ModuleVariableDeclaration) {}
    fn visit_type_decl(&mut self, _ty: &TypeDeclaration) {}
    fn visit_enum_decl(&mut self, _en: &EnumDeclaration) {}
    fn visit_event_decl(&mut self, _ev: &EventDeclaration) {}

    fn visit_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration { variables, .. } => {
                for v in variables {
                    if let Some(init) = &v.init_value {
                        self.visit_expression(init);
                    }
                }
            }
            Statement::Assignment { target, value, .. } => {
                self.visit_expression(target);
                self.visit_expression(value);
            }
            Statement::If {
                branches,
                else_branch,
                ..
            } => {
                for b in branches {
                    self.visit_expression(&b.condition);
                    for s in &b.body {
                        self.visit_statement(s);
                    }
                }
                if let Some(else_stmts) = else_branch {
                    for s in else_stmts {
                        self.visit_statement(s);
                    }
                }
            }
            Statement::SelectCase {
                expression,
                cases,
                else_case,
                ..
            } => {
                self.visit_expression(expression);
                for c in cases {
                    for v in &c.values {
                        match v {
                            CaseValue::Exact(e) => self.visit_expression(e),
                            CaseValue::Range(s, e) => {
                                self.visit_expression(s);
                                self.visit_expression(e);
                            }
                            CaseValue::Comparison(_, e) => self.visit_expression(e),
                        }
                    }
                    for s in &c.body {
                        self.visit_statement(s);
                    }
                }
                if let Some(el) = else_case {
                    for s in el {
                        self.visit_statement(s);
                    }
                }
            }
            Statement::For {
                start, end, step, body, ..
            } => {
                self.visit_expression(start);
                self.visit_expression(end);
                if let Some(st) = step {
                    self.visit_expression(st);
                }
                for s in body {
                    self.visit_statement(s);
                }
            }
            Statement::ForEach { collection, body, .. } => {
                self.visit_expression(collection);
                for s in body {
                    self.visit_statement(s);
                }
            }
            Statement::DoLoop { condition, body, .. } => {
                if let Some(cond) = condition {
                    match cond {
                        LoopCondition::While(e) | LoopCondition::Until(e) => {
                            self.visit_expression(e)
                        }
                    }
                }
                for s in body {
                    self.visit_statement(s);
                }
            }
            Statement::WhileWend { condition, body, .. } => {
                self.visit_expression(condition);
                for s in body {
                    self.visit_statement(s);
                }
            }
            Statement::With { target, body, .. } => {
                self.visit_expression(target);
                for s in body {
                    self.visit_statement(s);
                }
            }
            Statement::Call { arguments, .. } => {
                for a in arguments {
                    self.visit_expression(a);
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
        match expr {
            Expression::MemberAccess { object, .. } => self.visit_expression(object),
            Expression::IndexAccess { target, indices } => {
                self.visit_expression(target);
                for idx in indices {
                    self.visit_expression(idx);
                }
            }
            Expression::BinaryOp { left, right, .. } => {
                self.visit_expression(left);
                self.visit_expression(right);
            }
            Expression::UnaryOp { operand, .. } => self.visit_expression(operand),
            Expression::Paren(inner) => self.visit_expression(inner),
            _ => {}
        }
    }
}
