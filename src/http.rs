use crate::config::Config;
use crate::error::{Error, Result};
use reqwest::{Client as ReqwestClient, ClientBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::Path;
use std::time::Duration;

/// Constants for API paths
pub mod api_paths {
    /// Path for requesting a presigned upload URL
    pub const SIGNED_URL: &str = "/api/files/aws-presigned";
    /// Path for retrieving media results
    pub const MEDIA_RESULT: &str = "/api/media/users";
}

/// HTTP client for making API requests
pub struct HttpClient {
    client: ReqwestClient,
    config: Config,
}

impl HttpClient {
    /// Create a new HTTP client with the given configuration
    pub fn new(config: Config) -> Result<Self> {
        config.validate()?;

        // Use the same User-Agent as Go SDK might be using
        let client = ClientBuilder::new()
            .user_agent("realitydefender-go-sdk/1.0")
            .timeout(Duration::from_secs(config.get_timeout_seconds()))
            .build()?;

        Ok(Self { client, config })
    }

    /// Make a GET request to the specified endpoint
    pub async fn get<T: DeserializeOwned>(&self, endpoint: &str) -> Result<T> {
        let url = format!("{}{}", self.config.get_base_url(), endpoint);

        let request = self
            .client
            .get(&url)
            .header("X-API-KEY", &self.config.api_key)
            .header("Accept", "application/json")
            .header("Accept-Encoding", "gzip")
            .build()?;

        let response = self.client.execute(request).await?;
        self.handle_response(response).await
    }

    /// Make a POST request with JSON data to the specified endpoint
    pub async fn post<T: DeserializeOwned, D: Serialize>(
        &self,
        endpoint: &str,
        data: &D,
    ) -> Result<T> {
        let url = format!("{}{}", self.config.get_base_url(), endpoint);

        let request = self
            .client
            .post(&url)
            .header("X-API-KEY", &self.config.api_key)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("Accept-Encoding", "gzip")
            .json(data)
            .build()?;

        let response = self.client.execute(request).await?;
        self.handle_response(response).await
    }

    /// Make a PUT request to upload data to a URL (used for presigned URLs)
    pub async fn put(&self, url: &str, data: Vec<u8>, content_type: &str) -> Result<()> {
        let request = self
            .client
            .put(url)
            .header("Content-Type", content_type)
            .header("Content-Length", data.len().to_string())
            // Do not include X-API-KEY for presigned URL uploads
            .body(data.clone())
            .build()?;

        let response = self.client.execute(request).await?;
        let status = response.status();

        // Check if the upload was successful
        if !status.is_success() {
            let body = response.text().await?;
            return Err(Error::UploadFailed(format!(
                "Failed to upload to presigned URL. Status: {status} Body: {body}"
            )));
        }

        Ok(())
    }

    /// Upload a file using the presigned URL flow
    pub async fn upload_file<T: DeserializeOwned>(&self, file_path: &str) -> Result<T> {
        // 1. Get file name
        let path = Path::new(file_path);
        if !path.exists() {
            return Err(Error::InvalidFile(format!("File not found: {file_path}")));
        }

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| Error::InvalidFile("Invalid file name".to_string()))?;

        let payload = serde_json::json!({ "fileName": file_name });

        // 2. Request a presigned URL
        let signed_url_response = self
            .post::<crate::models::SignedUrlResponse, _>(api_paths::SIGNED_URL, &payload)
            .await?;

        // 3. Read the file content
        let file_content = tokio::fs::read(path).await?;

        // Check if file is empty or if there's an issue reading it
        if file_content.is_empty() {
            // Try with std::fs to see if it reads the file correctly
            let std_file_content = std::fs::read(path)?;

            if std_file_content.is_empty() {
                return Err(Error::InvalidFile(format!("File is empty: {file_path}")));
            }

            // Use the content read by std::fs instead
            return self
                .upload_file_with_content(signed_url_response, std_file_content, path)
                .await;
        }

        // 4. Determine content type based on file extension
        let content_type = match path.extension().and_then(|ext| ext.to_str()) {
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("png") => "image/png",
            Some("gif") => "image/gif",
            Some("mp4") => "video/mp4",
            Some("mov") => "video/quicktime",
            Some("avi") => "video/x-msvideo",
            Some("webm") => "video/webm",
            _ => "application/octet-stream",
        };

        // 5. Upload to the presigned URL
        self.put(
            &signed_url_response.response.signed_url,
            file_content,
            content_type,
        )
        .await?;

        // 6. Create upload result with request_id and media_id
        let upload_result = crate::models::UploadResult {
            request_id: signed_url_response.request_id,
            media_id: signed_url_response.media_id,
            result_url: None,
        };

        // 7. Convert to the requested type
        Ok(serde_json::from_value(serde_json::to_value(
            upload_result,
        )?)?)
    }

    /// Helper method to upload file with provided content
    async fn upload_file_with_content<T: DeserializeOwned>(
        &self,
        signed_url_response: crate::models::SignedUrlResponse,
        file_content: Vec<u8>,
        path: &Path,
    ) -> Result<T> {
        // Determine content type based on file extension
        let content_type = match path.extension().and_then(|ext| ext.to_str()) {
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("png") => "image/png",
            Some("gif") => "image/gif",
            Some("mp4") => "video/mp4",
            Some("mov") => "video/quicktime",
            Some("avi") => "video/x-msvideo",
            Some("webm") => "video/webm",
            _ => "application/octet-stream",
        };

        // Upload to the presigned URL
        self.put(
            &signed_url_response.response.signed_url,
            file_content,
            content_type,
        )
        .await?;

        // Create upload result with request_id and media_id
        let upload_result = crate::models::UploadResult {
            request_id: signed_url_response.request_id,
            media_id: signed_url_response.media_id,
            result_url: None,
        };

        // Convert to the requested type
        Ok(serde_json::from_value(serde_json::to_value(
            upload_result,
        )?)?)
    }

    /// Handle API responses and parse JSON
    async fn handle_response<T: DeserializeOwned>(&self, response: Response) -> Result<T> {
        let status = response.status();
        let body = response.bytes().await?;
        match status {
            StatusCode::OK | StatusCode::CREATED => Ok(serde_json::from_slice(&body)?),
            StatusCode::UNAUTHORIZED => Err(Error::Unauthorized),
            StatusCode::NOT_FOUND => Err(Error::NotFound("Resource not found".to_string())),
            StatusCode::INTERNAL_SERVER_ERROR => {
                Err(Error::ServerError("Server error".to_string()))
            }
            StatusCode::FORBIDDEN => {
                // Enhanced error for 403 Forbidden
                Err(Error::Unauthorized)
            }
            _ => {
                // Try to parse error message from response
                let error_msg =
                    if let Ok(error_resp) = serde_json::from_slice::<serde_json::Value>(&body) {
                        if let Some(msg) = error_resp.get("error").and_then(|e| e.as_str()) {
                            msg.to_string()
                        } else {
                            format!("Unknown error (HTTP {status})")
                        }
                    } else {
                        format!("Unknown error (HTTP {status})")
                    };

                Err(Error::UnknownError(error_msg))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Client, UploadOptions};
    use mockito::Matcher;
    use serde::Deserialize;
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
            })
            .await;

        // Should succeed
        assert!(
            result.is_ok(),
            "Failed to upload PNG file: {:?}",
            result.err()
        );
    }
}
