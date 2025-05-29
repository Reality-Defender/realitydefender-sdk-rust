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
                "Failed to upload to presigned URL. Status: {} Body: {}",
                status, body
            )));
        }

        Ok(())
    }

    /// Upload a file using the presigned URL flow
    pub async fn upload_file<T: DeserializeOwned>(&self, file_path: &str) -> Result<T> {
        // 1. Get file name
        let path = Path::new(file_path);
        if !path.exists() {
            return Err(Error::InvalidFile(format!("File not found: {}", file_path)));
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
                return Err(Error::InvalidFile(format!("File is empty: {}", file_path)));
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
                            format!("Unknown error (HTTP {})", status)
                        }
                    } else {
                        format!("Unknown error (HTTP {})", status)
                    };

                Err(Error::UnknownError(error_msg))
            }
        }
    }
}
