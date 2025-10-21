#[derive(Debug, Clone)]
pub struct NxError {
    pub message: Box<str>,
}

pub type NxResult<T> = Result<T, NxError>;

impl NxError {
    pub fn new(msg: impl Into<Box<str>>) -> Self {
        NxError {
            message: msg.into(),
        }
    }
}

pub fn nx_err<T>(message: impl Into<Box<str>>) -> NxResult<T> {
    Err(NxError::new(message))
}

pub fn nx_error(message: impl Into<Box<str>>) -> NxError {
    NxError::new(message)
}
