use realitydefender::Error;
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
            Error::Unauthorized,
            "Authentication failed: Invalid API key",
        ),
        (
            Error::NotFound("request id".to_string()),
            "Resource not found: request id",
        ),
        (
            Error::ServerError("internal error".to_string()),
            "Server error: internal error",
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
    fn returns_result_success() -> realitydefender::Result<String> {
        Ok("success".to_string())
    }

    fn returns_result_error() -> realitydefender::Result<String> {
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
