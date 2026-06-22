use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DesktopError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Core(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorPayload {
    pub code: &'static str,
    pub message: String,
}

impl Serialize for DesktopError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ErrorPayload {
            code: self.code(),
            message: self.to_string(),
        }
        .serialize(serializer)
    }
}

impl DesktopError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Message(_) => "message",
            Self::Io(_) => "io",
            Self::Core(_) => "core",
        }
    }
}

impl From<String> for DesktopError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for DesktopError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_string())
    }
}

pub type DesktopResult<T> = Result<T, DesktopError>;
