//! Executable statements in VB6 AST.

use crate::ast::expr::Expression;
use crate::ast::span::Span;
use crate::ast::types::TypeSpec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    /// Variable or constant declaration: `Dim X As Long, Y As String` or `Const PI = 3.14`
    VariableDeclaration {
        kind: VarDeclKind,
        variables: Vec<VariableDeclarator>,
        span: Span,
    },
    /// Value assignment: `X = 10` or `Set Obj = Nothing`
    Assignment {
        target: Expression,
        value: Expression,
        is_set: bool,
        span: Span,
    },
    /// If ... Then ... ElseIf ... Else ... End If
    If {
        branches: Vec<IfBranch>,
        else_branch: Option<Vec<Statement>>,
        span: Span,
    },
    /// Select Case Expr ... Case ... End Select
    SelectCase {
        expression: Expression,
        cases: Vec<CaseClause>,
        else_case: Option<Vec<Statement>>,
        span: Span,
    },
    /// For I = 1 To 10 Step 2 ... Next I
    For {
        counter: String,
        start: Expression,
        end: Expression,
        step: Option<Expression>,
        body: Vec<Statement>,
        span: Span,
    },
    /// For Each Item In Collection ... Next Item
    ForEach {
        element: String,
        collection: Expression,
        body: Vec<Statement>,
        span: Span,
    },
    /// Do While/Until ... Loop or Do ... Loop While/Until
    DoLoop {
        condition: Option<LoopCondition>,
        condition_at_start: bool,
        body: Vec<Statement>,
        span: Span,
    },
    /// While Cond ... Wend
    WhileWend {
        condition: Expression,
        body: Vec<Statement>,
        span: Span,
    },
    /// With Target ... End With
    With {
        target: Expression,
        body: Vec<Statement>,
        span: Span,
    },
    /// Procedure call: `MsgBox "Hello"` or `Call MySub(arg)`
    Call {
        name: String,
        arguments: Vec<Expression>,
        has_call_keyword: bool,
        span: Span,
    },
    /// Debug.Print Expr1, Expr2; Expr3
    DebugPrint {
        arguments: Vec<Expression>,
        span: Span,
    },
    /// On Error GoTo Label / On Error Resume Next / On Error GoTo 0
    OnError {
        action: OnErrorAction,
        span: Span,
    },
    /// Resume / Resume Next / Resume Label
    Resume {
        action: ResumeAction,
        span: Span,
    },
    /// Exit Sub / Exit Function / Exit For / Exit Do
    Exit {
        target: ExitTarget,
        span: Span,
    },
    /// GoTo Label
    GoTo {
        label: String,
        span: Span,
    },
    /// Line label or line number: `ErrorHandler:` or `100:`
    LineLabel {
        name: String,
        span: Span,
    },
    /// Raw comment inside procedure body
    Comment(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VarDeclKind {
    Dim,
    Static,
    Const,
    ReDim,
    ReDimPreserve,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrayDimExpr {
    pub lower: Option<Expression>,
    pub upper: Expression,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariableDeclarator {
    pub name: String,
    pub dimensions: Option<Vec<ArrayDimExpr>>,
    pub type_spec: Option<TypeSpec>,
    pub init_value: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IfBranch {
    pub condition: Expression,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaseClause {
    pub values: Vec<CaseValue>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CaseValue {
    Exact(Expression),
    Range(Expression, Expression), // Case 1 To 10
    Comparison(crate::ast::expr::BinaryOperator, Expression), // Case Is > 5
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LoopCondition {
    While(Expression),
    Until(Expression),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnErrorAction {
    GoTo(String),
    ResumeNext,
    GoToZero,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResumeAction {
    Current,
    Next,
    Label(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExitTarget {
    Sub,
    Function,
    Property,
    For,
    Do,
}
