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

pub const KEYWORDS: &[(&str, TokenType)] = &[
    ("true", TokenType::KwTrue),
    ("false", TokenType::KwFalse),
    ("null", TokenType::KwNull),
];
