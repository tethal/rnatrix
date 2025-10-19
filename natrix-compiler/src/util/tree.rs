// Helpers for defining AST/HIR nodes and their debug formatting.

macro_rules! def_formatter {
    ($name:ident) => {
        #[derive(Copy, Clone)]
        pub struct $name<'a> {
            ctx: Option<&'a CompilerContext>,
            indent: usize,
        }

        impl<'a> $name<'a> {
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

            pub fn indented(&self) -> Self {
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

            pub fn name(&self, f: &mut Formatter<'_>, value: Name) -> fmt::Result {
                if let Some(ctx) = self.ctx {
                    write!(f, "{:?}", ctx.interner.resolve(value))
                } else {
                    write!(f, "{:?}", value)
                }
            }

            pub fn begin_header(&self, f: &mut Formatter<'_>, name: &str) -> fmt::Result {
                write!(f, "{}{}", self.indent_str(), name)
            }

            pub fn end_header(&self, f: &mut Formatter<'_>, span: Span) -> fmt::Result {
                self.span(f, span)?;
                writeln!(f)
            }

            pub fn header(&self, f: &mut Formatter<'_>, name: &str, span: Span) -> fmt::Result {
                self.begin_header(f, name)?;
                self.end_header(f, span)
            }

            pub fn header_with_value<T: Debug>(
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

            pub fn header_with_name(
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

            pub fn property_with_span<T: Debug>(
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

            pub fn property_name_with_span(
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

            pub fn indent_str(&self) -> String {
                " ".repeat(self.indent)
            }
        }
    };
}

macro_rules! def_node {
    ($name:ident { $($field_name:ident: $field_type:ty),+ $(,)? }) => {
        pub struct $name {
            $(pub $field_name: $field_type,)+
        }

        impl $name {
            pub fn new($($field_name: $field_type),+) -> Self {
                Self { $($field_name),+ }
            }
        }
    };
}

macro_rules! impl_node_debug {
    ($name:ident as $field:ident => $debug_name:ident $formatter_name:ident) => {
        struct $debug_name<'a> {
            fmt: $formatter_name<'a>,
            $field: &'a $name,
        }

        impl<'a> $debug_name<'a> {
            pub fn new($field: &'a $name) -> Self {
                Self {
                    fmt: $formatter_name::new(),
                    $field,
                }
            }

            pub fn with_context($field: &'a $name, ctx: &'a CompilerContext) -> Self {
                Self {
                    fmt: $formatter_name::with_context(ctx),
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

        impl<'a> $formatter_name<'a> {
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

pub(crate) use def_formatter;
pub(crate) use def_node;
pub(crate) use impl_node_debug;
