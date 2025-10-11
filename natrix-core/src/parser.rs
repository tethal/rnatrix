//! Recursive descent parser for expressions.
//!
//! # Span Invariant
//!
//! All AST nodes constructed by this parser have valid source spans (`span` field is `Some`).
//! Parser code may safely `.unwrap()` spans on parsed expressions - a `None` indicates a parser bug.

use crate::ast::{BinaryOp, Expr, ExprKind, UnaryOp};
use crate::error::{NxError, NxResult};
use crate::src::{Source, Span};
use crate::token::{Token, TokenType, Tokenizer};
use std::str::FromStr;

pub type ParseResult = NxResult<Box<Expr>>;

pub fn parse(source: &Source) -> ParseResult {
    let mut parser = Parser::new(source)?;
    let result = parser.expr()?;
    if parser.tt() != TokenType::Eof {
        parser.err(format!("unexpected token: {:?}", parser.tt()))
    } else {
        Ok(result)
    }
}

struct Parser<'src> {
    tokenizer: Tokenizer<'src>,
    current_token: Token,
}

impl<'src> Parser<'src> {
    fn new(source: &'src Source) -> NxResult<Self> {
        let mut tokenizer = Tokenizer::new(source);
        let current_token = tokenizer.next_token()?;
        Ok(Parser {
            tokenizer,
            current_token,
        })
    }

    fn expr(&mut self) -> ParseResult {
        self.additive()
    }

    fn additive(&mut self) -> ParseResult {
        let mut left = self.multiplicative()?;
        loop {
            let op = match self.tt() {
                TokenType::Plus => BinaryOp::Add,
                TokenType::Minus => BinaryOp::Sub,
                _ => return Ok(left),
            };
            let op_span = self.consume()?.span;
            let right = self.multiplicative()?;
            let span = left.span.unwrap().extend_to(right.span.unwrap());
            left = Expr::boxed(
                ExprKind::Binary {
                    op,
                    op_span,
                    left,
                    right,
                },
                span,
            )
        }
    }

    fn multiplicative(&mut self) -> ParseResult {
        let mut left = self.unary()?;
        loop {
            let op = match self.tt() {
                TokenType::Star => BinaryOp::Mul,
                TokenType::Slash => BinaryOp::Div,
                _ => return Ok(left),
            };
            let op_span = self.consume()?.span;
            let right = self.unary()?;
            let span = left.span.unwrap().extend_to(right.span.unwrap());
            left = Expr::boxed(
                ExprKind::Binary {
                    op,
                    op_span,
                    left,
                    right,
                },
                span,
            )
        }
    }

    fn unary(&mut self) -> ParseResult {
        if self.tt() == TokenType::Minus {
            let op_span = self.consume()?.span;
            let expr = self.primary()?;
            let span = op_span.extend_to(expr.span.unwrap());
            Ok(Expr::boxed(
                ExprKind::Unary {
                    op: UnaryOp::Neg,
                    op_span,
                    expr,
                },
                span,
            ))
        } else {
            self.primary()
        }
    }

    fn primary(&mut self) -> ParseResult {
        match self.tt() {
            TokenType::IntLiteral => {
                let span = self.span();
                let value = i64::from_str(self.lexeme()).map_err(|e| self.error(e.to_string()))?;
                self.consume()?;
                Ok(Expr::boxed(ExprKind::IntLiteral(value), span))
            }
            TokenType::FloatLiteral => {
                let span = self.span();
                let value = f64::from_str(self.lexeme()).map_err(|e| self.error(e.to_string()))?;
                self.consume()?;
                Ok(Expr::boxed(ExprKind::FloatLiteral(value), span))
            }
            TokenType::KwTrue | TokenType::KwFalse => Ok(Expr::boxed(
                ExprKind::BoolLiteral(self.tt() == TokenType::KwTrue),
                self.consume()?.span,
            )),
            TokenType::KwNull => Ok(Expr::boxed(ExprKind::NullLiteral, self.consume()?.span)),
            TokenType::LParen => {
                let span = self.consume()?.span;
                let e = self.expr()?;
                let span = span.extend_to(self.expect(TokenType::RParen)?.span);
                Ok(Expr::boxed(ExprKind::Paren(e), span))
            }
            tt => self.err(format!("expected expression, not {:?}", tt)),
        }
    }

    fn expect(&mut self, tt: TokenType) -> NxResult<Token> {
        if self.tt() == tt {
            self.consume()
        } else {
            self.err(format!("expected {:?}, not {:?}", tt, self.tt()))
        }
    }

    fn consume(&mut self) -> NxResult<Token> {
        let token = self.current_token;
        self.current_token = self.tokenizer.next_token()?;
        Ok(token)
    }

    fn tt(&self) -> TokenType {
        self.current_token.tt
    }

    fn span(&self) -> Span {
        self.current_token.span
    }

    fn lexeme(&self) -> &str {
        self.tokenizer.lexeme(&self.current_token)
    }

    fn err<T>(&self, message: impl Into<String>) -> NxResult<T> {
        Err(self.error(message))
    }

    fn error(&self, message: impl Into<String>) -> NxError {
        NxError {
            message: message.into(),
            span: Some(self.current_token.span),
        }
    }
}
