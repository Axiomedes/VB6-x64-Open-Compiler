//! Parser for Visual Basic 6.0 source code.

use crate::ast::*;
use crate::lexer::{Token, TokenKind};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ParserError {
    #[error("Unexpected token {0:?} at line {1}, col {2}. Expected {3}")]
    UnexpectedToken(TokenKind, usize, usize, &'static str),
    #[error("Unexpected end of file while parsing {0}")]
    UnexpectedEof(&'static str),
    #[error("Procedure '{0}' was not closed with 'End Sub' or 'End Function'")]
    UnclosedProcedure(String),
}

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        &self.tokens[self.current - 1]
    }

    fn check(&self, kind: &TokenKind) -> bool {
        if self.is_at_end() {
            false
        } else {
            &self.peek().kind == kind
        }
    }

    fn skip_newlines_and_colons(&mut self) {
        while matches!(self.peek().kind, TokenKind::Newline | TokenKind::Colon) {
            self.advance();
        }
    }

    /// Parse a complete VB6 source file / module.
    pub fn parse_file(&mut self) -> Result<SourceFile, ParserError> {
        let mut declarations = Vec::new();

        self.skip_newlines_and_colons();

        while !self.is_at_end() {
            if let Some(decl) = self.parse_declaration()? {
                declarations.push(decl);
            }
            self.skip_newlines_and_colons();
        }

        Ok(SourceFile { declarations })
    }

    fn parse_declaration(&mut self) -> Result<Option<Declaration>, ParserError> {
        let token = self.peek().clone();

        match &token.kind {
            TokenKind::Comment(text) => {
                self.advance();
                Ok(Some(Declaration::Comment(text.clone())))
            }
            TokenKind::Public | TokenKind::Private | TokenKind::Friend => {
                let vis = match token.kind {
                    TokenKind::Public => Visibility::Public,
                    TokenKind::Private => Visibility::Private,
                    TokenKind::Friend => Visibility::Friend,
                    _ => Visibility::Default,
                };
                self.advance();
                self.parse_procedure_with_visibility(vis)
            }
            TokenKind::Sub | TokenKind::Function => {
                self.parse_procedure_with_visibility(Visibility::Default)
            }
            _ => {
                let span = token.span;
                Err(ParserError::UnexpectedToken(
                    token.kind,
                    span.start_line,
                    span.start_col,
                    "declaration (Sub, Function, etc.)",
                ))
            }
        }
    }

    fn parse_procedure_with_visibility(
        &mut self,
        visibility: Visibility,
    ) -> Result<Option<Declaration>, ParserError> {
        let proc_token = self.peek().clone();
        match proc_token.kind {
            TokenKind::Sub => {
                self.advance();
                let sub_decl = self.parse_sub_declaration(visibility, proc_token.span)?;
                Ok(Some(Declaration::Sub(sub_decl)))
            }
            TokenKind::Function => {
                self.advance();
                let func_decl = self.parse_func_declaration(visibility, proc_token.span)?;
                Ok(Some(Declaration::Function(func_decl)))
            }
            _ => {
                let span = proc_token.span;
                Err(ParserError::UnexpectedToken(
                    proc_token.kind,
                    span.start_line,
                    span.start_col,
                    "'Sub' or 'Function'",
                ))
            }
        }
    }

    fn parse_sub_declaration(
        &mut self,
        visibility: Visibility,
        start_span: Span,
    ) -> Result<SubDeclaration, ParserError> {
        let name_token = self.peek().clone();
        let name = match name_token.kind {
            TokenKind::Identifier(ref id) => {
                self.advance();
                id.clone()
            }
            _ => {
                let span = name_token.span;
                return Err(ParserError::UnexpectedToken(
                    name_token.kind,
                    span.start_line,
                    span.start_col,
                    "subroutine name identifier",
                ));
            }
        };

        // Parse optional parameter list: `(params)`
        let params = self.parse_parameter_list()?;

        // Expect end of line or colon after sub header
        self.expect_statement_terminator("Sub header")?;

        // Parse procedure body
        let mut body = Vec::new();
        let mut closed = false;
        let mut end_span = start_span.clone();

        while !self.is_at_end() {
            self.skip_newlines_and_colons();
            if self.is_at_end() {
                break;
            }

            // Check for `End Sub`
            if self.check(&TokenKind::End) {
                let end_token = self.advance().clone();
                if self.check(&TokenKind::Sub) {
                    let sub_end = self.advance().clone();
                    closed = true;
                    end_span = sub_end.span;
                    self.skip_newlines_and_colons();
                    break;
                } else {
                    return Err(ParserError::UnexpectedToken(
                        self.peek().kind.clone(),
                        end_token.span.start_line,
                        end_token.span.start_col,
                        "'Sub' after 'End'",
                    ));
                }
            }

            let stmt = self.parse_statement()?;
            body.push(stmt);
        }

        if !closed {
            return Err(ParserError::UnclosedProcedure(name));
        }

        Ok(SubDeclaration {
            visibility,
            name,
            params,
            body,
            span: Span::new(
                start_span.start_line,
                start_span.start_col,
                end_span.end_line,
                end_span.end_col,
            ),
        })
    }

    fn parse_func_declaration(
        &mut self,
        visibility: Visibility,
        start_span: Span,
    ) -> Result<FunctionDeclaration, ParserError> {
        let name_token = self.peek().clone();
        let name = match name_token.kind {
            TokenKind::Identifier(ref id) => {
                self.advance();
                id.clone()
            }
            _ => {
                let span = name_token.span;
                return Err(ParserError::UnexpectedToken(
                    name_token.kind,
                    span.start_line,
                    span.start_col,
                    "function name identifier",
                ));
            }
        };

        let params = self.parse_parameter_list()?;

        // Optional return type `As Type`
        let return_type = if self.check(&TokenKind::As) {
            self.advance();
            let type_token = self.peek().clone();
            if let TokenKind::Identifier(ref tid) = type_token.kind {
                self.advance();
                Some(tid.clone())
            } else {
                return Err(ParserError::UnexpectedToken(
                    type_token.kind,
                    type_token.span.start_line,
                    type_token.span.start_col,
                    "type name identifier",
                ));
            }
        } else {
            None
        };

        self.expect_statement_terminator("Function header")?;

        let mut body = Vec::new();
        let mut closed = false;
        let mut end_span = start_span.clone();

        while !self.is_at_end() {
            self.skip_newlines_and_colons();
            if self.is_at_end() {
                break;
            }

            if self.check(&TokenKind::End) {
                let end_token = self.advance().clone();
                if self.check(&TokenKind::Function) {
                    let func_end = self.advance().clone();
                    closed = true;
                    end_span = func_end.span;
                    self.skip_newlines_and_colons();
                    break;
                } else {
                    return Err(ParserError::UnexpectedToken(
                        self.peek().kind.clone(),
                        end_token.span.start_line,
                        end_token.span.start_col,
                        "'Function' after 'End'",
                    ));
                }
            }

            let stmt = self.parse_statement()?;
            body.push(stmt);
        }

        if !closed {
            return Err(ParserError::UnclosedProcedure(name));
        }

        Ok(FunctionDeclaration {
            visibility,
            name,
            params,
            return_type,
            body,
            span: Span::new(
                start_span.start_line,
                start_span.start_col,
                end_span.end_line,
                end_span.end_col,
            ),
        })
    }

    fn parse_parameter_list(&mut self) -> Result<Vec<Parameter>, ParserError> {
        let mut params = Vec::new();
        if self.check(&TokenKind::LParen) {
            self.advance(); // consume '('
            while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                let is_optional = if self.check(&TokenKind::Optional) {
                    self.advance();
                    true
                } else {
                    false
                };

                let is_by_val = if self.check(&TokenKind::ByVal) {
                    self.advance();
                    true
                } else if self.check(&TokenKind::ByRef) {
                    self.advance();
                    false
                } else {
                    // Default in VB6 is ByRef!
                    false
                };

                let name_token = self.peek().clone();
                let param_name = match name_token.kind {
                    TokenKind::Identifier(ref id) => {
                        self.advance();
                        id.clone()
                    }
                    _ => {
                        return Err(ParserError::UnexpectedToken(
                            name_token.kind,
                            name_token.span.start_line,
                            name_token.span.start_col,
                            "parameter name",
                        ));
                    }
                };

                let param_type = if self.check(&TokenKind::As) {
                    self.advance();
                    let t_token = self.peek().clone();
                    if let TokenKind::Identifier(ref tid) = t_token.kind {
                        self.advance();
                        Some(tid.clone())
                    } else {
                        return Err(ParserError::UnexpectedToken(
                            t_token.kind,
                            t_token.span.start_line,
                            t_token.span.start_col,
                            "parameter type",
                        ));
                    }
                } else {
                    None
                };

                params.push(Parameter {
                    name: param_name,
                    is_by_val,
                    param_type,
                    is_optional,
                });

                if self.check(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }

            if !self.check(&TokenKind::RParen) {
                let span = self.peek().span;
                return Err(ParserError::UnexpectedToken(
                    self.peek().kind.clone(),
                    span.start_line,
                    span.start_col,
                    "')' closing parameter list",
                ));
            }
            self.advance(); // consume ')'
        }
        Ok(params)
    }

    fn parse_statement(&mut self) -> Result<Statement, ParserError> {
        let token = self.peek().clone();

        match &token.kind {
            TokenKind::Comment(text) => {
                self.advance();
                Ok(Statement::Comment(text.clone()))
            }
            TokenKind::Call => {
                let call_span = self.advance().span;
                let target_token = self.peek().clone();
                let target_name = match target_token.kind {
                    TokenKind::Identifier(ref id) => {
                        self.advance();
                        id.clone()
                    }
                    _ => {
                        return Err(ParserError::UnexpectedToken(
                            target_token.kind,
                            target_token.span.start_line,
                            target_token.span.start_col,
                            "procedure name after Call",
                        ));
                    }
                };

                // In `Call Name(arg1, arg2)`, parentheses are mandatory
                let mut args = Vec::new();
                if self.check(&TokenKind::LParen) {
                    self.advance();
                    while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                        args.push(self.parse_expression()?);
                        if self.check(&TokenKind::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    if !self.check(&TokenKind::RParen) {
                        let span = self.peek().span;
                        return Err(ParserError::UnexpectedToken(
                            self.peek().kind.clone(),
                            span.start_line,
                            span.start_col,
                            "')'",
                        ));
                    }
                    self.advance();
                }

                self.expect_statement_terminator("Call statement")?;

                Ok(Statement::Call {
                    name: target_name,
                    arguments: args,
                    has_call_keyword: true,
                    span: call_span,
                })
            }
            TokenKind::Identifier(id) => {
                let id_clone = id.clone();
                let id_span = token.span;
                self.advance();

                // Check if this is an assignment: `Target = Value`
                if self.check(&TokenKind::Equals) {
                    self.advance();
                    let value = self.parse_expression()?;
                    self.expect_statement_terminator("Assignment statement")?;
                    return Ok(Statement::Assignment {
                        target: id_clone,
                        value,
                        is_set: false,
                        span: id_span,
                    });
                }

                // Otherwise, this is a procedure call without Call keyword:
                // e.g. `MsgBox "Hello VB6"`
                let mut args = Vec::new();
                while !matches!(self.peek().kind, TokenKind::Newline | TokenKind::Colon | TokenKind::Eof) {
                    args.push(self.parse_expression()?);
                    if self.check(&TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }

                self.expect_statement_terminator("Sub call statement")?;

                Ok(Statement::Call {
                    name: id_clone,
                    arguments: args,
                    has_call_keyword: false,
                    span: id_span,
                })
            }
            _ => {
                let span = token.span;
                Err(ParserError::UnexpectedToken(
                    token.kind,
                    span.start_line,
                    span.start_col,
                    "statement",
                ))
            }
        }
    }

    fn parse_expression(&mut self) -> Result<Expression, ParserError> {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::StringLiteral(s) => {
                self.advance();
                Ok(Expression::Literal(Literal::String(s)))
            }
            TokenKind::IntegerLiteral(i) => {
                self.advance();
                Ok(Expression::Literal(Literal::Integer(i)))
            }
            TokenKind::DoubleLiteral(d) => {
                self.advance();
                Ok(Expression::Literal(Literal::Double(d)))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expression::Literal(Literal::Boolean(true)))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expression::Literal(Literal::Boolean(false)))
            }
            TokenKind::Null => {
                self.advance();
                Ok(Expression::Literal(Literal::Null))
            }
            TokenKind::Empty => {
                self.advance();
                Ok(Expression::Literal(Literal::Empty))
            }
            TokenKind::Identifier(id) => {
                self.advance();
                Ok(Expression::Variable(id))
            }
            _ => {
                let span = token.span;
                Err(ParserError::UnexpectedToken(
                    token.kind,
                    span.start_line,
                    span.start_col,
                    "expression",
                ))
            }
        }
    }

    fn expect_statement_terminator(&mut self, context: &'static str) -> Result<(), ParserError> {
        if matches!(
            self.peek().kind,
            TokenKind::Newline | TokenKind::Colon | TokenKind::Eof
        ) {
            if !self.is_at_end() {
                self.advance();
            }
            Ok(())
        } else {
            let token = self.peek().clone();
            Err(ParserError::UnexpectedToken(
                token.kind,
                token.span.start_line,
                token.span.start_col,
                context,
            ))
        }
    }
}
