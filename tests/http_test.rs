use mockito::Matcher;
use realitydefender::{Client, Config, Error, UploadOptions};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[derive(Debug, Serialize, Deserialize)]
struct TestResponse {
    status: String,
    message: String,
}

#[tokio::test]
async fn test_client_new() {
    // Valid configuration
    let config = Config {
        api_key: "test_api_key".to_string(),
        ..Default::default()
    };
    let client = Client::new(config);
    assert!(client.is_ok());

    // Invalid configuration (empty API key)
    let invalid_config = Config {
        api_key: "".to_string(),
        ..Default::default()
    };
    let client = Client::new(invalid_config);
    assert!(client.is_err());
}

#[tokio::test]
async fn test_client_with_custom_url() {
    let server = mockito::Server::new_async().await;

    // Create client with mock server URL
    let config = Config {
        api_key: "test_api_key".to_string(),
        base_url: Some(server.url()),
        ..Default::default()
    };

    let client = Client::new(config);
    assert!(client.is_ok());
    // Cannot directly test get_base_url as it's private to the client
}

#[tokio::test]
async fn test_api_error_handling() {
    let mut server = mockito::Server::new_async().await;

    // Setup mock server for unauthorized error
    let _m = server
        .mock("GET", "/api/media/users/test-request")
        .with_status(401)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error": "Unauthorized access"}"#)
        .create_async()
        .await;

    // Create client with mock server URL
    let config = Config {
        api_key: "invalid_key".to_string(),
        base_url: Some(server.url()),
        ..Default::default()
    };
    let client = Client::new(config).unwrap();

    // Make request that should result in error
    let result = client.get_result("test-request", None).await;

    // Verify error
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::Unauthorized => {} // Expected error
        err => panic!("Unexpected error: {:?}", err),
    }

    // Setup mock server for not found error
    let _m = server
        .mock("GET", "/api/media/users/not-found")
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error": "Resource not found"}"#)
        .create_async()
        .await;

    // Make request that should result in not found error
    let result = client.get_result("not-found", None).await;

    // Verify error
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::NotFound(_) => {} // Expected error
        err => panic!("Unexpected error: {:?}", err),
    }
}

#[tokio::test]
async fn test_upload_file_flow() {
    let mut server = mockito::Server::new_async().await;
    println!("Server URL: {}", server.url());

    // Create a temporary file for testing
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.jpg");
    let mut file = File::create(&file_path).unwrap();
    file.write_all(b"test image data").unwrap();
    println!("Created test file at: {:?}", file_path);

    // Mock the presigned URL request
    let _m1 = server
        .mock("POST", "/api/files/aws-presigned")
        .with_status(200)
        .with_header("content-type", "application/json")
        .match_header("X-API-KEY", "test_api_key")
        .match_body(Matcher::Json(json!({"fileName": "test.jpg"})))
        .with_body(
            json!({
                "code": "success",
                "errno": 0,
                "requestId": "test-request-id",
                "mediaId": "test-media-id",
                "response": {
                    "signedUrl": format!("{}/upload", server.url())
                }
            })
            .to_string(),
        )
        .create_async()
        .await;
    println!("Mocked presigned URL endpoint");

    // Mock the upload endpoint
    let _m2 = server
        .mock("PUT", "/upload")
        .with_status(200)
        .match_header("content-type", "image/jpeg")
        .create_async()
        .await;
    println!("Mocked upload endpoint");

    // Create client with mock server URL
    let config = Config {
        api_key: "test_api_key".to_string(),
        base_url: Some(server.url()),
        ..Default::default()
    };
    let client = Client::new(config).unwrap();
    println!("Created client");

    // Upload file
    let result = client
        .upload(UploadOptions {
            file_path: file_path.to_str().unwrap().to_string(),
            metadata: None,
        })
        .await;

    // Print detailed error if test fails
    if let Err(ref e) = result {
        println!("Upload failed with error: {:?}", e);
    }

    // Verify result
    assert!(result.is_ok(), "Upload failed: {:?}", result.err());
    let upload_result = result.unwrap();
    assert_eq!(upload_result.request_id, "test-request-id");
    assert_eq!(upload_result.media_id, "test-media-id");
    assert!(upload_result.result_url.is_none());
}

#[tokio::test]
async fn test_http_post_request() {
    let mut server = mockito::Server::new_async().await;

    // Setup mock server for POST request
    let _m = server
        .mock("POST", "/api/test-endpoint")
        .with_status(200)
        .with_header("content-type", "application/json")
        .match_header("X-API-KEY", "test_api_key")
        .match_header("Content-Type", "application/json")
        .match_body(Matcher::Json(json!({"key": "value"})))
        .with_body(r#"{"status": "success", "message": "Test completed"}"#)
        .create_async()
        .await;

    // Create client with mock server URL
    let config = Config {
        api_key: "test_api_key".to_string(),
        base_url: Some(server.url()),
        ..Default::default()
    };
    let client = Client::new(config).unwrap();

    // Access the HTTP client's post method indirectly through client
    // We can test this through the get_result or upload methods
    let _test_data = json!({"key": "value"});

    // Use the client through a method that makes a POST request
    let mock_endpoint = server
        .mock("POST", "/api/files/aws-presigned")
        .with_status(200)
        .with_header("content-type", "application/json")
        .match_body(Matcher::Json(json!({"fileName": "test.jpg"})))
        .with_body(
            json!({
                "code": "success",
                "errno": 0,
                "requestId": "test-request-id",
                "mediaId": "test-media-id",
                "response": {
                    "signedUrl": format!("{}/upload", server.url())
                }
            })
            .to_string(),
        )
        .create_async()
        .await;

    let mock_upload = server
        .mock("PUT", "/upload")
        .with_status(200)
        .create_async()
        .await;

    // Create a temporary file for testing
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.jpg");
    let mut file = File::create(&file_path).unwrap();
    file.write_all(b"test image data").unwrap();

    // Upload file to test POST request
    let result = client
        .upload(UploadOptions {
            file_path: file_path.to_str().unwrap().to_string(),
            metadata: None,
        })
        .await;

    assert!(result.is_ok());
    mock_endpoint.assert_async().await;
    mock_upload.assert_async().await;
}

#[tokio::test]
async fn test_server_error_handling() {
    let mut server = mockito::Server::new_async().await;

    // Setup mock server for server error
    let _m = server
        .mock("GET", "/api/media/users/test-server-error")
        .with_status(500)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error": "Internal server error"}"#)
        .create_async()
        .await;

    // Create client with mock server URL
    let config = Config {
        api_key: "test_api_key".to_string(),
        base_url: Some(server.url()),
        ..Default::default()
    };
    let client = Client::new(config).unwrap();

    // Make request that should result in server error
    let result = client.get_result("test-server-error", None).await;

    // Verify error
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::ServerError(_) => {} // Expected error
        err => panic!("Unexpected error: {:?}", err),
    }
}

#[tokio::test]
async fn test_forbidden_error_handling() {
    let mut server = mockito::Server::new_async().await;

    // Setup mock server for forbidden error
    let _m = server
        .mock("GET", "/api/media/users/test-forbidden")
        .with_status(403)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error": "Forbidden access"}"#)
        .create_async()
        .await;

    // Create client with mock server URL
    let config = Config {
        api_key: "test_api_key".to_string(),
        base_url: Some(server.url()),
        ..Default::default()
    };
    let client = Client::new(config).unwrap();

    // Make request that should result in forbidden error
    let result = client.get_result("test-forbidden", None).await;

    // Verify error
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::Unauthorized => {} // Expected error (403 maps to Unauthorized)
        err => panic!("Unexpected error: {:?}", err),
    }
}

#[tokio::test]
async fn test_unknown_error_handling() {
    let mut server = mockito::Server::new_async().await;

    // Setup mock server for unknown error with parseable error message
    let _m = server
        .mock("GET", "/api/media/users/test-unknown-error")
        .with_status(400)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error": "Custom error message"}"#)
        .create_async()
        .await;

    // Create client with mock server URL
    let config = Config {
        api_key: "test_api_key".to_string(),
        base_url: Some(server.url()),
        ..Default::default()
    };
    let client = Client::new(config).unwrap();

    // Make request that should result in unknown error
    let result = client.get_result("test-unknown-error", None).await;

    // Verify error
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::UnknownError(msg) => {
            assert_eq!(msg, "Custom error message");
        }
        err => panic!("Unexpected error: {:?}", err),
    }

    // Setup mock server for unknown error with unparseable response
    let _m2 = server
        .mock("GET", "/api/media/users/test-unknown-error-2")
        .with_status(422)
        .with_header("content-type", "text/plain")
        .with_body("Unparseable error")
        .create_async()
        .await;

    // Make request that should result in unknown error with unparseable body
    let result = client.get_result("test-unknown-error-2", None).await;

    // Verify error
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::UnknownError(msg) => {
            assert_eq!(msg, "Unknown error (HTTP 422 Unprocessable Entity)");
        }
        err => panic!("Unexpected error: {:?}", err),
    }
}

#[tokio::test]
async fn test_upload_with_empty_file() {
    let mut server = mockito::Server::new_async().await;

    // Create a temporary empty file for testing
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("empty.jpg");
    File::create(&file_path).unwrap(); // Create empty file

    // Mock the presigned URL request
    let _m = server
        .mock("POST", "/api/files/aws-presigned")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "code": "success",
                "errno": 0,
                "requestId": "test-request-id",
                "mediaId": "test-media-id",
                "response": {
                    "signedUrl": format!("{}/upload", server.url())
                }
            })
            .to_string(),
        )
        .create_async()
        .await;

    // Create client with mock server URL
    let config = Config {
        api_key: "test_api_key".to_string(),
        base_url: Some(server.url()),
        ..Default::default()
    };
    let client = Client::new(config).unwrap();

    // Upload empty file
    let result = client
        .upload(UploadOptions {
            file_path: file_path.to_str().unwrap().to_string(),
            metadata: None,
        })
        .await;

    // Should fail with InvalidFile error
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::InvalidFile(_) => {} // Expected error
        err => panic!("Unexpected error: {:?}", err),
    }
}

#[tokio::test]
async fn test_upload_failure() {
    let mut server = mockito::Server::new_async().await;

    // Create a temporary file for testing
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.jpg");
    let mut file = File::create(&file_path).unwrap();
    file.write_all(b"test image data").unwrap();

    // Mock the presigned URL request
    let _m1 = server
        .mock("POST", "/api/files/aws-presigned")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "code": "success",
                "errno": 0,
                "requestId": "test-request-id",
                "mediaId": "test-media-id",
                "response": {
                    "signedUrl": format!("{}/upload-fail", server.url())
                }
            })
            .to_string(),
        )
        .create_async()
        .await;

    // Mock the upload endpoint with failure
    let _m2 = server
        .mock("PUT", "/upload-fail")
        .with_status(400)
        .with_body("Upload failed")
        .create_async()
        .await;

    // Create client with mock server URL
    let config = Config {
        api_key: "test_api_key".to_string(),
        base_url: Some(server.url()),
        ..Default::default()
    };
    let client = Client::new(config).unwrap();

    // Upload file that should fail
    let result = client
        .upload(UploadOptions {
            file_path: file_path.to_str().unwrap().to_string(),
            metadata: None,
        })
        .await;

    // Should fail with UploadFailed error
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::UploadFailed(_) => {} // Expected error
        err => panic!("Unexpected error: {:?}", err),
    }
}

#[tokio::test]
async fn test_file_content_type_detection() {
    let mut server = mockito::Server::new_async().await;

    // Create a temporary test file
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.jpg");
    let mut file = File::create(&file_path).unwrap();
    file.write_all(b"test image data").unwrap();

    // Mock the presigned URL request
    let _m1 = server
        .mock("POST", "/api/files/aws-presigned")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "code": "success",
                "errno": 0,
                "requestId": "test-request-id",
                "mediaId": "test-media-id",
                "response": {
                    "signedUrl": format!("{}/upload-test", server.url())
                }
            })
            .to_string(),
        )
        .create_async()
        .await;

    // Mock the upload endpoint - verify it's a JPEG
    let _m2 = server
        .mock("PUT", "/upload-test")
        .match_header("Content-Type", "image/jpeg")
        .with_status(200)
        .create_async()
        .await;

    // Create client with mock server URL
    let config = Config {
        api_key: "test_api_key".to_string(),
        base_url: Some(server.url()),
        ..Default::default()
    };
    let client = Client::new(config).unwrap();

    // Upload JPEG file
    let result = client
        .upload(UploadOptions {
            file_path: file_path.to_str().unwrap().to_string(),
            metadata: None,
        })
        .await;

    // Should succeed
    assert!(
        result.is_ok(),
        "Failed to upload JPEG file: {:?}",
        result.err()
    );

    // Now test a PNG file
    let png_path = dir.path().join("test.png");
    let mut png_file = File::create(&png_path).unwrap();
    png_file.write_all(b"test png data").unwrap();

    // Mock for PNG
    let _m3 = server
        .mock("POST", "/api/files/aws-presigned")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "code": "success",
                "errno": 0,
                "requestId": "test-png-id",
                "mediaId": "test-png-media",
                "response": {
                    "signedUrl": format!("{}/upload-png", server.url())
                }
            })
            .to_string(),
        )
        .create_async()
        .await;

    // Mock the upload endpoint for PNG
    let _m4 = server
        .mock("PUT", "/upload-png")
        .match_header("Content-Type", "image/png")
        .with_status(200)
        .create_async()
        .await;

    // Upload PNG file
    let result = client
        .upload(UploadOptions {
            file_path: png_path.to_str().unwrap().to_string(),
            metadata: None,
        })
        .await;

    // Should succeed
    assert!(
        result.is_ok(),
        "Failed to upload PNG file: {:?}",
        result.err()
    );
}
