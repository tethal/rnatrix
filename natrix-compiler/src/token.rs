use crate::ctx::{CompilerContext, Interner, Name};
use crate::error::{SourceError, SourceResult};
use crate::src::{Cursor, SourceId, Span};
pub use crate::token_type::TokenType;

#[derive(Debug, Copy, Clone)]
pub struct Token {
    pub tt: TokenType,
    pub span: Span,
    pub name: Option<Name>,
}

pub struct Tokenizer<'a> {
    cursor: Cursor<'a>,
    interner: &'a mut Interner,
}

impl<'a> Tokenizer<'a> {
    pub fn new(ctx: &'a mut CompilerContext, source_id: SourceId) -> Tokenizer<'a> {
        Tokenizer {
            cursor: Cursor::new(ctx.sources.get_by_id(source_id)),
            interner: &mut ctx.interner,
        }
    }

    pub fn next_token(&mut self) -> SourceResult<Token> {
        loop {
            self.cursor.mark();
            let tt = self.parse_token_type()?;
            if tt == TokenType::Comment || tt == TokenType::Whitespace {
                continue;
            }
            let span = self.cursor.span_from_mark();
            if tt == TokenType::Identifier {
                let lexeme = self.cursor.lexeme(span);
                let name = self.interner.intern(lexeme);
                return Ok(Token {
                    tt: self
                        .interner
                        .resolve_keyword(name)
                        .unwrap_or(TokenType::Identifier),
                    span,
                    name: Some(name),
                });
            }
            return Ok(Token {
                tt,
                span,
                name: None,
            });
        }
    }

    pub fn lexeme(&self, token: &Token) -> &str {
        self.cursor.lexeme(token.span)
    }

    fn parse_token_type(&mut self) -> SourceResult<TokenType> {
        match self.cursor.advance() {
            Some(c) if c.is_whitespace() => self.do_whitespace(),
            Some(c) if c.is_ascii_digit() => self.do_number(),
            Some(c) if c.is_ascii_alphabetic() || c == '_' => self.do_identifier(),
            Some('(') => Ok(TokenType::LParen),
            Some(')') => Ok(TokenType::RParen),
            Some('[') => Ok(TokenType::LBracket),
            Some(']') => Ok(TokenType::RBracket),
            Some('{') => Ok(TokenType::LBrace),
            Some('}') => Ok(TokenType::RBrace),
            Some('+') => Ok(TokenType::Plus),
            Some('-') => Ok(TokenType::Minus),
            Some('*') => Ok(TokenType::Star),
            Some('/') => {
                if self.cursor.peek() == Some('/') {
                    while self.cursor.peek() != Some('\n') && self.cursor.peek() != None {
                        self.cursor.advance();
                    }
                    Ok(TokenType::Comment)
                } else {
                    Ok(TokenType::Slash)
                }
            }
            Some('%') => Ok(TokenType::Percent),
            Some('=') => self.two_char_symbol('=', TokenType::Assign, TokenType::Eq),
            Some('!') => self.two_char_symbol('=', TokenType::Bang, TokenType::Ne),
            Some('>') => self.two_char_symbol('=', TokenType::Gt, TokenType::Ge),
            Some('<') => self.two_char_symbol('=', TokenType::Lt, TokenType::Le),
            Some('|') => {
                if self.cursor.peek() == Some('|') {
                    self.cursor.advance();
                    Ok(TokenType::Or)
                } else {
                    self.err("bitwise or not supported")
                }
            }
            Some('&') => {
                if self.cursor.peek() == Some('&') {
                    self.cursor.advance();
                    Ok(TokenType::And)
                } else {
                    self.err("bitwise and not supported")
                }
            }
            Some(',') => Ok(TokenType::Comma),
            Some(';') => Ok(TokenType::Semicolon),
            Some('"') => self.do_string_literal(),
            Some(c) => self.err(format!("unexpected character {:?}", c)),
            None => Ok(TokenType::Eof),
        }
    }

    fn two_char_symbol(
        &mut self,
        second_char: char,
        one_char: TokenType,
        two_char: TokenType,
    ) -> SourceResult<TokenType> {
        if self.cursor.peek() == Some(second_char) {
            self.cursor.advance();
            Ok(two_char)
        } else {
            Ok(one_char)
        }
    }

    fn do_whitespace(&mut self) -> SourceResult<TokenType> {
        while self.cursor.peek().is_some_and(|c| c.is_whitespace()) {
            self.cursor.advance();
        }
        Ok(TokenType::Whitespace)
    }

    fn do_number(&mut self) -> SourceResult<TokenType> {
        while self.cursor.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.cursor.advance();
        }
        if self.cursor.peek() == Some('.') {
            self.cursor.advance();
            if !self.cursor.peek().is_some_and(|c| c.is_ascii_digit()) {
                return self.err("expected digit after decimal point");
            }
            while self.cursor.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.cursor.advance();
            }
            Ok(TokenType::FloatLiteral)
        } else {
            Ok(TokenType::IntLiteral)
        }
    }

    fn do_identifier(&mut self) -> SourceResult<TokenType> {
        while self
            .cursor
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            self.cursor.advance();
        }
        Ok(TokenType::Identifier)
    }

    fn do_string_literal(&mut self) -> SourceResult<TokenType> {
        // Opening quote already consumed
        loop {
            match self.cursor.peek() {
                None => {
                    return self.err("unterminated string literal");
                }
                Some('\n') => {
                    return self.err("unterminated string literal (newline in string)");
                }
                Some('"') => {
                    self.cursor.advance(); // Consume closing quote
                    return Ok(TokenType::StringLiteral);
                }
                Some('\\') => {
                    self.cursor.advance(); // Consume backslash
                    match self.cursor.peek() {
                        Some('"') | Some('\\') | Some('n') | Some('t') | Some('r') | Some('0') => {
                            self.cursor.advance(); // Consume escape char
                        }
                        Some(c) => {
                            return self.err(format!("unknown escape sequence: \\{}", c));
                        }
                        None => {
                            return self.err("unterminated string literal (escape at end)");
                        }
                    }
                }
                Some(_) => {
                    self.cursor.advance(); // Regular character
                }
            }
        }
    }

    fn err<T>(&self, message: impl Into<Box<str>>) -> SourceResult<T> {
        Err(self.error(message))
    }

    fn error(&self, message: impl Into<Box<str>>) -> SourceError {
        SourceError {
            message: message.into(),
            span: self.cursor.span_from_mark(),
        }
    }
}
