//! Expressions and operators in VB6 AST.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    Literal(Literal),
    Variable(String),
    MemberAccess {
        object: Box<Expression>,
        member: String,
    },
    IndexAccess {
        target: Box<Expression>,
        indices: Vec<Expression>,
    },
    WithMemberAccess(String), // .Property inside With
    BinaryOp {
        op: BinaryOperator,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    UnaryOp {
        op: UnaryOperator,
        operand: Box<Expression>,
    },
    New(String),
    Me,
    Paren(Box<Expression>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    String(String),
    Integer(i32),
    Double(f64),
    Boolean(bool),
    Date(String),
    Null,
    Empty,
    Nothing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOperator {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    IntegerDivide, // \
    Modulo,        // Mod
    Exponent,      // ^
    // Concatenation
    Concat,        // &
    // Comparison
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Is,
    Like,
    // Logical / Bitwise
    And,
    Or,
    Xor,
    Eqv,
    Imp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOperator {
    Not,
    Negate,
    Plus,
}
