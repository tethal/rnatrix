use crate::src::{Sources, Span};
use natrix_runtime::error::{NxError, NxResult};
use std::fmt::{Debug, Display};

pub type SourceResult<T> = Result<T, SourceError>;

#[derive(Debug)]
pub struct SourceError {
    pub message: Box<str>,
    pub span: Span,
}

impl SourceError {
    pub fn display_with<'a>(&'a self, sources: &'a Sources) -> ErrorDisplay<'a> {
        ErrorDisplay::new(sources, &self.message, Some(self.span))
    }
}

pub trait AttachErrSpan {
    type Output;
    fn err_at(self, span: Span) -> Self::Output;
}

impl AttachErrSpan for NxError {
    type Output = SourceError;
    fn err_at(self, span: Span) -> SourceError {
        SourceError {
            message: self.message,
            span,
        }
    }
}

impl<T> AttachErrSpan for NxResult<T> {
    type Output = SourceResult<T>;
    fn err_at(self, span: Span) -> SourceResult<T> {
        self.map_err(|e| e.err_at(span))
    }
}

pub struct ErrorDisplay<'a> {
    sources: &'a Sources,
    message: &'a str,
    span: Option<Span>,
}

impl<'a> ErrorDisplay<'a> {
    fn new(sources: &'a Sources, message: &'a str, span: Option<Span>) -> Self {
        Self {
            sources,
            message,
            span,
        }
    }
}

impl Display for ErrorDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(span) = self.span {
            let src = self.sources.get_by_id(span.source_id());
            let (sline, scol) = span.start_pos(self.sources);
            let (eline, ecol) = span.end_pos(self.sources);
            let text = src.get_line(sline);
            write!(
                f,
                "{}:{}:{}: error: {}",
                src.name(),
                sline,
                scol,
                self.message
            )?;
            if !text.trim().is_empty() {
                let cnt: usize = if eline == sline {
                    if ecol <= scol { 1 } else { ecol - scol }
                } else {
                    text.chars().count() - scol + 1
                };
                write!(f, "\n{}\n{}{}", text, " ".repeat(scol - 1), "^".repeat(cnt))
            } else {
                Ok(())
            }
        } else {
            write!(f, "error: {}", self.message)
        }
    }
}

pub fn err_at<T>(span: Span, message: impl Into<Box<str>>) -> SourceResult<T> {
    Err(error_at(span, message))
}

pub fn error_at(span: Span, message: impl Into<Box<str>>) -> SourceError {
    SourceError {
        message: message.into(),
        span,
    }
}
