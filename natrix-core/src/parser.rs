use crate::ast::{BinaryOp, Expr, ExprKind, Stmt, StmtKind, UnaryOp};
use crate::ctx::{CompilerContext, Name};
use crate::error::{err_at, NxError, NxResult};
use crate::src::{SourceId, Span};
use crate::token::{Token, TokenType, Tokenizer};
use std::str::FromStr;

pub type ParseResult<T> = NxResult<T>;

pub fn parse(ctx: &mut CompilerContext, source_id: SourceId) -> ParseResult<Stmt> {
    let mut parser = Parser::new(ctx, source_id)?;
    let result = parser.block()?;
    assert_eq!(parser.tt(), TokenType::Eof);
    Ok(result)
}

struct Parser<'ctx> {
    tokenizer: Tokenizer<'ctx>,
    current_token: Token,
}

impl<'ctx> Parser<'ctx> {
    fn new(ctx: &'ctx mut CompilerContext, source_id: SourceId) -> NxResult<Self> {
        let mut tokenizer = Tokenizer::new(ctx, source_id);
        let current_token = tokenizer.next_token()?;
        Ok(Parser {
            tokenizer,
            current_token,
        })
    }

    fn block(&mut self) -> ParseResult<Stmt> {
        let mut stmts = Vec::new();
        let start_span = self.span();
        //         match(Kind.LBRACE);
        //         while (token.kind() != Kind.RBRACE) {
        while self.tt() != TokenType::Eof {
            if self.tt() == TokenType::KwVar {
                stmts.push(self.var_decl()?);
            } else {
                stmts.push(self.stmt()?);
            }
        }
        let end_span = self.span();
        //         match(Kind.RBRACE);
        Ok(Stmt::new(
            StmtKind::Block(stmts),
            start_span.extend_to(end_span),
        ))
    }

    fn var_decl(&mut self) -> ParseResult<Stmt> {
        let start_span = self.expect(TokenType::KwVar)?.span;
        let name_token = self.expect(TokenType::Identifier)?;
        //         match(Kind.COLON);
        //         TypeNode type = type();
        self.expect(TokenType::Assign)?;
        let init = self.expr()?;
        let end_span = self.expect(TokenType::Semicolon)?.span;
        Ok(Stmt::new(
            StmtKind::VarDecl {
                name: name_token.name.unwrap(),
                name_span: name_token.span,
                init,
            },
            start_span.extend_to(end_span),
        ))
    }

    fn stmt(&mut self) -> ParseResult<Stmt> {
        let expr = self.expr()?;
        if self.tt() == TokenType::Assign {
            self.consume()?.span;
            let right = self.expr()?;
            self.expect(TokenType::Semicolon)?;
            if !expr.is_lvalue() {
                err_at(expr.span, "expected lvalue on the left side of assignment")
            } else {
                let span = expr.span.extend_to(right.span);
                Ok(Stmt::new(StmtKind::Assign { left: expr, right }, span))
            }
        } else {
            self.expect(TokenType::Semicolon)?;
            let span = expr.span;
            Ok(Stmt::new(StmtKind::Expr(expr), span))
        }
    }

    fn expr(&mut self) -> ParseResult<Expr> {
        self.additive()
    }

    fn additive(&mut self) -> ParseResult<Expr> {
        let mut left = self.multiplicative()?;
        loop {
            let op = match self.tt() {
                TokenType::Plus => BinaryOp::Add,
                TokenType::Minus => BinaryOp::Sub,
                _ => return Ok(left),
            };
            let op_span = self.consume()?.span;
            let right = self.multiplicative()?;
            let span = left.span.extend_to(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op,
                    op_span,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            )
        }
    }

    fn multiplicative(&mut self) -> ParseResult<Expr> {
        let mut left = self.unary()?;
        loop {
            let op = match self.tt() {
                TokenType::Star => BinaryOp::Mul,
                TokenType::Slash => BinaryOp::Div,
                _ => return Ok(left),
            };
            let op_span = self.consume()?.span;
            let right = self.unary()?;
            let span = left.span.extend_to(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op,
                    op_span,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            )
        }
    }

    fn unary(&mut self) -> ParseResult<Expr> {
        if self.tt() == TokenType::Minus {
            let op_span = self.consume()?.span;
            let expr = self.primary()?;
            let span = op_span.extend_to(expr.span);
            Ok(Expr::new(
                ExprKind::Unary {
                    op: UnaryOp::Neg,
                    op_span,
                    expr: Box::new(expr),
                },
                span,
            ))
        } else {
            self.primary()
        }
    }

    fn primary(&mut self) -> ParseResult<Expr> {
        match self.tt() {
            TokenType::IntLiteral => {
                let span = self.span();
                let value = i64::from_str(self.lexeme()).map_err(|e| self.error(e.to_string()))?;
                self.consume()?;
                Ok(Expr::new(ExprKind::IntLiteral(value), span))
            }
            TokenType::FloatLiteral => {
                let span = self.span();
                let value = f64::from_str(self.lexeme()).map_err(|e| self.error(e.to_string()))?;
                self.consume()?;
                Ok(Expr::new(ExprKind::FloatLiteral(value), span))
            }
            TokenType::KwTrue | TokenType::KwFalse => Ok(Expr::new(
                ExprKind::BoolLiteral(self.tt() == TokenType::KwTrue),
                self.consume()?.span,
            )),
            TokenType::KwNull => Ok(Expr::new(ExprKind::NullLiteral, self.consume()?.span)),
            TokenType::LParen => {
                let span = self.consume()?.span;
                let e = self.expr()?;
                let span = span.extend_to(self.expect(TokenType::RParen)?.span);
                Ok(Expr::new(ExprKind::Paren(Box::new(e)), span))
            }
            TokenType::Identifier => {
                Ok(Expr::new(ExprKind::Var(self.name()), self.consume()?.span))
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

    fn name(&self) -> Name {
        self.current_token.name.unwrap()
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
