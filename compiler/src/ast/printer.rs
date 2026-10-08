//! ASTPrettyPrinter for rendering human-readable AST hierarchy trees.

use crate::ast::decl::*;
use crate::ast::stmt::*;
use crate::ast::SourceFile;

pub struct ASTPrinter {
    output: String,
    indent: usize,
}

impl ASTPrinter {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
        }
    }

    pub fn print(file: &SourceFile) -> String {
        let mut printer = Self::new();
        printer.print_file(file);
        printer.output
    }

    fn line(&mut self, text: &str) {
        for _ in 0..self.indent {
            self.output.push_str("  ");
        }
        self.output.push_str(text);
        self.output.push('\n');
    }

    pub fn print_file(&mut self, file: &SourceFile) {
        self.line("Program");
        self.indent += 1;
        for decl in &file.declarations {
            self.print_declaration(decl);
        }
        self.indent -= 1;
    }

    fn print_declaration(&mut self, decl: &Declaration) {
        match decl {
            Declaration::Sub(s) => {
                self.line(&format!("SubDeclaration: {}", s.name));
                self.indent += 1;
                for stmt in &s.body {
                    self.print_statement(stmt);
                }
                self.indent -= 1;
            }
            Declaration::Function(f) => {
                self.line(&format!("FunctionDeclaration: {}", f.name));
                self.indent += 1;
                for stmt in &f.body {
                    self.print_statement(stmt);
                }
                self.indent -= 1;
            }
            Declaration::Property(p) => {
                self.line(&format!("PropertyDeclaration: {:?} {}", p.kind, p.name));
                self.indent += 1;
                for stmt in &p.body {
                    self.print_statement(stmt);
                }
                self.indent -= 1;
            }
            Declaration::ModuleVariables(v) => {
                self.line("ModuleVariableDeclaration");
                self.indent += 1;
                for var in &v.variables {
                    self.line(&format!("Dim {}", var.name));
                }
                self.indent -= 1;
            }
            Declaration::Comment(c) => {
                self.line(&format!("Comment: '{}'", c));
            }
            _ => {
                self.line("Declaration");
            }
        }
    }

    fn print_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration { kind, variables, .. } => {
                let vars_str: Vec<String> = variables
                    .iter()
                    .map(|v| {
                        if let Some(ts) = &v.type_spec {
                            format!("{} As {}", v.name, ts.name)
                        } else {
                            v.name.clone()
                        }
                    })
                    .collect();
                self.line(&format!("VariableDeclaration: {:?} {}", kind, vars_str.join(", ")));
            }
            Statement::Assignment { target, value, is_set, .. } => {
                let prefix = if *is_set { "Set " } else { "" };
                self.line(&format!("Assignment: {}{:?} = {:?}", prefix, target, value));
            }
            Statement::If { branches, else_branch, .. } => {
                self.line("IfStatement");
                self.indent += 1;
                for (i, b) in branches.iter().enumerate() {
                    let branch_name = if i == 0 { "If Condition" } else { "ElseIf Condition" };
                    self.line(&format!("{}: {:?}", branch_name, b.condition));
                    self.indent += 1;
                    for s in &b.body {
                        self.print_statement(s);
                    }
                    self.indent -= 1;
                }
                if let Some(el) = else_branch {
                    self.line("Else");
                    self.indent += 1;
                    for s in el {
                        self.print_statement(s);
                    }
                    self.indent -= 1;
                }
                self.indent -= 1;
            }
            Statement::For { counter, start, end, body, .. } => {
                self.line(&format!("ForStatement: {} = {:?} To {:?}", counter, start, end));
                self.indent += 1;
                for s in body {
                    self.print_statement(s);
                }
                self.indent -= 1;
            }
            Statement::Call { name, arguments, has_call_keyword, .. } => {
                self.line(&format!("CallStatement: {} (Call kw: {}) with {} args", name, has_call_keyword, arguments.len()));
            }
            Statement::DebugPrint { arguments, .. } => {
                self.line(&format!("DebugPrint: {} args", arguments.len()));
            }
            Statement::Comment(c) => {
                self.line(&format!("Comment: '{}'", c));
            }
            _ => {
                self.line("Statement");
            }
        }
    }
}
