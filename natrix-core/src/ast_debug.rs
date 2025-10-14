use crate::ast::{Expr, ExprKind, FunDecl, Param, Program, Stmt, StmtKind};
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
}

macro_rules! impl_ast_debug {
    ($name:ident as $field:ident => $debug_name:ident) => {
        struct $debug_name<'a> {
            fmt: AstFormatter<'a>,
            $field: &'a $name,
        }

        impl<'a> $debug_name<'a> {
            pub fn new($field: &'a $name) -> Self {
                Self {
                    fmt: AstFormatter::new(),
                    $field,
                }
            }

            pub fn with_context($field: &'a $name, ctx: &'a CompilerContext) -> Self {
                Self {
                    fmt: AstFormatter::with_context(ctx),
                    $field,
                }
            }
        }

        impl $name {
            pub fn debug_with<'a>(&'a self, ctx: &'a CompilerContext) -> impl Debug + 'a {
                $debug_name::with_context(self, ctx)
            }
        }

        impl Debug for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                $debug_name::new(self).fmt(f)
            }
        }

        impl<'a> AstFormatter<'a> {
            #[allow(dead_code)]
            fn $field(&self, f: &mut Formatter<'_>, $field: &$name) -> fmt::Result {
                $debug_name {
                    fmt: self.indented(),
                    $field,
                }
                .fmt(f)
            }
        }
    };
}

impl_ast_debug!(Program as program => ProgramDebug);

impl<'a> Debug for ProgramDebug<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt.header(f, "Program", self.program.span)?;
        for d in self.program.decls.iter() {
            self.fmt.fun_decl(f, d)?
        }
        Ok(())
    }
}

impl_ast_debug!(FunDecl as fun_decl => FunDeclDebug);

impl<'a> Debug for FunDeclDebug<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt
            .header_with_name(f, "FunDecl", self.fun_decl.name_span, self.fun_decl.name)?;
        for p in self.fun_decl.params.iter() {
            self.fmt.param(f, p)?
        }
        self.fmt.stmt(f, &self.fun_decl.body)
    }
}

impl_ast_debug!(Param as param => ParamDebug);

impl<'a> Debug for ParamDebug<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt
            .header_with_name(f, "Param", self.param.name_span, self.param.name)
    }
}

impl_ast_debug!(Stmt as stmt => StmtDebug);

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
            StmtKind::Print(expr) => {
                self.fmt.header(f, "Print", span)?;
                self.fmt.expr(f, expr)
            }
            StmtKind::Return(expr) => {
                self.fmt.header(f, "Return", span)?;
                if let Some(expr) = expr {
                    self.fmt.expr(f, expr)
                } else {
                    Ok(())
                }
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

impl_ast_debug!(Expr as expr => ExprDebug);

impl Debug for ExprDebug<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let span = self.expr.span;
        match &self.expr.kind {
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
            ExprKind::BoolLiteral(value) => {
                self.fmt.header_with_value(f, "BoolLiteral", span, value)
            }
            ExprKind::Call {
                name,
                name_span,
                args,
            } => {
                self.fmt.header_with_name(f, "Call", *name_span, *name)?;
                for a in args {
                    self.fmt.expr(f, a)?;
                }
                Ok(())
            }
            ExprKind::FloatLiteral(value) => {
                self.fmt.header_with_value(f, "FloatLiteral", span, value)
            }
            ExprKind::IntLiteral(value) => self.fmt.header_with_value(f, "IntLiteral", span, value),
            ExprKind::LogicalBinary {
                and,
                op_span,
                left,
                right,
            } => {
                self.fmt.header(f, "LogicalBinary", span)?;
                self.fmt.property_with_span(f, "and", and, *op_span)?;
                self.fmt.expr(f, left)?;
                self.fmt.expr(f, right)
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
            ExprKind::Var(name) => self.fmt.header_with_name(f, "Var", span, *name),
        }
    }
}
