//! VB6 Open Compiler - Core Compiler Library

pub mod ast;
pub mod codegen;
pub mod controls;
pub mod ir;
pub mod lexer;
pub mod optimizer;
pub mod parser;
pub mod semantic;
pub mod symbols;
pub mod types;

use ast::SourceFile;
use lexer::Lexer;
use parser::Parser;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("Lexer error: {0}")]
    Lexer(#[from] lexer::LexerError),
    #[error("Parser error: {0}")]
    Parser(#[from] parser::ParserError),
}

/// Parse VB6 source code text into an Abstract Syntax Tree.
pub fn parse_source(source: &str) -> Result<SourceFile, CompileError> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let ast = parser.parse_file()?;
    Ok(ast)
}
