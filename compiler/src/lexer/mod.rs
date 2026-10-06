//! Lexer / Tokenizer for Visual Basic 6.0 source code.

use crate::ast::Span;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LexerError {
    #[error("Unterminated string literal at line {0}, col {1}")]
    UnterminatedString(usize, usize),
    #[error("Unexpected character '{0}' at line {1}, col {2}")]
    UnexpectedCharacter(char, usize, usize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Sub,
    Function,
    End,
    Call,
    Set,
    Let,
    Dim,
    As,
    Public,
    Private,
    Friend,
    ByVal,
    ByRef,
    Optional,
    True,
    False,
    Null,
    Empty,

    // Identifiers & Literals
    Identifier(String),
    StringLiteral(String),
    IntegerLiteral(i32),
    DoubleLiteral(f64),

    // Symbols & Punctuation
    LParen,
    RParen,
    Comma,
    Dot,
    Colon,
    Equals,
    NotEquals,
    LessThan,
    GreaterThan,
    Plus,
    Minus,
    Asterisk,
    Slash,
    Ampersand,

    // Structure
    Newline,
    Comment(String),
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub struct Lexer<'a> {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    _source: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            _source: source,
        }
    }

    fn peek(&self) -> Option<char> {
        if self.pos < self.chars.len() {
            Some(self.chars[self.pos])
        } else {
            None
        }
    }

    fn peek_next(&self) -> Option<char> {
        if self.pos + 1 < self.chars.len() {
            Some(self.chars[self.pos + 1])
        } else {
            None
        }
    }

    fn advance(&mut self) -> Option<char> {
        if let Some(ch) = self.peek() {
            self.pos += 1;
            if ch == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            Some(ch)
        } else {
            None
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();

        while let Some(ch) = self.peek() {
            let start_line = self.line;
            let start_col = self.col;

            // Handle whitespace (excluding newlines)
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
                continue;
            }

            // Handle line continuation: ' _' followed by newline
            if ch == '_' {
                if let Some(next) = self.peek_next() {
                    if next == '\r' || next == '\n' {
                        self.advance(); // consume '_'
                        if self.peek() == Some('\r') {
                            self.advance();
                        }
                        if self.peek() == Some('\n') {
                            self.advance();
                        }
                        continue;
                    }
                }
            }

            // Handle Newlines
            if ch == '\n' {
                self.advance();
                tokens.push(Token {
                    kind: TokenKind::Newline,
                    span: Span::new(start_line, start_col, start_line, start_col + 1),
                });
                continue;
            }

            // Handle Comments: single quote '
            if ch == '\'' {
                self.advance();
                let mut comment = String::new();
                while let Some(c) = self.peek() {
                    if c == '\n' || c == '\r' {
                        break;
                    }
                    comment.push(c);
                    self.advance();
                }
                tokens.push(Token {
                    kind: TokenKind::Comment(comment),
                    span: Span::new(start_line, start_col, self.line, self.col),
                });
                continue;
            }

            // Handle String Literals: "..."
            if ch == '"' {
                self.advance(); // consume opening quote
                let mut string_val = String::new();
                let mut closed = false;

                while let Some(c) = self.peek() {
                    if c == '"' {
                        if self.peek_next() == Some('"') {
                            // Escaped double quote ("")
                            self.advance();
                            self.advance();
                            string_val.push('"');
                        } else {
                            // Closing quote
                            self.advance();
                            closed = true;
                            break;
                        }
                    } else if c == '\n' || c == '\r' {
                        // In VB6, string literals cannot span across unescaped newlines
                        return Err(LexerError::UnterminatedString(start_line, start_col));
                    } else {
                        string_val.push(c);
                        self.advance();
                    }
                }

                if !closed {
                    return Err(LexerError::UnterminatedString(start_line, start_col));
                }

                tokens.push(Token {
                    kind: TokenKind::StringLiteral(string_val),
                    span: Span::new(start_line, start_col, self.line, self.col),
                });
                continue;
            }

            // Handle Numbers
            if ch.is_ascii_digit() {
                let mut num_str = String::new();
                let mut is_double = false;

                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() {
                        num_str.push(c);
                        self.advance();
                    } else if c == '.' && !is_double && self.peek_next().map_or(false, |n| n.is_ascii_digit()) {
                        is_double = true;
                        num_str.push(c);
                        self.advance();
                    } else {
                        break;
                    }
                }

                let kind = if is_double {
                    TokenKind::DoubleLiteral(num_str.parse().unwrap_or(0.0))
                } else {
                    TokenKind::IntegerLiteral(num_str.parse().unwrap_or(0))
                };

                tokens.push(Token {
                    kind,
                    span: Span::new(start_line, start_col, self.line, self.col),
                });
                continue;
            }

            // Handle Identifiers and Keywords
            if ch.is_alphabetic() || ch == '_' {
                let mut id = String::new();
                while let Some(c) = self.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        id.push(c);
                        self.advance();
                    } else {
                        break;
                    }
                }

                // Check for Rem comment
                if id.eq_ignore_ascii_case("rem") {
                    let mut comment = String::new();
                    while let Some(c) = self.peek() {
                        if c == '\n' || c == '\r' {
                            break;
                        }
                        comment.push(c);
                        self.advance();
                    }
                    tokens.push(Token {
                        kind: TokenKind::Comment(comment),
                        span: Span::new(start_line, start_col, self.line, self.col),
                    });
                    continue;
                }

                // Match keywords case-insensitively
                let kind = match id.to_ascii_uppercase().as_str() {
                    "SUB" => TokenKind::Sub,
                    "FUNCTION" => TokenKind::Function,
                    "END" => TokenKind::End,
                    "CALL" => TokenKind::Call,
                    "SET" => TokenKind::Set,
                    "LET" => TokenKind::Let,
                    "DIM" => TokenKind::Dim,
                    "AS" => TokenKind::As,
                    "PUBLIC" => TokenKind::Public,
                    "PRIVATE" => TokenKind::Private,
                    "FRIEND" => TokenKind::Friend,
                    "BYVAL" => TokenKind::ByVal,
                    "BYREF" => TokenKind::ByRef,
                    "OPTIONAL" => TokenKind::Optional,
                    "TRUE" => TokenKind::True,
                    "FALSE" => TokenKind::False,
                    "NULL" => TokenKind::Null,
                    "EMPTY" => TokenKind::Empty,
                    _ => TokenKind::Identifier(id),
                };

                tokens.push(Token {
                    kind,
                    span: Span::new(start_line, start_col, self.line, self.col),
                });
                continue;
            }

            // Handle Punctuation and Operators
            self.advance();
            let kind = match ch {
                '(' => TokenKind::LParen,
                ')' => TokenKind::RParen,
                ',' => TokenKind::Comma,
                '.' => TokenKind::Dot,
                ':' => TokenKind::Colon,
                '=' => TokenKind::Equals,
                '+' => TokenKind::Plus,
                '-' => TokenKind::Minus,
                '*' => TokenKind::Asterisk,
                '/' => TokenKind::Slash,
                '&' => TokenKind::Ampersand,
                '<' => {
                    if self.peek() == Some('>') {
                        self.advance();
                        TokenKind::NotEquals
                    } else {
                        TokenKind::LessThan
                    }
                }
                '>' => TokenKind::GreaterThan,
                _ => return Err(LexerError::UnexpectedCharacter(ch, start_line, start_col)),
            };

            tokens.push(Token {
                kind,
                span: Span::new(start_line, start_col, self.line, self.col),
            });
        }

        tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span::new(self.line, self.col, self.line, self.col),
        });

        Ok(tokens)
    }
}
