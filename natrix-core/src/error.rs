use crate::src::{Sources, Span};
use std::fmt::Display;

pub type NxResult<T> = Result<T, NxError>;

#[derive(Debug)]
pub struct NxError {
    pub message: String,
    pub span: Option<Span>,
}

impl NxError {
    pub fn display_with<'a>(&'a self, sources: &'a Sources) -> ErrorDisplay<'a> {
        ErrorDisplay::new(sources, &self.message, self.span)
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
            let cnt: usize = if eline == sline {
                if ecol <= scol { 1 } else { ecol - scol }
            } else {
                text.chars().count() - scol + 1
            };
            write!(
                f,
                "{}:{}:{}: error: {}\n{}\n{}{}",
                src.name(),
                sline,
                scol,
                self.message,
                text,
                " ".repeat(scol - 1),
                "^".repeat(cnt)
            )
        } else {
            write!(f, "error: {}", self.message)
        }
    }
}

pub fn err<T>(message: impl Into<String>) -> NxResult<T> {
    Err(error(message))
}

pub fn error(message: impl Into<String>) -> NxError {
    NxError {
        message: message.into(),
        span: None,
    }
}

pub fn err_at<T>(span: Span, message: impl Into<String>) -> NxResult<T> {
    Err(error_at(span, message))
}

pub fn error_at(span: Span, message: impl Into<String>) -> NxError {
    NxError {
        message: message.into(),
        span: Some(span),
    }
}
