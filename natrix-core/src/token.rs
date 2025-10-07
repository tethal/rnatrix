use crate::src::{Cursor, Source, Span};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TokenType {
    Eof,
    Error,
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

    pub fn next_token(&mut self) -> Token {
        loop {
            let start = self.cursor.offset();
            let mut tt = self.parse_token_type();
            let span = self.cursor.span_from(start);
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
                return token;
            }
        }
    }

    pub fn lexeme(&self, token: &Token) -> &str {
        self.cursor.lexeme(token.span)
    }

    fn parse_token_type(&mut self) -> TokenType {
        match self.cursor.advance() {
            Some(c) if c.is_whitespace() => self.do_whitespace(),
            Some(c) if c.is_ascii_digit() => self.do_number(),
            Some(c) if c.is_ascii_alphabetic() || c == '_' => self.do_identifier(),
            Some('(') => TokenType::LParen,
            Some(')') => TokenType::RParen,
            Some('+') => TokenType::Plus,
            Some('-') => TokenType::Minus,
            Some('*') => TokenType::Star,
            Some('/') => TokenType::Slash,
            Some(_) => TokenType::Error,
            None => TokenType::Eof,
        }
    }

    fn do_whitespace(&mut self) -> TokenType {
        while self.cursor.peek().is_some_and(|c| c.is_whitespace()) {
            self.cursor.advance();
        }
        TokenType::Whitespace
    }

    fn do_number(&mut self) -> TokenType {
        while self.cursor.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.cursor.advance();
        }
        if self.cursor.peek() == Some('.') {
            self.cursor.advance();
            if !self.cursor.peek().is_some_and(|c| c.is_ascii_digit()) {
                return TokenType::Error;
            }
            while self.cursor.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.cursor.advance();
            }
            TokenType::FloatLiteral
        } else {
            TokenType::IntLiteral
        }
    }

    fn do_identifier(&mut self) -> TokenType {
        while self
            .cursor
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            self.cursor.advance();
        }
        TokenType::Identifier
    }
}
