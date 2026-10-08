//! Abstract Syntax Tree (AST) definitions for Visual Basic 6.0.

pub mod decl;
pub mod expr;
pub mod printer;
pub mod span;
pub mod stmt;
pub mod types;
pub mod visitor;

pub use decl::*;
pub use expr::*;
pub use printer::ASTPrinter;
pub use span::Span;
pub use stmt::*;
pub use types::*;
pub use visitor::ASTVisitor;

use serde::{Deserialize, Serialize};

/// A parsed VB6 module or file (representing .bas, .cls, or code-behind .frm).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceFile {
    pub declarations: Vec<Declaration>,
}
