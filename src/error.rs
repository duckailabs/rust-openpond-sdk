use thiserror::Error;

#[derive(Error, Debug)]
pub enum OpenPondError {
    #[error("API error: {status} - {message}")]
    ApiError {
        status: u16,
        message: String,
    },
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("HTTP client error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("SSE client error")]
    SSEError,
}

impl From<eventsource_client::Error> for OpenPondError {
    fn from(_: eventsource_client::Error) -> Self {
        OpenPondError::SSEError
    }
}

pub type Result<T> = std::result::Result<T, OpenPondError>; 