use crate::ast::{Expr, ExprKind, Stmt, StmtKind};
use crate::ctx::{CompilerContext, Name};
use crate::src::Span;
use std::fmt::{self, Debug, Formatter};

#[derive(Copy, Clone)]
pub struct AstFormatter<'a> {
    ctx: Option<&'a CompilerContext>,
    indent: usize,
}

impl<'a> AstFormatter<'a> {
    pub fn new() -> Self {
        Self {
            ctx: None,
            indent: 0,
        }
    }

    pub fn with_context(ctx: &'a CompilerContext) -> Self {
        Self {
            ctx: Some(ctx),
            indent: 0,
        }
    }

    fn indented(&self) -> Self {
        Self {
            ctx: self.ctx,
            indent: self.indent + 2,
        }
    }

    fn span(&self, f: &mut Formatter<'_>, span: Span) -> fmt::Result {
        write!(f, " ")?;
        if let Some(ctx) = self.ctx {
            span.debug_with(&ctx.sources).fmt(f)
        } else {
            span.fmt(f)
        }
    }

    fn name(&self, f: &mut Formatter<'_>, value: Name) -> fmt::Result {
        if let Some(ctx) = self.ctx {
            write!(f, "{:?}", ctx.interner.resolve(value))
        } else {
            write!(f, "{:?}", value)
        }
    }

    fn begin_header(&self, f: &mut Formatter<'_>, name: &str) -> fmt::Result {
        write!(f, "{}{}", self.indent_str(), name)
    }

    fn end_header(&self, f: &mut Formatter<'_>, span: Span) -> fmt::Result {
        self.span(f, span)?;
        writeln!(f)
    }

    fn header(&self, f: &mut Formatter<'_>, name: &str, span: Span) -> fmt::Result {
        self.begin_header(f, name)?;
        self.end_header(f, span)
    }

    fn header_with_value<T: Debug>(
        &self,
        f: &mut Formatter<'_>,
        name: &str,
        span: Span,
        value: T,
    ) -> fmt::Result {
        self.begin_header(f, name)?;
        write!(f, "({:?})", value)?;
        self.end_header(f, span)
    }

    fn header_with_name(
        &self,
        f: &mut Formatter<'_>,
        name: &str,
        span: Span,
        value: Name,
    ) -> fmt::Result {
        self.begin_header(f, name)?;
        write!(f, "(")?;
        self.name(f, value)?;
        write!(f, ")")?;
        self.end_header(f, span)
    }

    fn property_with_span<T: Debug>(
        &self,
        f: &mut Formatter<'_>,
        name: &str,
        value: T,
        span: Span,
    ) -> fmt::Result {
        write!(f, "{}  {}: {:?}", self.indent_str(), name, value)?;
        self.span(f, span)?;
        writeln!(f)
    }

    fn property_name_with_span(
        &self,
        f: &mut Formatter<'_>,
        name: &str,
        value: Name,
        span: Span,
    ) -> fmt::Result {
        write!(f, "{}  {}: ", self.indent_str(), name)?;
        self.name(f, value)?;
        self.span(f, span)?;
        writeln!(f)
    }

    fn indent_str(&self) -> String {
        " ".repeat(self.indent)
    }

    fn expr(&self, f: &mut Formatter<'_>, expr: &Expr) -> fmt::Result {
        ExprDebug {
            fmt: self.indented(),
            expr,
        }
        .fmt(f)
    }

    fn stmt(&self, f: &mut Formatter<'_>, stmt: &Stmt) -> fmt::Result {
        StmtDebug {
            fmt: self.indented(),
            stmt,
        }
        .fmt(f)
    }
}

pub struct ExprDebug<'a> {
    fmt: AstFormatter<'a>,
    expr: &'a Expr,
}

impl<'a> ExprDebug<'a> {
    pub fn new(expr: &'a Expr) -> Self {
        Self {
            fmt: AstFormatter::new(),
            expr,
        }
    }

    pub fn with_context(expr: &'a Expr, ctx: &'a CompilerContext) -> Self {
        Self {
            fmt: AstFormatter::with_context(ctx),
            expr,
        }
    }
}

impl Debug for ExprDebug<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let span = self.expr.span;
        match &self.expr.kind {
            ExprKind::IntLiteral(value) => self.fmt.header_with_value(f, "IntLiteral", span, value),
            ExprKind::FloatLiteral(value) => {
                self.fmt.header_with_value(f, "FloatLiteral", span, value)
            }
            ExprKind::BoolLiteral(value) => {
                self.fmt.header_with_value(f, "BoolLiteral", span, value)
            }
            ExprKind::NullLiteral => self.fmt.header(f, "NullLiteral", span),
            ExprKind::Paren(inner) => {
                self.fmt.header(f, "Paren", span)?;
                self.fmt.expr(f, inner)
            }
            ExprKind::Unary { op, op_span, expr } => {
                self.fmt.header(f, "Unary", span)?;
                self.fmt.property_with_span(f, "op", *op, *op_span)?;
                self.fmt.expr(f, expr)
            }
            ExprKind::Binary {
                op,
                op_span,
                left,
                right,
            } => {
                self.fmt.header(f, "Binary", span)?;
                self.fmt.property_with_span(f, "op", *op, *op_span)?;
                self.fmt.expr(f, left)?;
                self.fmt.expr(f, right)
            }
            ExprKind::Var(name) => self.fmt.header_with_name(f, "Var", span, *name),
        }
    }
}

pub struct StmtDebug<'a> {
    fmt: AstFormatter<'a>,
    stmt: &'a Stmt,
}

impl<'a> StmtDebug<'a> {
    pub fn new(stmt: &'a Stmt) -> Self {
        Self {
            fmt: AstFormatter::new(),
            stmt,
        }
    }

    pub fn with_context(stmt: &'a Stmt, ctx: &'a CompilerContext) -> Self {
        Self {
            fmt: AstFormatter::with_context(ctx),
            stmt,
        }
    }
}

impl Debug for StmtDebug<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let span = self.stmt.span;
        match &self.stmt.kind {
            StmtKind::Assign { left, right } => {
                self.fmt.header(f, "Assign", span)?;
                self.fmt.expr(f, left)?;
                self.fmt.expr(f, right)
            }
            StmtKind::Block(stmts) => {
                self.fmt.header(f, "Block", span)?;
                for stmt in stmts {
                    self.fmt.stmt(f, stmt)?;
                }
                Ok(())
            }
            StmtKind::Expr(expr) => {
                self.fmt.header(f, "Expr", span)?;
                self.fmt.expr(f, expr)
            }
            StmtKind::VarDecl {
                name,
                name_span,
                init,
            } => {
                self.fmt.header(f, "VarDecl", span)?;
                self.fmt
                    .property_name_with_span(f, "name", *name, *name_span)?;
                self.fmt.expr(f, init)
            }
        }
    }
}
