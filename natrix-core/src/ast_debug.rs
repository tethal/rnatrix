use crate::ast::{Expr, ExprKind};
use crate::src::{Sources, Span};
use std::fmt::{self, Debug, Formatter};

/// Wrapper for displaying AST nodes with optional source context.
pub struct AstDebug<'a> {
    expr: &'a Expr,
    sources: Option<&'a Sources>,
    indent: usize,
}

impl<'a> AstDebug<'a> {
    pub fn new(expr: &'a Expr) -> Self {
        Self {
            expr,
            sources: None,
            indent: 0,
        }
    }

    pub fn with_sources(expr: &'a Expr, sources: &'a Sources) -> Self {
        Self {
            expr,
            sources: Some(sources),
            indent: 0,
        }
    }

    fn indented(&self, expr: &'a Expr) -> Self {
        Self {
            expr,
            sources: self.sources,
            indent: self.indent + 2,
        }
    }

    fn fmt_span(&self, f: &mut Formatter<'_>, span: Span) -> fmt::Result {
        write!(f, " ")?;
        if let Some(sources) = self.sources {
            span.debug_with(sources).fmt(f)
        } else {
            span.fmt(f)
        }
    }

    fn fmt_begin_header(&self, f: &mut Formatter<'_>, name: &str) -> fmt::Result {
        let indent = " ".repeat(self.indent);
        write!(f, "{}{}", indent, name)
    }

    fn fmt_end_header(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(span) = self.expr.span {
            self.fmt_span(f, span)?;
        }
        writeln!(f)
    }

    fn fmt_header(&self, f: &mut Formatter<'_>, name: &str) -> fmt::Result {
        self.fmt_begin_header(f, name)?;
        self.fmt_end_header(f)
    }

    fn fmt_header_with_value<T: Debug>(
        &self,
        f: &mut Formatter<'_>,
        name: &str,
        value: T,
    ) -> fmt::Result {
        self.fmt_begin_header(f, name)?;
        write!(f, "({:?})", value)?;
        self.fmt_end_header(f)
    }

    fn fmt_property_with_span<T: Debug>(
        &self,
        f: &mut Formatter<'_>,
        name: &str,
        value: T,
        span: Span,
    ) -> fmt::Result {
        let indent = " ".repeat(self.indent);
        write!(f, "{}  {}: {:?}", indent, name, value)?;
        self.fmt_span(f, span)?;
        writeln!(f)
    }

    fn fmt_child(&self, f: &mut Formatter<'_>, expr: &Expr) -> fmt::Result {
        self.indented(expr).fmt(f)
    }
}

impl Debug for AstDebug<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.expr.kind {
            ExprKind::IntLiteral(value) => self.fmt_header_with_value(f, "IntLiteral", value),
            ExprKind::Paren(inner) => {
                self.fmt_header(f, "Paren")?;
                self.fmt_child(f, inner)
            }
            ExprKind::Unary { op, op_span, expr } => {
                self.fmt_header(f, "Unary")?;
                self.fmt_property_with_span(f, "op", *op, *op_span)?;
                self.fmt_child(f, expr)
            }
            ExprKind::Binary {
                op,
                op_span,
                left,
                right,
            } => {
                self.fmt_header(f, "Binary")?;
                self.fmt_property_with_span(f, "op", *op, *op_span)?;
                self.fmt_child(f, left)?;
                self.fmt_child(f, right)
            }
        }
    }
}
