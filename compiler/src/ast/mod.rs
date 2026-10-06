//! Abstract Syntax Tree (AST) definitions for Visual Basic 6.0.

use serde::{Deserialize, Serialize};

/// Source code location span (1-indexed for lines and columns).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

impl Span {
    pub fn new(start_line: usize, start_col: usize, end_line: usize, end_col: usize) -> Self {
        Self {
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }
}

/// A parsed VB6 module or file (can represent .bas, .cls, or code-behind .frm).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceFile {
    pub declarations: Vec<Declaration>,
}

/// Top-level declarations in a VB6 source file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Declaration {
    Sub(SubDeclaration),
    Function(FunctionDeclaration),
    Comment(String),
}

/// Procedure visibility / scope in VB6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    Friend,
    Default,
}

/// Subroutine declaration (`Sub Name(...) ... End Sub`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubDeclaration {
    pub visibility: Visibility,
    pub name: String,
    pub params: Vec<Parameter>,
    pub body: Vec<Statement>,
    pub span: Span,
}

/// Function declaration (`Function Name(...) As Type ... End Function`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    pub visibility: Visibility,
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<String>,
    pub body: Vec<Statement>,
    pub span: Span,
}

/// Parameter in a procedure declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub is_by_val: bool,
    pub param_type: Option<String>,
    pub is_optional: bool,
}

/// VB6 executable statements within procedure bodies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    /// Procedure or sub call: `MsgBox "Hello VB6"` or `Call MsgBox("Hello VB6")`
    Call {
        name: String,
        arguments: Vec<Expression>,
        has_call_keyword: bool,
        span: Span,
    },
    /// Assignment: `X = 10` or `Set X = Nothing`
    Assignment {
        target: String,
        value: Expression,
        is_set: bool,
        span: Span,
    },
    /// Raw comment within a procedure body
    Comment(String),
}

/// Expressions in VB6 code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    Literal(Literal),
    Variable(String),
    BinaryOp {
        op: BinaryOperator,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    UnaryOp {
        op: UnaryOperator,
        operand: Box<Expression>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    String(String),
    Integer(i32),
    Double(f64),
    Boolean(bool),
    Null,
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Concat, // & operator
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOperator {
    Not,
    Negate,
}
