use thiserror::Error;

#[derive(Error, Debug)]
pub enum OpenPondError {
    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("API error: {status} - {message}")]
    ApiError {
        status: u16,
        message: String,
    },

    #[error("SDK not started")]
    NotStarted,

    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Request failed: {0}")]
    NetworkError(String),
}

pub type Result<T> = std::result::Result<T, OpenPondError>; 