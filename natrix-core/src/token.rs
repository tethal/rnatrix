use crate::error::{err_at, NxResult};
use crate::src::{Cursor, Source, Span};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TokenType {
    Eof,
    Whitespace,
    Comment,
    KwTrue,
    KwFalse,
    KwNull,
    Identifier,
    IntLiteral,
    FloatLiteral,
    LParen,
    RParen,
    Plus,
    Minus,
    Star,
    Slash,
}

#[derive(Debug, Copy, Clone)]
pub struct Token {
    pub tt: TokenType,
    pub span: Span,
}

pub struct Tokenizer<'src> {
    cursor: Cursor<'src>,
}

impl<'src> Tokenizer<'src> {
    pub fn new(source: &'src Source) -> Tokenizer<'src> {
        Tokenizer {
            cursor: Cursor::new(source),
        }
    }

    pub fn next_token(&mut self) -> NxResult<Token> {
        loop {
            self.cursor.mark();
            let mut tt = self.parse_token_type()?;
            let span = self.cursor.span_from_mark();
            if tt == TokenType::Identifier {
                let lexeme = self.cursor.lexeme(span);
                tt = match lexeme {
                    "true" => TokenType::KwTrue,
                    "false" => TokenType::KwFalse,
                    "null" => TokenType::KwNull,
                    _ => TokenType::Identifier,
                }
            }
            let token = Token { tt, span };
            if token.tt != TokenType::Comment && token.tt != TokenType::Whitespace {
                return Ok(token);
            }
        }
    }

    pub fn lexeme(&self, token: &Token) -> &str {
        self.cursor.lexeme(token.span)
    }

    fn parse_token_type(&mut self) -> NxResult<TokenType> {
        match self.cursor.advance() {
            Some(c) if c.is_whitespace() => self.do_whitespace(),
            Some(c) if c.is_ascii_digit() => self.do_number(),
            Some(c) if c.is_ascii_alphabetic() || c == '_' => self.do_identifier(),
            Some('(') => Ok(TokenType::LParen),
            Some(')') => Ok(TokenType::RParen),
            Some('+') => Ok(TokenType::Plus),
            Some('-') => Ok(TokenType::Minus),
            Some('*') => Ok(TokenType::Star),
            Some('/') => Ok(TokenType::Slash),
            Some(c) => err_at(
                self.cursor.span_from_mark(),
                format!("unexpected character {:?}", c),
            ),
            None => Ok(TokenType::Eof),
        }
    }

    fn do_whitespace(&mut self) -> NxResult<TokenType> {
        while self.cursor.peek().is_some_and(|c| c.is_whitespace()) {
            self.cursor.advance();
        }
        Ok(TokenType::Whitespace)
    }

    fn do_number(&mut self) -> NxResult<TokenType> {
        while self.cursor.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.cursor.advance();
        }
        if self.cursor.peek() == Some('.') {
            self.cursor.advance();
            if !self.cursor.peek().is_some_and(|c| c.is_ascii_digit()) {
                return err_at(
                    self.cursor.span_from_mark(),
                    "expected digit after decimal point",
                );
            }
            while self.cursor.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.cursor.advance();
            }
            Ok(TokenType::FloatLiteral)
        } else {
            Ok(TokenType::IntLiteral)
        }
    }

    fn do_identifier(&mut self) -> NxResult<TokenType> {
        while self
            .cursor
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            self.cursor.advance();
        }
        Ok(TokenType::Identifier)
    }
}
