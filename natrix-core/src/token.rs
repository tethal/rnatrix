use crate::src::{Cursor, Source, Span};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TokenType {
    Eof,
    Error,
    Whitespace,
    Comment,
    IntLiteral,
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
    next_token: Token,
}

impl<'src> Tokenizer<'src> {
    pub fn new(source: &'src Source) -> Tokenizer<'src> {
        let cursor = Cursor::new(source);
        let next_token = Token {
            tt: TokenType::Error,
            span: cursor.span_from(0),
        };
        let mut tokenizer = Tokenizer { cursor, next_token };
        tokenizer.advance();
        tokenizer
    }

    pub fn peek(&self) -> Token {
        self.next_token
    }

    pub fn advance(&mut self) -> Token {
        let next_token = self.next_token;
        loop {
            let start = self.cursor.offset();
            self.next_token = Token {
                tt: self.parse_token_type(),
                span: self.cursor.span_from(start),
            };
            if self.next_token.tt != TokenType::Comment
                && self.next_token.tt != TokenType::Whitespace
            {
                break;
            }
        }
        next_token
    }

    pub fn lexeme(&self, token: &Token) -> &str {
        self.cursor.lexeme(&token.span)
    }

    fn parse_token_type(&mut self) -> TokenType {
        match self.cursor.advance() {
            Some(c) if c.is_whitespace() => self.do_whitespace(),
            Some(c) if c.is_digit(10) => self.do_int_literal(),
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
        while let Some(c) = self.cursor.peek()
            && c.is_whitespace()
        {
            self.cursor.advance();
        }
        TokenType::Whitespace
    }

    fn do_int_literal(&mut self) -> TokenType {
        while let Some(c) = self.cursor.peek()
            && c.is_digit(10)
        {
            self.cursor.advance();
        }
        TokenType::IntLiteral
    }
}
