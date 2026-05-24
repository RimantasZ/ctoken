use thiserror::Error;

#[derive(Error, Debug)]
pub enum CtokenError {
    #[error("{0}")]
    Usage(String),
    #[error("{0}")]
    Runtime(#[from] anyhow::Error),
}

impl CtokenError {
    pub fn exit_code(&self) -> i32 {
        match self {
            CtokenError::Usage(_) => 2,
            CtokenError::Runtime(_) => 1,
        }
    }

    pub fn usage(msg: impl Into<String>) -> Self {
        CtokenError::Usage(msg.into())
    }
}

pub type Result<T> = std::result::Result<T, CtokenError>;
