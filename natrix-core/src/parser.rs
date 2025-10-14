use crate::ast::{BinaryOp, Expr, ExprKind, FunDecl, Param, Program, Stmt, StmtKind, UnaryOp};
use crate::ctx::CompilerContext;
use crate::error::{err_at, NxError, NxResult};
use crate::src::{SourceId, Span};
use crate::token::{Token, TokenType, Tokenizer};
use std::rc::Rc;
use std::str::FromStr;

pub type ParseResult<T> = NxResult<T>;

pub fn parse(ctx: &mut CompilerContext, source_id: SourceId) -> ParseResult<Program> {
    let mut parser = Parser::new(ctx, source_id)?;
    let start_span = parser.span();
    let mut fun_decls = Vec::new();
    while parser.tt() != TokenType::Eof {
        fun_decls.push(parser.fun_decl()?);
    }
    Ok(Program::new(fun_decls, start_span.extend_to(parser.span())))
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

    fn fun_decl(&mut self) -> NxResult<FunDecl> {
        self.expect(TokenType::KwFun)?;
        let name_span = self.span();
        let name = self.expect(TokenType::Identifier)?.name.unwrap();
        let params = self.params()?;
        let body = self.block()?;
        Ok(FunDecl::new(name, name_span, params, body))
    }

    fn params(&mut self) -> NxResult<Vec<Param>> {
        let mut params = Vec::new();
        self.expect(TokenType::LParen)?;
        if self.tt() != TokenType::RParen {
            params.push(self.param()?);
            while self.tt() == TokenType::Comma {
                self.consume()?;
                params.push(self.param()?);
            }
        };
        self.expect(TokenType::RParen)?;
        Ok(params)
    }

    fn param(&mut self) -> NxResult<Param> {
        let name_span = self.span();
        let name = self.expect(TokenType::Identifier)?.name.unwrap();
        // match(Kind.COLON);
        // TypeNode type = type();
        Ok(Param::new(name, name_span))
    }

    fn block(&mut self) -> ParseResult<Stmt> {
        let mut stmts = Vec::new();
        let start_span = self.expect(TokenType::LBrace)?.span;
        while self.tt() != TokenType::RBrace {
            if self.tt() == TokenType::KwVar {
                stmts.push(self.var_decl()?);
            } else {
                stmts.push(self.stmt()?);
            }
        }
        let end_span = self.expect(TokenType::RBrace)?.span;
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
        match self.tt() {
            TokenType::LBrace => self.block(),
            TokenType::KwBreak => {
                let span = self.consume()?.span;
                let span = span.extend_to(self.expect(TokenType::Semicolon)?.span);
                Ok(Stmt::new(StmtKind::Break, span))
            }
            TokenType::KwContinue => {
                let span = self.consume()?.span;
                let span = span.extend_to(self.expect(TokenType::Semicolon)?.span);
                Ok(Stmt::new(StmtKind::Continue, span))
            }
            TokenType::KwIf => {
                let start_span = self.consume()?.span;
                self.expect(TokenType::LParen)?;
                let cond = self.expr()?;
                self.expect(TokenType::RParen)?;
                let then_body = self.stmt()?;
                let else_body = if self.tt() == TokenType::KwElse {
                    self.consume()?;
                    Some(self.stmt()?)
                } else {
                    None
                };
                let span = start_span.extend_to(else_body.as_ref().unwrap_or(&then_body).span);
                Ok(Stmt::new(
                    StmtKind::If {
                        cond,
                        then_body: Box::new(then_body),
                        else_body: else_body.map(|body| Box::new(body)),
                    },
                    span,
                ))
            }
            TokenType::KwReturn => {
                let start_span = self.consume()?.span;
                let expr = if self.tt() != TokenType::Semicolon {
                    Some(self.expr()?)
                } else {
                    None
                };
                let end_span = self.expect(TokenType::Semicolon)?.span;
                Ok(Stmt::new(
                    StmtKind::Return(expr),
                    start_span.extend_to(end_span),
                ))
            }
            TokenType::KwWhile => {
                let start_span = self.consume()?.span;
                self.expect(TokenType::LParen)?;
                let cond = self.expr()?;
                self.expect(TokenType::RParen)?;
                let body = self.stmt()?;
                let span = start_span.extend_to(body.span);
                Ok(Stmt::new(
                    StmtKind::While {
                        cond,
                        body: Box::new(body),
                    },
                    span,
                ))
            }
            _ => {
                let expr = self.expr()?;
                if self.tt() == TokenType::Assign {
                    self.consume()?;
                    if !is_lvalue(&expr) {
                        err_at(expr.span, "expected lvalue on the left side of assignment")
                    } else {
                        let right = self.expr()?;
                        self.expect(TokenType::Semicolon)?;
                        let span = expr.span.extend_to(right.span);
                        Ok(Stmt::new(StmtKind::Assign { left: expr, right }, span))
                    }
                } else {
                    self.expect(TokenType::Semicolon)?;
                    let span = expr.span;
                    Ok(Stmt::new(StmtKind::Expr(expr), span))
                }
            }
        }
    }

    fn expr(&mut self) -> ParseResult<Expr> {
        self.logic_or()
    }

    fn logic_or(&mut self) -> ParseResult<Expr> {
        let mut left = self.logic_and()?;
        while self.tt() == TokenType::Or {
            let op_span = self.consume()?.span;
            let right = self.logic_and()?;
            let span = left.span.extend_to(right.span);
            left = Expr::new(
                ExprKind::LogicalBinary {
                    and: false,
                    op_span,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            )
        }
        Ok(left)
    }

    fn logic_and(&mut self) -> ParseResult<Expr> {
        let mut left = self.equality()?;
        while self.tt() == TokenType::And {
            let op_span = self.consume()?.span;
            let right = self.equality()?;
            let span = left.span.extend_to(right.span);
            left = Expr::new(
                ExprKind::LogicalBinary {
                    and: true,
                    op_span,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            )
        }
        Ok(left)
    }

    fn equality(&mut self) -> ParseResult<Expr> {
        let mut left = self.comparison()?;
        loop {
            let op = match self.tt() {
                TokenType::Eq => BinaryOp::Eq,
                TokenType::Ne => BinaryOp::Ne,
                _ => return Ok(left),
            };
            let op_span = self.consume()?.span;
            let right = self.comparison()?;
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

    fn comparison(&mut self) -> ParseResult<Expr> {
        let mut left = self.additive()?;
        loop {
            let op = match self.tt() {
                TokenType::Lt => BinaryOp::Lt,
                TokenType::Le => BinaryOp::Le,
                TokenType::Gt => BinaryOp::Gt,
                TokenType::Ge => BinaryOp::Ge,
                _ => return Ok(left),
            };
            let op_span = self.consume()?.span;
            let right = self.additive()?;
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
                TokenType::Percent => BinaryOp::Mod,
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
        let op = match self.tt() {
            TokenType::Bang => UnaryOp::Not,
            TokenType::Minus => UnaryOp::Neg,
            _ => return self.primary(),
        };
        let op_span = self.consume()?.span;
        let expr = self.unary()?;
        let span = op_span.extend_to(expr.span);
        Ok(Expr::new(
            ExprKind::Unary {
                op,
                op_span,
                expr: Box::new(expr),
            },
            span,
        ))
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
            TokenType::StringLiteral => {
                let span = self.span();
                let value = Rc::new(decode_string_literal(self.lexeme()));
                self.consume()?;
                Ok(Expr::new(ExprKind::StringLiteral(value), span))
            }
            TokenType::LParen => {
                let span = self.consume()?.span;
                let e = self.expr()?;
                let span = span.extend_to(self.expect(TokenType::RParen)?.span);
                Ok(Expr::new(ExprKind::Paren(Box::new(e)), span))
            }
            TokenType::Identifier => {
                let name_span = self.span();
                let name = self.consume()?.name.unwrap();
                if self.tt() == TokenType::LParen {
                    self.consume()?;
                    let mut args = vec![];
                    if self.tt() != TokenType::RParen {
                        args.push(self.expr()?);
                        while self.tt() == TokenType::Comma {
                            self.consume()?;
                            args.push(self.expr()?);
                        }
                    }
                    let span = name_span.extend_to(self.expect(TokenType::RParen)?.span);
                    Ok(Expr::new(
                        ExprKind::Call {
                            name,
                            name_span,
                            args,
                        },
                        span,
                    ))
                } else {
                    Ok(Expr::new(ExprKind::Var(name), name_span))
                }
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

fn is_lvalue(expr: &Expr) -> bool {
    matches!(expr.kind, ExprKind::Var(_))
}

/// Decodes a string literal by removing surrounding quotes and processing escape sequences.
/// Assumes the tokenizer has already validated the escape sequences.
fn decode_string_literal(lexeme: &str) -> String {
    let mut result = String::new();
    let inner = &lexeme[1..lexeme.len() - 1]; // Remove quotes
    let mut chars = inner.chars();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => result.push('"'),
                Some('\\') => result.push('\\'),
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('0') => result.push('\0'),
                _ => unreachable!("tokenizer should have validated escape sequences"),
            }
        } else {
            result.push(c);
        }
    }

    result
}
