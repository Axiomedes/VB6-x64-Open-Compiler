//! Top-level declarations in VB6 AST.

use crate::ast::span::Span;
use crate::ast::stmt::Statement;
use crate::ast::types::TypeSpec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Declaration {
    Sub(SubDeclaration),
    Function(FunctionDeclaration),
    Property(PropertyDeclaration),
    ModuleVariables(ModuleVariableDeclaration),
    Type(TypeDeclaration),
    Enum(EnumDeclaration),
    Class(ClassDeclaration),
    Event(EventDeclaration),
    Comment(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    Friend,
    Default,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubDeclaration {
    pub visibility: Visibility,
    pub name: String,
    pub params: Vec<Parameter>,
    pub body: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    pub visibility: Visibility,
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<TypeSpec>,
    pub body: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertyKind {
    Get,
    Let,
    Set,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyDeclaration {
    pub kind: PropertyKind,
    pub visibility: Visibility,
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<TypeSpec>,
    pub body: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub is_by_val: bool,
    pub param_type: Option<TypeSpec>,
    pub is_optional: bool,
    pub is_param_array: bool,
    pub default_value: Option<crate::ast::expr::Expression>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleVariableDeclaration {
    pub visibility: Visibility,
    pub is_const: bool,
    pub variables: Vec<crate::ast::stmt::VariableDeclarator>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeDeclaration {
    pub visibility: Visibility,
    pub name: String,
    pub members: Vec<TypeMember>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeMember {
    pub name: String,
    pub type_spec: TypeSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumDeclaration {
    pub visibility: Visibility,
    pub name: String,
    pub members: Vec<EnumMember>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumMember {
    pub name: String,
    pub value: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventDeclaration {
    pub visibility: Visibility,
    pub name: String,
    pub params: Vec<Parameter>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassDeclaration {
    pub name: String,
    pub visibility: Visibility,
    pub declarations: Vec<Declaration>,
    pub span: Span,
}
