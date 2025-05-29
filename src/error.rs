use std::io;
use thiserror::Error;

/// Custom result type for the SDK
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for the Reality Defender SDK
#[derive(Error, Debug)]
pub enum Error {
    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Authentication failure
    #[error("Authentication failed: Invalid API key")]
    Unauthorized,

    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Server error from the API
    #[error("Server error: {0}")]
    ServerError(String),

    /// Invalid file
    #[error("Invalid file: {0}")]
    InvalidFile(String),

    /// Upload failed
    #[error("Upload failed: {0}")]
    UploadFailed(String),

    /// Invalid request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Invalid data format
    #[error("Invalid data format: {0}")]
    InvalidData(String),

    /// HTTP request error
    #[error("HTTP request error: {0}")]
    RequestError(#[from] reqwest::Error),

    /// IO error
    #[error("IO error: {0}")]
    IOError(#[from] io::Error),

    /// JSON parsing error
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Unknown error
    #[error("Unknown error: {0}")]
    UnknownError(String),
}
