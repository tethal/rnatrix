#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TokenType {
    Eof,
    Whitespace,
    Comment,
    KwTrue,
    KwFalse,
    KwNull,
    KwVar,
    Identifier,
    IntLiteral,
    FloatLiteral,
    LParen,
    RParen,
    Plus,
    Minus,
    Star,
    Slash,
    Semicolon,
    Assign,
}

pub const KEYWORDS: &[(&str, TokenType)] = &[
    ("true", TokenType::KwTrue),
    ("false", TokenType::KwFalse),
    ("null", TokenType::KwNull),
    ("var", TokenType::KwVar),
];
