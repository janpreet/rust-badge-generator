use thiserror::Error;

#[derive(Error, Debug)]
pub enum BadgeError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Parse error: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("Environment variable error: {0}")]
    EnvError(#[from] std::env::VarError),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Unknown registry: {0}")]
    UnknownRegistry(String),

    #[error("No download data available")]
    NoDownloads,

    #[error("Invalid header value: {0}")]
    InvalidHeader(String),
}