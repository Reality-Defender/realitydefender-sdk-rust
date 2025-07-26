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
    #[error("{0}")]
    Unauthorized(String),

    /// Resource not found
    #[error("Resource not found")]
    NotFound,

    /// Server error from the API
    #[error("API error: {0}")]
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

#[cfg(test)]
mod tests {
    use crate::Error;
    use std::io;

    #[test]
    fn test_error_display() {
        // Test display implementation for each error variant
        let errors = [
            (
                Error::InvalidConfig("missing api key".to_string()),
                "Invalid configuration: missing api key",
            ),
            (
                Error::Unauthorized("Authentication failed: Invalid API key".to_string()),
                "Authentication failed: Invalid API key",
            ),
            (Error::NotFound, "Resource not found"),
            (
                Error::ServerError("internal error".to_string()),
                "API error: internal error",
            ),
            (
                Error::InvalidFile("file not found".to_string()),
                "Invalid file: file not found",
            ),
            (
                Error::UploadFailed("connection error".to_string()),
                "Upload failed: connection error",
            ),
            (
                Error::InvalidRequest("missing parameter".to_string()),
                "Invalid request: missing parameter",
            ),
            (
                Error::InvalidData("malformed json".to_string()),
                "Invalid data format: malformed json",
            ),
            (
                Error::UnknownError("unexpected error".to_string()),
                "Unknown error: unexpected error",
            ),
        ];

        for (error, expected_message) in errors {
            assert_eq!(error.to_string(), expected_message);
        }
    }

    #[test]
    fn test_error_from_io_error() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let error: Error = io_error.into();

        match error {
            Error::IOError(_) => {} // Success
            _ => panic!("Expected IOError variant"),
        }
    }

    #[test]
    fn test_error_from_json_error() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let error: Error = json_error.into();

        match error {
            Error::JsonError(_) => {} // Success
            _ => panic!("Expected JsonError variant"),
        }
    }

    #[test]
    fn test_result_type() {
        // Test the Result type alias
        fn returns_result_success() -> crate::Result<String> {
            Ok("success".to_string())
        }

        fn returns_result_error() -> crate::Result<String> {
            Err(Error::InvalidConfig("test error".to_string()))
        }

        let success = returns_result_success();
        assert!(success.is_ok());
        assert_eq!(success.unwrap(), "success");

        let error = returns_result_error();
        assert!(error.is_err());
        match error.unwrap_err() {
            Error::InvalidConfig(msg) => assert_eq!(msg, "test error"),
            _ => panic!("Wrong error type"),
        }
    }
}
