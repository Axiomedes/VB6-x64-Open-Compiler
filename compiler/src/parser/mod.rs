//! Comprehensive Parser for Visual Basic 6.0 source code.

use crate::ast::decl::*;
use crate::ast::expr::*;
use crate::ast::span::Span;
use crate::ast::stmt::*;
use crate::ast::types::*;
use crate::ast::SourceFile;
use crate::lexer::{Token, TokenKind};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ParserError {
    #[error("Unexpected token {0:?} at line {1}, col {2}. Expected {3}")]
    UnexpectedToken(TokenKind, usize, usize, &'static str),
    #[error("Unexpected end of file while parsing {0}")]
    UnexpectedEof(&'static str),
    #[error("Unclosed block: expected '{0}' to close '{1}'")]
    UnclosedBlock(&'static str, String),
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

    fn peek_next(&self) -> &Token {
        if self.current + 1 < self.tokens.len() {
            &self.tokens[self.current + 1]
        } else {
            &self.tokens[self.current]
        }
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

    fn check_next(&self, kind: &TokenKind) -> bool {
        &self.peek_next().kind == kind
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn skip_newlines_and_colons(&mut self) {
        while matches!(self.peek().kind, TokenKind::Newline | TokenKind::Colon) {
            self.advance();
        }
    }

    /// Parse complete source file.
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
                self.parse_declaration_with_visibility(vis)
            }
            TokenKind::Sub | TokenKind::Function | TokenKind::Property => {
                self.parse_declaration_with_visibility(Visibility::Default)
            }
            TokenKind::Dim | TokenKind::Const | TokenKind::Static => {
                let decl = self.parse_module_variables(Visibility::Private)?;
                Ok(Some(Declaration::ModuleVariables(decl)))
            }
            _ => {
                let span = token.span;
                Err(ParserError::UnexpectedToken(
                    token.kind,
                    span.start_line,
                    span.start_col,
                    "top-level declaration (Sub, Function, Dim, etc.)",
                ))
            }
        }
    }

    fn parse_declaration_with_visibility(
        &mut self,
        vis: Visibility,
    ) -> Result<Option<Declaration>, ParserError> {
        let next_token = self.peek().clone();
        match next_token.kind {
            TokenKind::Sub => {
                self.advance();
                let sub = self.parse_sub_declaration(vis, next_token.span)?;
                Ok(Some(Declaration::Sub(sub)))
            }
            TokenKind::Function => {
                self.advance();
                let func = self.parse_func_declaration(vis, next_token.span)?;
                Ok(Some(Declaration::Function(func)))
            }
            TokenKind::Property => {
                self.advance();
                let prop = self.parse_prop_declaration(vis, next_token.span)?;
                Ok(Some(Declaration::Property(prop)))
            }
            TokenKind::Dim | TokenKind::Const => {
                let vars = self.parse_module_variables(vis)?;
                Ok(Some(Declaration::ModuleVariables(vars)))
            }
            _ => {
                // Check if this is `Public VarName As Type` (implicit Dim)
                if matches!(next_token.kind, TokenKind::Identifier(_)) {
                    let vars = self.parse_variable_declarators()?;
                    self.expect_statement_terminator("Module variable declaration")?;
                    Ok(Some(Declaration::ModuleVariables(ModuleVariableDeclaration {
                        visibility: vis,
                        is_const: false,
                        variables: vars,
                        span: next_token.span,
                    })))
                } else {
                    let span = next_token.span;
                    Err(ParserError::UnexpectedToken(
                        next_token.kind,
                        span.start_line,
                        span.start_col,
                        "'Sub', 'Function', 'Property', or variable name",
                    ))
                }
            }
        }
    }

    fn parse_module_variables(
        &mut self,
        vis: Visibility,
    ) -> Result<ModuleVariableDeclaration, ParserError> {
        let span = self.peek().span;
        let is_const = self.match_token(&TokenKind::Const);
        if !is_const {
            self.match_token(&TokenKind::Dim);
            self.match_token(&TokenKind::Static);
        }

        let variables = self.parse_variable_declarators()?;
        self.expect_statement_terminator("variable declaration")?;

        Ok(ModuleVariableDeclaration {
            visibility: vis,
            is_const,
            variables,
            span,
        })
    }

    fn parse_sub_declaration(
        &mut self,
        visibility: Visibility,
        start_span: Span,
    ) -> Result<SubDeclaration, ParserError> {
        let name = self.expect_identifier("subroutine name")?;
        let params = self.parse_parameter_list()?;
        self.expect_statement_terminator("Sub header")?;

        let (body, end_span) = self.parse_block_until(TokenKind::Sub, &name)?;

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
        let name = self.expect_identifier("function name")?;
        let params = self.parse_parameter_list()?;

        let return_type = if self.match_token(&TokenKind::As) {
            Some(self.parse_type_spec()?)
        } else {
            None
        };

        self.expect_statement_terminator("Function header")?;

        let (body, end_span) = self.parse_block_until(TokenKind::Function, &name)?;

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

    fn parse_prop_declaration(
        &mut self,
        visibility: Visibility,
        start_span: Span,
    ) -> Result<PropertyDeclaration, ParserError> {
        let kind = if self.match_token(&TokenKind::Get) {
            PropertyKind::Get
        } else if self.match_token(&TokenKind::Let) {
            PropertyKind::Let
        } else if self.match_token(&TokenKind::Set) {
            PropertyKind::Set
        } else {
            let span = self.peek().span;
            return Err(ParserError::UnexpectedToken(
                self.peek().kind.clone(),
                span.start_line,
                span.start_col,
                "'Get', 'Let', or 'Set' after 'Property'",
            ));
        };

        let name = self.expect_identifier("property name")?;
        let params = self.parse_parameter_list()?;

        let return_type = if self.match_token(&TokenKind::As) {
            Some(self.parse_type_spec()?)
        } else {
            None
        };

        self.expect_statement_terminator("Property header")?;

        let (body, end_span) = self.parse_block_until(TokenKind::Property, &name)?;

        Ok(PropertyDeclaration {
            kind,
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

    fn parse_block_until(
        &mut self,
        expected_end_kind: TokenKind,
        block_name: &str,
    ) -> Result<(Vec<Statement>, Span), ParserError> {
        let mut body = Vec::new();

        while !self.is_at_end() {
            self.skip_newlines_and_colons();
            if self.is_at_end() {
                break;
            }

            if self.check(&TokenKind::End) && self.check_next(&expected_end_kind) {
                self.advance(); // consume End
                let end_token = self.advance().clone();
                self.skip_newlines_and_colons();
                return Ok((body, end_token.span));
            }

            let stmt = self.parse_statement()?;
            body.push(stmt);
        }

        Err(ParserError::UnclosedBlock("End", block_name.to_string()))
    }

    fn parse_parameter_list(&mut self) -> Result<Vec<Parameter>, ParserError> {
        let mut params = Vec::new();
        if self.match_token(&TokenKind::LParen) {
            while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                let is_optional = self.match_token(&TokenKind::Optional);
                let is_by_val = if self.match_token(&TokenKind::ByVal) {
                    true
                } else {
                    self.match_token(&TokenKind::ByRef);
                    false // Default in VB6 is ByRef
                };

                let is_param_array = self.match_token(&TokenKind::ParamArray);

                let name = self.expect_identifier("parameter name")?;

                // Check for array parentheses: e.g. Arr() or ParamArray Items()
                let is_array_param = if self.match_token(&TokenKind::LParen) {
                    self.expect(&TokenKind::RParen, "')' closing array parameter")?;
                    true
                } else {
                    false
                };

                let mut param_type = if self.match_token(&TokenKind::As) {
                    Some(self.parse_type_spec()?)
                } else {
                    None
                };

                if is_param_array || is_array_param {
                    if let Some(ref mut pt) = param_type {
                        if pt.array_dimensions.is_empty() {
                            pt.array_dimensions.push(crate::ast::types::ArrayDimension { lower: None, upper: -1 });
                        }
                    } else if is_param_array {
                        param_type = Some(TypeSpec {
                            name: "Variant".to_string(),
                            is_new: false,
                            fixed_length: None,
                            array_dimensions: vec![crate::ast::types::ArrayDimension { lower: None, upper: -1 }],
                        });
                    }
                }

                let default_value = if self.match_token(&TokenKind::Equals) {
                    Some(self.parse_expression()?)
                } else {
                    None
                };

                params.push(Parameter {
                    name,
                    is_by_val,
                    param_type,
                    is_optional,
                    is_param_array,
                    default_value,
                });

                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(&TokenKind::RParen, "')' closing parameter list")?;
        }
        Ok(params)
    }

    fn parse_type_spec(&mut self) -> Result<TypeSpec, ParserError> {
        let is_new = self.match_token(&TokenKind::New);
        let name = self.expect_identifier("type name")?;
        Ok(TypeSpec {
            name,
            is_new,
            fixed_length: None,
            array_dimensions: Vec::new(),
        })
    }

    // Statements
    pub fn parse_statement(&mut self) -> Result<Statement, ParserError> {
        let token = self.peek().clone();

        match &token.kind {
            TokenKind::Comment(c) => {
                self.advance();
                Ok(Statement::Comment(c.clone()))
            }
            TokenKind::Dim | TokenKind::Static | TokenKind::Const | TokenKind::ReDim => {
                let kind = match token.kind {
                    TokenKind::Static => VarDeclKind::Static,
                    TokenKind::Const => VarDeclKind::Const,
                    TokenKind::ReDim => {
                        if self.check_next(&TokenKind::Preserve) {
                            VarDeclKind::ReDimPreserve
                        } else {
                            VarDeclKind::ReDim
                        }
                    }
                    _ => VarDeclKind::Dim,
                };
                self.advance();
                if kind == VarDeclKind::ReDimPreserve {
                    self.advance(); // consume Preserve
                }

                let variables = self.parse_variable_declarators()?;
                self.expect_statement_terminator("variable declaration")?;

                Ok(Statement::VariableDeclaration {
                    kind,
                    variables,
                    span: token.span,
                })
            }
            TokenKind::If => self.parse_if_statement(),
            TokenKind::For => self.parse_for_statement(),
            TokenKind::Do => self.parse_do_loop_statement(),
            TokenKind::While => self.parse_while_wend_statement(),
            TokenKind::Select => self.parse_select_case_statement(),
            TokenKind::With => self.parse_with_statement(),
            TokenKind::Exit => self.parse_exit_statement(),
            TokenKind::GoTo => {
                self.advance();
                let label = self.expect_identifier("GoTo target label")?;
                self.expect_statement_terminator("GoTo statement")?;
                Ok(Statement::GoTo {
                    label,
                    span: token.span,
                })
            }
            TokenKind::On => self.parse_on_error_statement(),
            TokenKind::Resume => self.parse_resume_statement(),
            TokenKind::Debug => {
                self.advance();
                self.expect(&TokenKind::Dot, "'.' after Debug")?;
                self.expect(&TokenKind::Print, "'Print' after Debug.")?;
                let mut args = Vec::new();
                while !matches!(self.peek().kind, TokenKind::Newline | TokenKind::Colon | TokenKind::Eof) {
                    args.push(self.parse_expression()?);
                    if self.check(&TokenKind::Comma) || self.check(&TokenKind::Semicolon) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect_statement_terminator("Debug.Print statement")?;
                Ok(Statement::DebugPrint {
                    arguments: args,
                    span: token.span,
                })
            }
            TokenKind::Call => {
                let call_span = self.advance().span;
                let name = self.expect_identifier("procedure name after Call")?;
                let mut args = Vec::new();
                if self.match_token(&TokenKind::LParen) {
                    while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                        args.push(self.parse_expression()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(&TokenKind::RParen, "')' closing Call arguments")?;
                }
                self.expect_statement_terminator("Call statement")?;
                Ok(Statement::Call {
                    name,
                    arguments: args,
                    has_call_keyword: true,
                    span: call_span,
                })
            }
            TokenKind::Set | TokenKind::Let => {
                let is_set = token.kind == TokenKind::Set;
                self.advance();
                let target = self.parse_postfix()?;
                self.expect(&TokenKind::Equals, "'=' in assignment")?;
                let value = self.parse_expression()?;
                self.expect_statement_terminator("Assignment statement")?;
                Ok(Statement::Assignment {
                    target,
                    value,
                    is_set,
                    span: token.span,
                })
            }
            TokenKind::Identifier(id) => {
                // Check if this is a line label: `Label:`
                if self.check_next(&TokenKind::Colon) {
                    let label_name = id.clone();
                    let span = token.span;
                    self.advance(); // id
                    self.advance(); // colon
                    return Ok(Statement::LineLabel {
                        name: label_name,
                        span,
                    });
                }

                // Check assignment: Target = Value or Member.Access = Value
                let target_expr = self.parse_postfix()?;
                if self.match_token(&TokenKind::Equals) {
                    let value = self.parse_expression()?;
                    self.expect_statement_terminator("Assignment statement")?;
                    return Ok(Statement::Assignment {
                        target: target_expr,
                        value,
                        is_set: false,
                        span: token.span,
                    });
                }

                // Sub call command style without Call keyword: SubName Arg1, Arg2
                // or SubName(Arg1, Arg2)
                let (name, args) = match target_expr {
                    Expression::Variable(n) => {
                        let mut args = Vec::new();
                        while !matches!(self.peek().kind, TokenKind::Newline | TokenKind::Colon | TokenKind::Eof) {
                            args.push(self.parse_expression()?);
                            if !self.match_token(&TokenKind::Comma) {
                                break;
                            }
                        }
                        (n, args)
                    }
                    Expression::IndexAccess { target, indices } => {
                        if let Expression::Variable(n) = *target {
                            (n, indices)
                        } else {
                            let span = token.span;
                            return Err(ParserError::UnexpectedToken(
                                token.kind,
                                span.start_line,
                                span.start_col,
                                "statement",
                            ));
                        }
                    }
                    _ => {
                        let span = token.span;
                        return Err(ParserError::UnexpectedToken(
                            token.kind,
                            span.start_line,
                            span.start_col,
                            "statement",
                        ));
                    }
                };

                self.expect_statement_terminator("Sub call statement")?;
                Ok(Statement::Call {
                    name,
                    arguments: args,
                    has_call_keyword: false,
                    span: token.span,
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

    fn parse_variable_declarators(&mut self) -> Result<Vec<VariableDeclarator>, ParserError> {
        let mut vars = Vec::new();
        loop {
            let name = self.expect_identifier("variable name")?;
            let dimensions = if self.match_token(&TokenKind::LParen) {
                let mut dims = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop {
                        let first_expr = self.parse_expression()?;
                        if self.match_token(&TokenKind::To) {
                            let second_expr = self.parse_expression()?;
                            dims.push(crate::ast::stmt::ArrayDimExpr {
                                lower: Some(first_expr),
                                upper: second_expr,
                            });
                        } else {
                            dims.push(crate::ast::stmt::ArrayDimExpr {
                                lower: None,
                                upper: first_expr,
                            });
                        }
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(&TokenKind::RParen, "')' closing array dimensions")?;
                Some(dims)
            } else {
                None
            };
            let type_spec = if self.match_token(&TokenKind::As) {
                Some(self.parse_type_spec()?)
            } else {
                None
            };
            let init_value = if self.match_token(&TokenKind::Equals) {
                Some(self.parse_expression()?)
            } else {
                None
            };

            vars.push(VariableDeclarator {
                name,
                dimensions,
                type_spec,
                init_value,
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        Ok(vars)
    }

    fn parse_if_statement(&mut self) -> Result<Statement, ParserError> {
        let span = self.advance().span; // consume 'If'
        let condition = self.parse_expression()?;
        self.expect(&TokenKind::Then, "'Then' after If condition")?;

        // Check if single-line If: If Cond Then Stmt [Else Stmt]
        if !matches!(self.peek().kind, TokenKind::Newline | TokenKind::Colon) {
            let stmt = self.parse_statement()?;
            let else_branch = if self.match_token(&TokenKind::Else) {
                Some(vec![self.parse_statement()?])
            } else {
                None
            };
            return Ok(Statement::If {
                branches: vec![IfBranch {
                    condition,
                    body: vec![stmt],
                }],
                else_branch,
                span,
            });
        }

        self.expect_statement_terminator("If header")?;

        let mut branches = Vec::new();
        let mut current_cond = condition;
        let mut current_body = Vec::new();
        let mut else_branch = None;

        while !self.is_at_end() {
            self.skip_newlines_and_colons();
            if self.is_at_end() {
                break;
            }

            if self.match_token(&TokenKind::ElseIf) {
                branches.push(IfBranch {
                    condition: current_cond,
                    body: current_body,
                });
                current_body = Vec::new();
                current_cond = self.parse_expression()?;
                self.expect(&TokenKind::Then, "'Then' after ElseIf condition")?;
                self.expect_statement_terminator("ElseIf header")?;
                continue;
            }

            if self.match_token(&TokenKind::Else) {
                branches.push(IfBranch {
                    condition: current_cond,
                    body: current_body,
                });
                self.expect_statement_terminator("Else header")?;
                let mut else_stmts = Vec::new();
                while !self.is_at_end() {
                    self.skip_newlines_and_colons();
                    if self.check(&TokenKind::End) && self.check_next(&TokenKind::If) {
                        break;
                    }
                    else_stmts.push(self.parse_statement()?);
                }
                else_branch = Some(else_stmts);
                break;
            }

            if self.check(&TokenKind::End) && self.check_next(&TokenKind::If) {
                branches.push(IfBranch {
                    condition: current_cond,
                    body: current_body,
                });
                break;
            }

            current_body.push(self.parse_statement()?);
        }

        self.expect(&TokenKind::End, "'End'")?;
        self.expect(&TokenKind::If, "'If' after End")?;
        self.skip_newlines_and_colons();

        Ok(Statement::If {
            branches,
            else_branch,
            span,
        })
    }

    fn parse_for_statement(&mut self) -> Result<Statement, ParserError> {
        let span = self.advance().span; // For

        if self.match_token(&TokenKind::Each) {
            // For Each Item In Collection
            let element = self.expect_identifier("element variable")?;
            self.expect(&TokenKind::In, "'In' after element")?;
            let collection = self.parse_expression()?;
            self.expect_statement_terminator("For Each header")?;

            let mut body = Vec::new();
            while !self.is_at_end() {
                self.skip_newlines_and_colons();
                if self.check(&TokenKind::Next) {
                    break;
                }
                body.push(self.parse_statement()?);
            }
            self.expect(&TokenKind::Next, "'Next'")?;
            if matches!(self.peek().kind, TokenKind::Identifier(_)) {
                self.advance();
            }
            self.expect_statement_terminator("Next statement")?;

            return Ok(Statement::ForEach {
                element,
                collection,
                body,
                span,
            });
        }

        let counter = self.expect_identifier("counter variable")?;
        self.expect(&TokenKind::Equals, "'=' in For loop")?;
        let start = self.parse_expression()?;
        self.expect(&TokenKind::To, "'To' in For loop")?;
        let end = self.parse_expression()?;

        let step = if self.match_token(&TokenKind::Step) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.expect_statement_terminator("For header")?;

        let mut body = Vec::new();
        while !self.is_at_end() {
            self.skip_newlines_and_colons();
            if self.check(&TokenKind::Next) {
                break;
            }
            body.push(self.parse_statement()?);
        }

        self.expect(&TokenKind::Next, "'Next'")?;
        if matches!(self.peek().kind, TokenKind::Identifier(_)) {
            self.advance();
        }
        self.expect_statement_terminator("Next statement")?;

        Ok(Statement::For {
            counter,
            start,
            end,
            step,
            body,
            span,
        })
    }

    fn parse_do_loop_statement(&mut self) -> Result<Statement, ParserError> {
        let span = self.advance().span; // Do

        let mut condition = None;
        let mut condition_at_start = false;

        if self.match_token(&TokenKind::While) {
            condition = Some(LoopCondition::While(self.parse_expression()?));
            condition_at_start = true;
        } else if self.match_token(&TokenKind::Until) {
            condition = Some(LoopCondition::Until(self.parse_expression()?));
            condition_at_start = true;
        }

        self.expect_statement_terminator("Do header")?;

        let mut body = Vec::new();
        while !self.is_at_end() {
            self.skip_newlines_and_colons();
            if self.check(&TokenKind::Loop) {
                break;
            }
            body.push(self.parse_statement()?);
        }

        self.expect(&TokenKind::Loop, "'Loop'")?;

        if !condition_at_start {
            if self.match_token(&TokenKind::While) {
                condition = Some(LoopCondition::While(self.parse_expression()?));
            } else if self.match_token(&TokenKind::Until) {
                condition = Some(LoopCondition::Until(self.parse_expression()?));
            }
        }

        self.expect_statement_terminator("Loop statement")?;

        Ok(Statement::DoLoop {
            condition,
            condition_at_start,
            body,
            span,
        })
    }

    fn parse_while_wend_statement(&mut self) -> Result<Statement, ParserError> {
        let span = self.advance().span; // While
        let condition = self.parse_expression()?;
        self.expect_statement_terminator("While header")?;

        let mut body = Vec::new();
        while !self.is_at_end() {
            self.skip_newlines_and_colons();
            if self.check(&TokenKind::Wend) {
                break;
            }
            body.push(self.parse_statement()?);
        }

        self.expect(&TokenKind::Wend, "'Wend'")?;
        self.expect_statement_terminator("Wend statement")?;

        Ok(Statement::WhileWend {
            condition,
            body,
            span,
        })
    }

    fn parse_select_case_statement(&mut self) -> Result<Statement, ParserError> {
        let span = self.advance().span; // Select
        self.expect(&TokenKind::Case, "'Case' after Select")?;
        let expression = self.parse_expression()?;
        self.expect_statement_terminator("Select Case header")?;

        let mut cases = Vec::new();
        let mut else_case = None;

        while !self.is_at_end() {
            self.skip_newlines_and_colons();
            if self.check(&TokenKind::End) && self.check_next(&TokenKind::Select) {
                break;
            }

            if self.match_token(&TokenKind::Case) {
                if self.match_token(&TokenKind::Else) {
                    self.expect_statement_terminator("Case Else")?;
                    let mut body = Vec::new();
                    while !self.is_at_end() {
                        self.skip_newlines_and_colons();
                        if (self.check(&TokenKind::End) && self.check_next(&TokenKind::Select)) || self.check(&TokenKind::Case) {
                            break;
                        }
                        body.push(self.parse_statement()?);
                    }
                    else_case = Some(body);
                    continue;
                }

                let mut values = Vec::new();
                loop {
                    if self.match_token(&TokenKind::Is) {
                        let op = match self.peek().kind {
                            TokenKind::Equals => BinaryOperator::Equal,
                            TokenKind::NotEquals => BinaryOperator::NotEqual,
                            TokenKind::LessThan => BinaryOperator::LessThan,
                            TokenKind::LessThanOrEqual => BinaryOperator::LessThanOrEqual,
                            TokenKind::GreaterThan => BinaryOperator::GreaterThan,
                            TokenKind::GreaterThanOrEqual => BinaryOperator::GreaterThanOrEqual,
                            _ => BinaryOperator::Equal,
                        };
                        self.advance();
                        let target = self.parse_expression()?;
                        values.push(CaseValue::Comparison(op, target));
                    } else {
                        let val = self.parse_expression()?;
                        if self.match_token(&TokenKind::To) {
                            let end_val = self.parse_expression()?;
                            values.push(CaseValue::Range(val, end_val));
                        } else {
                            values.push(CaseValue::Exact(val));
                        }
                    }

                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }

                self.expect_statement_terminator("Case clause")?;

                let mut body = Vec::new();
                while !self.is_at_end() {
                    self.skip_newlines_and_colons();
                    if self.check(&TokenKind::Case) || (self.check(&TokenKind::End) && self.check_next(&TokenKind::Select)) {
                        break;
                    }
                    body.push(self.parse_statement()?);
                }

                cases.push(CaseClause { values, body });
            }
        }

        self.expect(&TokenKind::End, "'End'")?;
        self.expect(&TokenKind::Select, "'Select' after End")?;
        self.skip_newlines_and_colons();

        Ok(Statement::SelectCase {
            expression,
            cases,
            else_case,
            span,
        })
    }

    fn parse_with_statement(&mut self) -> Result<Statement, ParserError> {
        let span = self.advance().span; // With
        let target = self.parse_expression()?;
        self.expect_statement_terminator("With header")?;

        let mut body = Vec::new();
        while !self.is_at_end() {
            self.skip_newlines_and_colons();
            if self.check(&TokenKind::End) && self.check_next(&TokenKind::With) {
                break;
            }
            body.push(self.parse_statement()?);
        }

        self.expect(&TokenKind::End, "'End'")?;
        self.expect(&TokenKind::With, "'With' after End")?;
        self.skip_newlines_and_colons();

        Ok(Statement::With {
            target,
            body,
            span,
        })
    }

    fn parse_exit_statement(&mut self) -> Result<Statement, ParserError> {
        let span = self.advance().span; // Exit
        let target = match self.peek().kind {
            TokenKind::Sub => ExitTarget::Sub,
            TokenKind::Function => ExitTarget::Function,
            TokenKind::Property => ExitTarget::Property,
            TokenKind::For => ExitTarget::For,
            TokenKind::Do => ExitTarget::Do,
            _ => {
                let span = self.peek().span;
                return Err(ParserError::UnexpectedToken(
                    self.peek().kind.clone(),
                    span.start_line,
                    span.start_col,
                    "'Sub', 'Function', 'Property', 'For', or 'Do' after Exit",
                ));
            }
        };
        self.advance();
        self.expect_statement_terminator("Exit statement")?;
        Ok(Statement::Exit { target, span })
    }

    fn parse_on_error_statement(&mut self) -> Result<Statement, ParserError> {
        let span = self.advance().span; // On
        self.expect(&TokenKind::Error, "'Error' after On")?;

        let action = if self.match_token(&TokenKind::Resume) {
            self.expect(&TokenKind::Next, "'Next' after Resume")?;
            OnErrorAction::ResumeNext
        } else if self.match_token(&TokenKind::GoTo) {
            if self.check(&TokenKind::IntegerLiteral(0)) {
                self.advance();
                OnErrorAction::GoToZero
            } else {
                let label = self.expect_identifier("label after GoTo")?;
                OnErrorAction::GoTo(label)
            }
        } else {
            let span = self.peek().span;
            return Err(ParserError::UnexpectedToken(
                self.peek().kind.clone(),
                span.start_line,
                span.start_col,
                "'Resume Next' or 'GoTo'",
            ));
        };

        self.expect_statement_terminator("On Error statement")?;
        Ok(Statement::OnError { action, span })
    }

    fn parse_resume_statement(&mut self) -> Result<Statement, ParserError> {
        let span = self.advance().span; // Resume
        let action = if self.match_token(&TokenKind::Next) {
            ResumeAction::Next
        } else if matches!(self.peek().kind, TokenKind::Identifier(_)) {
            let label = self.expect_identifier("Resume label")?;
            ResumeAction::Label(label)
        } else {
            ResumeAction::Current
        };
        self.expect_statement_terminator("Resume statement")?;
        Ok(Statement::Resume { action, span })
    }

    // Expressions (Precedence Climbing)
    pub fn parse_expression(&mut self) -> Result<Expression, ParserError> {
        self.parse_logical_imp()
    }

    fn parse_logical_imp(&mut self) -> Result<Expression, ParserError> {
        let mut left = self.parse_logical_eqv()?;
        while self.match_token(&TokenKind::Imp) {
            let right = self.parse_logical_eqv()?;
            left = Expression::BinaryOp {
                op: BinaryOperator::Imp,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_logical_eqv(&mut self) -> Result<Expression, ParserError> {
        let mut left = self.parse_logical_xor()?;
        while self.match_token(&TokenKind::Eqv) {
            let right = self.parse_logical_xor()?;
            left = Expression::BinaryOp {
                op: BinaryOperator::Eqv,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_logical_xor(&mut self) -> Result<Expression, ParserError> {
        let mut left = self.parse_logical_or()?;
        while self.match_token(&TokenKind::Xor) {
            let right = self.parse_logical_or()?;
            left = Expression::BinaryOp {
                op: BinaryOperator::Xor,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_logical_or(&mut self) -> Result<Expression, ParserError> {
        let mut left = self.parse_logical_and()?;
        while self.match_token(&TokenKind::Or) {
            let right = self.parse_logical_and()?;
            left = Expression::BinaryOp {
                op: BinaryOperator::Or,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<Expression, ParserError> {
        let mut left = self.parse_logical_not()?;
        while self.match_token(&TokenKind::And) {
            let right = self.parse_logical_not()?;
            left = Expression::BinaryOp {
                op: BinaryOperator::And,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_logical_not(&mut self) -> Result<Expression, ParserError> {
        if self.match_token(&TokenKind::Not) {
            let operand = self.parse_logical_not()?;
            Ok(Expression::UnaryOp {
                op: UnaryOperator::Not,
                operand: Box::new(operand),
            })
        } else {
            self.parse_comparison()
        }
    }

    fn parse_comparison(&mut self) -> Result<Expression, ParserError> {
        let mut left = self.parse_concatenation()?;
        while let Some(op) = self.match_comparison_op() {
            let right = self.parse_concatenation()?;
            left = Expression::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn match_comparison_op(&mut self) -> Option<BinaryOperator> {
        let op = match self.peek().kind {
            TokenKind::Equals => BinaryOperator::Equal,
            TokenKind::NotEquals => BinaryOperator::NotEqual,
            TokenKind::LessThan => BinaryOperator::LessThan,
            TokenKind::LessThanOrEqual => BinaryOperator::LessThanOrEqual,
            TokenKind::GreaterThan => BinaryOperator::GreaterThan,
            TokenKind::GreaterThanOrEqual => BinaryOperator::GreaterThanOrEqual,
            TokenKind::Is => BinaryOperator::Is,
            TokenKind::Like => BinaryOperator::Like,
            _ => return None,
        };
        self.advance();
        Some(op)
    }

    fn parse_concatenation(&mut self) -> Result<Expression, ParserError> {
        let mut left = self.parse_additive()?;
        while self.match_token(&TokenKind::Ampersand) {
            let right = self.parse_additive()?;
            left = Expression::BinaryOp {
                op: BinaryOperator::Concat,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expression, ParserError> {
        let mut left = self.parse_modulo()?;
        while let Some(op) = self.match_additive_op() {
            let right = self.parse_modulo()?;
            left = Expression::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn match_additive_op(&mut self) -> Option<BinaryOperator> {
        let op = match self.peek().kind {
            TokenKind::Plus => BinaryOperator::Add,
            TokenKind::Minus => BinaryOperator::Subtract,
            _ => return None,
        };
        self.advance();
        Some(op)
    }

    fn parse_modulo(&mut self) -> Result<Expression, ParserError> {
        let mut left = self.parse_integer_divide()?;
        while self.match_token(&TokenKind::Mod) {
            let right = self.parse_integer_divide()?;
            left = Expression::BinaryOp {
                op: BinaryOperator::Modulo,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_integer_divide(&mut self) -> Result<Expression, ParserError> {
        let mut left = self.parse_multiplicative()?;
        while self.match_token(&TokenKind::Backslash) {
            let right = self.parse_multiplicative()?;
            left = Expression::BinaryOp {
                op: BinaryOperator::IntegerDivide,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expression, ParserError> {
        let mut left = self.parse_unary()?;
        while let Some(op) = self.match_multiplicative_op() {
            let right = self.parse_unary()?;
            left = Expression::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn match_multiplicative_op(&mut self) -> Option<BinaryOperator> {
        let op = match self.peek().kind {
            TokenKind::Asterisk => BinaryOperator::Multiply,
            TokenKind::Slash => BinaryOperator::Divide,
            _ => return None,
        };
        self.advance();
        Some(op)
    }

    fn parse_unary(&mut self) -> Result<Expression, ParserError> {
        if self.match_token(&TokenKind::Minus) {
            let operand = self.parse_unary()?;
            Ok(Expression::UnaryOp {
                op: UnaryOperator::Negate,
                operand: Box::new(operand),
            })
        } else if self.match_token(&TokenKind::Plus) {
            let operand = self.parse_unary()?;
            Ok(Expression::UnaryOp {
                op: UnaryOperator::Plus,
                operand: Box::new(operand),
            })
        } else {
            self.parse_exponent()
        }
    }

    fn parse_exponent(&mut self) -> Result<Expression, ParserError> {
        let mut left = self.parse_postfix()?;
        while self.match_token(&TokenKind::Caret) {
            let right = self.parse_unary()?;
            left = Expression::BinaryOp {
                op: BinaryOperator::Exponent,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_postfix(&mut self) -> Result<Expression, ParserError> {
        let mut expr = self.parse_primary_expression()?;

        loop {
            if self.match_token(&TokenKind::Dot) {
                let member = self.expect_identifier("member name after '.'")?;
                expr = Expression::MemberAccess {
                    object: Box::new(expr),
                    member,
                };
            } else if self.match_token(&TokenKind::LParen) {
                let mut indices = Vec::new();
                while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                    indices.push(self.parse_expression()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RParen, "')' closing index access")?;
                expr = Expression::IndexAccess {
                    target: Box::new(expr),
                    indices,
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary_expression(&mut self) -> Result<Expression, ParserError> {
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
            TokenKind::DateLiteral(dt) => {
                self.advance();
                Ok(Expression::Literal(Literal::Date(dt)))
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
            TokenKind::Nothing => {
                self.advance();
                Ok(Expression::Literal(Literal::Nothing))
            }
            TokenKind::Dot => {
                // With member access: .PropertyName
                self.advance();
                let member = self.expect_identifier("member name after '.'")?;
                Ok(Expression::WithMemberAccess(member))
            }
            TokenKind::Identifier(id) => {
                self.advance();
                Ok(Expression::Variable(id))
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(&TokenKind::RParen, "')' closing expression")?;
                Ok(Expression::Paren(Box::new(expr)))
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

    fn expect_identifier(&mut self, context: &'static str) -> Result<String, ParserError> {
        let token = self.peek().clone();
        if let TokenKind::Identifier(id) = token.kind {
            self.advance();
            Ok(id)
        } else {
            Err(ParserError::UnexpectedToken(
                token.kind,
                token.span.start_line,
                token.span.start_col,
                context,
            ))
        }
    }

    fn expect(&mut self, kind: &TokenKind, context: &'static str) -> Result<(), ParserError> {
        if self.check(kind) {
            self.advance();
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
