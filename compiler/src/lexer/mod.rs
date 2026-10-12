//! Lexer / Tokenizer for Visual Basic 6.0 source code.

use crate::ast::Span;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LexerError {
    #[error("Unterminated string literal at line {0}, col {1}")]
    UnterminatedString(usize, usize),
    #[error("Unterminated date literal at line {0}, col {1}")]
    UnterminatedDate(usize, usize),
    #[error("Unexpected character '{0}' at line {1}, col {2}")]
    UnexpectedCharacter(char, usize, usize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Declarations
    Sub,
    Function,
    Property,
    Get,
    End,
    Dim,
    ReDim,
    Preserve,
    Static,
    Const,
    Type,
    Enum,
    Class,
    Me,
    As,
    New,
    Public,
    Private,
    Friend,
    ByVal,
    ByRef,
    Optional,
    ParamArray,

    // Flow control
    If,
    Then,
    Else,
    ElseIf,
    Select,
    Case,
    For,
    To,
    Step,
    Next,
    Each,
    In,
    Do,
    While,
    Until,
    Loop,
    Wend,
    With,
    Exit,
    GoTo,
    On,
    Error,
    Resume,

    // Statements & calls
    Call,
    Set,
    Let,
    Debug,
    Print,

    // Literals & Identifiers
    True,
    False,
    Null,
    Empty,
    Nothing,
    Identifier(String),
    StringLiteral(String),
    IntegerLiteral(i32),
    DoubleLiteral(f64),
    DateLiteral(String),

    // Operators
    Plus,
    Minus,
    Asterisk,
    Slash,
    Backslash, // Integer division \
    Caret,     // Exponentiation ^
    Ampersand, // String concatenation &
    Mod,
    Equals,
    NotEquals,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    And,
    Or,
    Xor,
    Eqv,
    Imp,
    Not,
    Is,
    Like,

    // Punctuation & Delimiters
    LParen,
    RParen,
    Comma,
    Dot,
    Colon,
    Semicolon,

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

            // Whitespace (excluding newlines)
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
                continue;
            }

            // Line continuation: '_' followed by newline
            if ch == '_' {
                if let Some(next) = self.peek_next() {
                    if next == '\r' || next == '\n' {
                        self.advance();
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

            // Newline
            if ch == '\n' {
                self.advance();
                tokens.push(Token {
                    kind: TokenKind::Newline,
                    span: Span::new(start_line, start_col, start_line, start_col + 1),
                });
                continue;
            }

            // Comments: '
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

            // Date Literals: #...#
            if ch == '#' {
                self.advance();
                let mut date_str = String::new();
                let mut closed = false;
                while let Some(c) = self.peek() {
                    if c == '#' {
                        self.advance();
                        closed = true;
                        break;
                    } else if c == '\n' || c == '\r' {
                        break;
                    } else {
                        date_str.push(c);
                        self.advance();
                    }
                }
                if !closed {
                    return Err(LexerError::UnterminatedDate(start_line, start_col));
                }
                tokens.push(Token {
                    kind: TokenKind::DateLiteral(date_str),
                    span: Span::new(start_line, start_col, self.line, self.col),
                });
                continue;
            }

            // String Literals: "..."
            if ch == '"' {
                self.advance();
                let mut string_val = String::new();
                let mut closed = false;

                while let Some(c) = self.peek() {
                    if c == '"' {
                        if self.peek_next() == Some('"') {
                            self.advance();
                            self.advance();
                            string_val.push('"');
                        } else {
                            self.advance();
                            closed = true;
                            break;
                        }
                    } else if c == '\n' || c == '\r' {
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

            // Hex or Octal numbers: &H... or &O... or standalone & (concatenation)
            if ch == '&' {
                if let Some(next) = self.peek_next() {
                    if next.eq_ignore_ascii_case(&'h') {
                        // Hexadecimal literal &H1F
                        self.advance(); // &
                        self.advance(); // H
                        let mut hex_str = String::new();
                        while let Some(c) = self.peek() {
                            if c.is_ascii_hexdigit() {
                                hex_str.push(c);
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        // Optional suffix & or %
                        if self.peek() == Some('&') || self.peek() == Some('%') {
                            self.advance();
                        }
                        let val = i32::from_str_radix(&hex_str, 16).unwrap_or(0);
                        tokens.push(Token {
                            kind: TokenKind::IntegerLiteral(val),
                            span: Span::new(start_line, start_col, self.line, self.col),
                        });
                        continue;
                    } else if next.eq_ignore_ascii_case(&'o') {
                        // Octal literal &O77
                        self.advance(); // &
                        self.advance(); // O
                        let mut oct_str = String::new();
                        while let Some(c) = self.peek() {
                            if ('0'..='7').contains(&c) {
                                oct_str.push(c);
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        let val = i32::from_str_radix(&oct_str, 8).unwrap_or(0);
                        tokens.push(Token {
                            kind: TokenKind::IntegerLiteral(val),
                            span: Span::new(start_line, start_col, self.line, self.col),
                        });
                        continue;
                    }
                }
                // Standalone & operator (string concatenation)
                self.advance();
                tokens.push(Token {
                    kind: TokenKind::Ampersand,
                    span: Span::new(start_line, start_col, self.line, self.col),
                });
                continue;
            }

            // Decimal Numbers
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
                    } else if (c == 'e' || c == 'E') && !is_double {
                        is_double = true;
                        num_str.push(c);
                        self.advance();
                        if self.peek() == Some('+') || self.peek() == Some('-') {
                            num_str.push(self.advance().unwrap());
                        }
                    } else {
                        break;
                    }
                }

                // Optional type suffixes on numbers (e.g. 10&, 10#, 10!)
                if let Some(c) = self.peek() {
                    if c == '&' || c == '%' || c == '!' || c == '#' || c == '@' {
                        self.advance();
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

            // Escaped Bracket Identifiers: [Select], [Group]
            if ch == '[' {
                self.advance();
                let mut id = String::new();
                while let Some(c) = self.peek() {
                    if c == ']' {
                        self.advance();
                        break;
                    } else if c == '\n' || c == '\r' {
                        break;
                    } else {
                        id.push(c);
                        self.advance();
                    }
                }
                tokens.push(Token {
                    kind: TokenKind::Identifier(id),
                    span: Span::new(start_line, start_col, self.line, self.col),
                });
                continue;
            }

            // Identifiers and Keywords
            if ch.is_alphabetic() || ch == '_' {
                let mut id = String::new();
                while let Some(c) = self.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        id.push(c);
                        self.advance();
                    } else if c == '%' || c == '&' || c == '!' || c == '#' || c == '$' || c == '@' {
                        // Type suffix character at end of identifier
                        id.push(c);
                        self.advance();
                        break;
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

                // Case-insensitive keywords
                let kind = match id.to_ascii_uppercase().as_str() {
                    "SUB" => TokenKind::Sub,
                    "FUNCTION" => TokenKind::Function,
                    "PROPERTY" => TokenKind::Property,
                    "GET" => TokenKind::Get,
                    "END" => TokenKind::End,
                    "DIM" => TokenKind::Dim,
                    "REDIM" => TokenKind::ReDim,
                    "PRESERVE" => TokenKind::Preserve,
                    "STATIC" => TokenKind::Static,
                    "CONST" => TokenKind::Const,
                    "TYPE" => TokenKind::Type,
                    "ENUM" => TokenKind::Enum,
                    "CLASS" => TokenKind::Class,
                    "ME" => TokenKind::Me,
                    "AS" => TokenKind::As,
                    "NEW" => TokenKind::New,
                    "PUBLIC" => TokenKind::Public,
                    "PRIVATE" => TokenKind::Private,
                    "FRIEND" => TokenKind::Friend,
                    "BYVAL" => TokenKind::ByVal,
                    "BYREF" => TokenKind::ByRef,
                    "OPTIONAL" => TokenKind::Optional,
                    "PARAMARRAY" => TokenKind::ParamArray,
                    "IF" => TokenKind::If,
                    "THEN" => TokenKind::Then,
                    "ELSE" => TokenKind::Else,
                    "ELSEIF" => TokenKind::ElseIf,
                    "SELECT" => TokenKind::Select,
                    "CASE" => TokenKind::Case,
                    "FOR" => TokenKind::For,
                    "TO" => TokenKind::To,
                    "STEP" => TokenKind::Step,
                    "NEXT" => TokenKind::Next,
                    "EACH" => TokenKind::Each,
                    "IN" => TokenKind::In,
                    "DO" => TokenKind::Do,
                    "WHILE" => TokenKind::While,
                    "UNTIL" => TokenKind::Until,
                    "LOOP" => TokenKind::Loop,
                    "WEND" => TokenKind::Wend,
                    "WITH" => TokenKind::With,
                    "EXIT" => TokenKind::Exit,
                    "GOTO" => TokenKind::GoTo,
                    "ON" => TokenKind::On,
                    "ERROR" => TokenKind::Error,
                    "RESUME" => TokenKind::Resume,
                    "CALL" => TokenKind::Call,
                    "SET" => TokenKind::Set,
                    "LET" => TokenKind::Let,
                    "DEBUG" => TokenKind::Debug,
                    "PRINT" => TokenKind::Print,
                    "TRUE" => TokenKind::True,
                    "FALSE" => TokenKind::False,
                    "NULL" => TokenKind::Null,
                    "EMPTY" => TokenKind::Empty,
                    "NOTHING" => TokenKind::Nothing,
                    "MOD" => TokenKind::Mod,
                    "AND" => TokenKind::And,
                    "OR" => TokenKind::Or,
                    "XOR" => TokenKind::Xor,
                    "EQV" => TokenKind::Eqv,
                    "IMP" => TokenKind::Imp,
                    "NOT" => TokenKind::Not,
                    "IS" => TokenKind::Is,
                    "LIKE" => TokenKind::Like,
                    _ => TokenKind::Identifier(id),
                };

                tokens.push(Token {
                    kind,
                    span: Span::new(start_line, start_col, self.line, self.col),
                });
                continue;
            }

            // Punctuation and Operators
            self.advance();
            let kind = match ch {
                '(' => TokenKind::LParen,
                ')' => TokenKind::RParen,
                ',' => TokenKind::Comma,
                '.' => TokenKind::Dot,
                ':' => TokenKind::Colon,
                ';' => TokenKind::Semicolon,
                '=' => TokenKind::Equals,
                '+' => TokenKind::Plus,
                '-' => TokenKind::Minus,
                '*' => TokenKind::Asterisk,
                '/' => TokenKind::Slash,
                '\\' => TokenKind::Backslash,
                '^' => TokenKind::Caret,
                '<' => {
                    if self.peek() == Some('>') {
                        self.advance();
                        TokenKind::NotEquals
                    } else if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::LessThanOrEqual
                    } else {
                        TokenKind::LessThan
                    }
                }
                '>' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::GreaterThanOrEqual
                    } else {
                        TokenKind::GreaterThan
                    }
                }
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
