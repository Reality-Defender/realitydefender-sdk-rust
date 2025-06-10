use crate::config::Config;
use crate::error::{Error, Result};
use crate::http::{api_paths, HttpClient};
use crate::models::{AnalysisResult, BatchOptions, GetResultOptions, UploadOptions, UploadResult};
use futures::future;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Client for interacting with the Reality Defender API
pub struct Client {
    http_client: HttpClient,
}

impl Client {
    /// Create a new client with the given configuration
    pub fn new(config: Config) -> Result<Self> {
        let http_client = HttpClient::new(config)?;
        Ok(Self { http_client })
    }

    /// Upload a file for analysis
    pub async fn upload(&self, options: UploadOptions) -> Result<UploadResult> {
        // Validate file path
        if !crate::utils::file_exists(&options.file_path) {
            return Err(Error::InvalidFile(format!(
                "File not found: {}",
                options.file_path
            )));
        }

        // Upload file using the presigned URL flow
        self.http_client
            .upload_file::<UploadResult>(&options.file_path)
            .await
    }

    /// Get the analysis result for a specific request ID
    pub async fn get_result(
        &self,
        request_id: &str,
        options: Option<GetResultOptions>,
    ) -> Result<AnalysisResult> {
        let opts = options.unwrap_or_default();
        let should_wait = opts.wait.unwrap_or(false);
        let timeout_seconds = opts.timeout_seconds.unwrap_or(300); // 5 minutes default

        if should_wait {
            self.wait_for_result(request_id, timeout_seconds).await
        } else {
            self.fetch_result(request_id).await
        }
    }

    /// Fetch a result without waiting
    async fn fetch_result(&self, request_id: &str) -> Result<AnalysisResult> {
        let endpoint = format!("{}/{}", api_paths::MEDIA_RESULT, request_id);
        let mut result = self.http_client.get::<AnalysisResult>(&endpoint).await?;

        // Normalize scores from 0-100 to 0-1 range if needed
        self.normalize_scores(&mut result);

        Ok(result)
    }

    /// Normalize scores from 0-100 to 0-1 range
    fn normalize_scores(&self, result: &mut AnalysisResult) {
        // Replace FAKE with ARTIFICIAL in overall status
        if result.status == "FAKE" {
            result.status = "ARTIFICIAL".to_string();
        }

        // Check if we have a score in resultsSummary metadata
        if result.score.is_none() && result.results_summary.is_some() {
            if let Some(metadata) = &result.results_summary.as_ref().unwrap().metadata {
                if let Some(final_score) = metadata.get("finalScore") {
                    if let Some(score_value) = final_score.as_f64() {
                        result.score = Some(if score_value > 1.0 {
                            score_value / 100.0
                        } else {
                            score_value
                        });
                    }
                }
            }
        }

        // Normalize overall score if it exists
        if let Some(score) = &mut result.score {
            if *score > 1.0 {
                *score /= 100.0;
            }
        }

        // Normalize model scores and handle missing scores
        for model in &mut result.models {
            // Replace FAKE with ARTIFICIAL in model status
            if model.status == "FAKE" {
                model.status = "ARTIFICIAL".to_string();
            }

            // If the model has no score, try to get it from other fields
            if model.score.is_none() {
                // Try prediction_number first
                if let Some(pred) = model.prediction_number {
                    model.score = Some(if pred > 1.0 { pred / 100.0 } else { pred });
                }
                // If not, try normalized_prediction_number
                else if let Some(norm_pred) = model.normalized_prediction_number {
                    model.score = Some(if norm_pred > 1.0 {
                        norm_pred / 100.0
                    } else {
                        norm_pred
                    });
                }
                // If not, try final_score
                else if let Some(final_score) = model.final_score {
                    model.score = Some(if final_score > 1.0 {
                        final_score / 100.0
                    } else {
                        final_score
                    });
                }
            }

            // Normalize existing score if needed
            if let Some(score) = &mut model.score {
                if *score > 1.0 {
                    *score /= 100.0;
                }
            }
        }

        // Also replace FAKE with ARTIFICIAL in results_summary if it exists
        if let Some(summary) = &mut result.results_summary {
            if summary.status == "FAKE" {
                summary.status = "ARTIFICIAL".to_string();
            }
        }
    }

    /// Wait for a result to be ready
    async fn wait_for_result(
        &self,
        request_id: &str,
        timeout_seconds: u64,
    ) -> Result<AnalysisResult> {
        let start_time = Instant::now();
        let timeout_duration = Duration::from_secs(timeout_seconds);

        loop {
            let result = self.fetch_result(request_id).await?;

            // Check if analysis is complete. The API uses "ANALYZING" while processing
            // and various status values when complete.
            match result.status.as_str() {
                "ANALYZING" => {
                    // Still processing - continue polling
                    // Check if we've exceeded the timeout
                    if start_time.elapsed() >= timeout_duration {
                        return Err(Error::UnknownError(format!(
                            "Timed out waiting for result after {} seconds",
                            timeout_seconds
                        )));
                    }

                    sleep(Duration::from_secs(2)).await;
                }
                // Any other status means the analysis is done (COMPLETED, ERROR, etc.)
                _ => {
                    return Ok(result);
                }
            }
        }
    }

    /// Process a batch of files concurrently
    pub async fn process_batch(
        &self,
        file_paths: Vec<&str>,
        options: BatchOptions,
    ) -> Result<Vec<AnalysisResult>> {
        if file_paths.is_empty() {
            return Ok(Vec::new());
        }

        let max_concurrency = options.max_concurrency.unwrap_or(5);
        let should_wait = options.wait.unwrap_or(true);
        let timeout_seconds = options.timeout_seconds.unwrap_or(300);

        // Upload all files concurrently with limited concurrency
        let uploads = future::join_all(
            file_paths
                .chunks(max_concurrency)
                .map(|chunk| {
                    let chunk_futures = chunk.iter().map(|&path| {
                        let upload_options = UploadOptions {
                            file_path: path.to_string(),
                            metadata: None,
                        };
                        self.upload(upload_options)
                    });
                    future::join_all(chunk_futures)
                })
                .collect::<Vec<_>>(),
        )
        .await
        .into_iter()
        .flatten()
        .collect::<Vec<Result<UploadResult>>>();

        // Collect all request IDs from successful uploads
        let request_ids: Vec<String> = uploads
            .into_iter()
            .filter_map(|upload_result| match upload_result {
                Ok(result) => Some(result.request_id),
                Err(_) => None,
            })
            .collect();

        // If waiting for results is enabled, get all results
        if should_wait {
            let get_options = GetResultOptions {
                wait: Some(true),
                timeout_seconds: Some(timeout_seconds),
            };

            // Get results concurrently with limited concurrency
            let results = future::join_all(
                request_ids
                    .chunks(max_concurrency)
                    .map(|chunk| {
                        let chunk_futures = chunk
                            .iter()
                            .map(|id| self.get_result(id, Some(get_options.clone())));
                        future::join_all(chunk_futures)
                    })
                    .collect::<Vec<_>>(),
            )
            .await
            .into_iter()
            .flatten()
            .collect::<Vec<Result<AnalysisResult>>>();

            // Filter out errors and return successful results
            Ok(results.into_iter().filter_map(|r| r.ok()).collect())
        } else {
            // Just return empty results with request IDs if not waiting
            Ok(request_ids
                .into_iter()
                .map(|id| AnalysisResult {
                    request_id: id,
                    status: "PROCESSING".to_string(),
                    score: None,
                    models: Vec::new(),
                    info: None,
                    created_at: None,
                    updated_at: None,
                    results_summary: None,
                })
                .collect())
        }
    }

    /// Simplified method to detect a file
    pub async fn detect_file(&self, file_path: &str) -> Result<AnalysisResult> {
        let upload_result = self
            .upload(UploadOptions {
                file_path: file_path.to_string(),
                metadata: None,
            })
            .await?;

        self.get_result(
            &upload_result.request_id,
            Some(GetResultOptions {
                wait: Some(true),
                timeout_seconds: Some(300),
            }),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use crate::{BatchOptions, Client, Config, Error, GetResultOptions, UploadOptions};
    use serde_json::json;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_client_new() {
        let client = Client::new(Config {
            api_key: "test_api_key".to_string(),
            ..Default::default()
        });
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_client_new_empty_api_key() {
        let client = Client::new(Config {
            api_key: "".to_string(),
            ..Default::default()
        });
        assert!(client.is_err());
    }

    #[tokio::test]
    async fn test_get_result() {
        let mut server = mockito::Server::new_async().await;
        let request_id = "test_request_id";

        let mock = server
            .mock("GET", "/api/media/users/test_request_id")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "requestId": request_id,
                    "overallStatus": "COMPLETED",
                    "finalScore": 0.85,
                    "models": [
                        {
                            "name": "TestModel",
                            "status": "COMPLETED",
                            "score": 0.85,
                            "prediction_number": null,
                            "normalized_prediction_number": null,
                            "final_score": null
                        }
                    ],
                    "resultsSummary": {
                        "status": "COMPLETED",
                        "metadata": {
                            "finalScore": 85
                        }
                    }
                })
                .to_string(),
            )
            .create_async()
            .await;

        let client = Client::new(Config {
            api_key: "test_api_key".to_string(),
            base_url: Some(server.url()),
            ..Default::default()
        })
        .unwrap();

        let result = client.get_result(request_id, None).await.unwrap();

        assert_eq!(result.request_id, request_id);
        assert_eq!(result.status, "COMPLETED");
        assert_eq!(result.score, Some(0.85));
        assert_eq!(result.models.len(), 1);
        assert_eq!(result.models[0].name, "TestModel");

        mock.assert_async().await;
    }

    // Testing normalization indirectly through the fetch_result method
    #[tokio::test]
    async fn test_score_normalization() {
        let mut server = mockito::Server::new_async().await;
        let request_id = "test_normalize";

        // Test case 1: score from resultsSummary.metadata.finalScore
        let mock1 = server
            .mock("GET", "/api/media/users/test_normalize")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "requestId": request_id,
                    "overallStatus": "COMPLETED",
                    "models": [],
                    "resultsSummary": {
                        "status": "COMPLETED",
                        "metadata": {
                            "finalScore": 85
                        }
                    }
                })
                .to_string(),
            )
            .create_async()
            .await;

        let client = Client::new(Config {
            api_key: "test_api_key".to_string(),
            base_url: Some(server.url()),
            ..Default::default()
        })
        .unwrap();

        let result = client.get_result(request_id, None).await.unwrap();
        assert_eq!(result.score, Some(0.85)); // Should be normalized from 85 to 0.85

        mock1.assert_async().await;

        // Test case 2: Model with prediction_number
        let mock2 = server
            .mock("GET", "/api/media/users/test_normalize")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "requestId": request_id,
                    "overallStatus": "COMPLETED",
                    "models": [
                        {
                            "name": "Model1",
                            "status": "COMPLETED",
                            "predictionNumber": 92.0
                        }
                    ]
                })
                .to_string(),
            )
            .create_async()
            .await;

        let result = client.get_result(request_id, None).await.unwrap();
        assert_eq!(result.models[0].score, Some(0.92)); // Should be normalized

        mock2.assert_async().await;

        // Test case 3: Model with normalizedPredictionNumber
        let mock3 = server
            .mock("GET", "/api/media/users/test_normalize")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "requestId": request_id,
                    "overallStatus": "COMPLETED",
                    "models": [
                        {
                            "name": "Model2",
                            "status": "COMPLETED",
                            "normalizedPredictionNumber": 80.0
                        }
                    ]
                })
                .to_string(),
            )
            .create_async()
            .await;

        let result = client.get_result(request_id, None).await.unwrap();
        assert_eq!(result.models[0].score, Some(0.80)); // Should be normalized

        mock3.assert_async().await;

        // Test case 4: Model with finalScore
        let mock4 = server
            .mock("GET", "/api/media/users/test_normalize")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "requestId": request_id,
                    "overallStatus": "COMPLETED",
                    "models": [
                        {
                            "name": "Model3",
                            "status": "COMPLETED",
                            "finalScore": 70.0
                        }
                    ]
                })
                .to_string(),
            )
            .create_async()
            .await;

        let result = client.get_result(request_id, None).await.unwrap();
        assert_eq!(result.models[0].score, Some(0.70)); // Should be normalized

        mock4.assert_async().await;
    }

    #[tokio::test]
    async fn test_upload_with_invalid_file() {
        let client = Client::new(Config {
            api_key: "test_api_key".to_string(),
            ..Default::default()
        })
        .unwrap();

        // Test with non-existent file
        let result = client
            .upload(UploadOptions {
                file_path: "non_existent_file.jpg".to_string(),
                metadata: None,
            })
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidFile(_) => {} // Expected error
            err => panic!("Unexpected error: {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_wait_for_result() {
        let mut server = mockito::Server::new_async().await;
        let request_id = "test-wait-request";

        // First response - analyzing
        let mock1 = server
            .mock("GET", format!("/api/media/users/{}", request_id).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "requestId": request_id,
                    "overallStatus": "ANALYZING",
                    "models": []
                })
                .to_string(),
            )
            .create_async()
            .await;

        // Second response - completed
        let mock2 = server
            .mock("GET", format!("/api/media/users/{}", request_id).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "requestId": request_id,
                    "overallStatus": "COMPLETED",
                    "finalScore": 0.75,
                    "models": []
                })
                .to_string(),
            )
            .create_async()
            .await;

        let client = Client::new(Config {
            api_key: "test_api_key".to_string(),
            base_url: Some(server.url()),
            ..Default::default()
        })
        .unwrap();

        // Test with waiting option
        let result = client
            .get_result(
                request_id,
                Some(GetResultOptions {
                    wait: Some(true),
                    timeout_seconds: Some(5), // Short timeout for test
                }),
            )
            .await;

        assert!(result.is_ok());
        let analysis_result = result.unwrap();
        assert_eq!(analysis_result.request_id, request_id);
        assert_eq!(analysis_result.status, "COMPLETED");
        assert_eq!(analysis_result.score, Some(0.75));

        mock1.assert_async().await;
        mock2.assert_async().await;
    }

    #[tokio::test]
    async fn test_process_batch_empty() {
        let client = Client::new(Config {
            api_key: "test_api_key".to_string(),
            ..Default::default()
        })
        .unwrap();

        // Test with empty file paths
        let result = client
            .process_batch(
                vec![],
                BatchOptions {
                    max_concurrency: Some(2),
                    wait: Some(true),
                    timeout_seconds: Some(10),
                },
            )
            .await;

        assert!(result.is_ok());
        let batch_results = result.unwrap();
        assert!(batch_results.is_empty());
    }

    #[tokio::test]
    async fn test_process_batch_without_waiting() {
        let mut server = mockito::Server::new_async().await;

        // Create temporary test files
        let dir = tempdir().unwrap();
        let file_path1 = dir.path().join("test1.jpg");
        let mut file1 = File::create(&file_path1).unwrap();
        file1.write_all(b"test image data 1").unwrap();

        let file_path2 = dir.path().join("test2.jpg");
        let mut file2 = File::create(&file_path2).unwrap();
        file2.write_all(b"test image data 2").unwrap();

        // Mock the presigned URL requests
        let mock1 = server
            .mock("POST", "/api/files/aws-presigned")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "code": "success",
                    "errno": 0,
                    "requestId": "test-request-id-1",
                    "mediaId": "test-media-id-1",
                    "response": {
                        "signedUrl": format!("{}/upload1", server.url())
                    }
                })
                .to_string(),
            )
            .create_async()
            .await;

        let mock2 = server
            .mock("POST", "/api/files/aws-presigned")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "code": "success",
                    "errno": 0,
                    "requestId": "test-request-id-2",
                    "mediaId": "test-media-id-2",
                    "response": {
                        "signedUrl": format!("{}/upload2", server.url())
                    }
                })
                .to_string(),
            )
            .create_async()
            .await;

        // Mock the upload endpoints
        let mock_upload1 = server
            .mock("PUT", "/upload1")
            .with_status(200)
            .create_async()
            .await;

        let mock_upload2 = server
            .mock("PUT", "/upload2")
            .with_status(200)
            .create_async()
            .await;

        let client = Client::new(Config {
            api_key: "test_api_key".to_string(),
            base_url: Some(server.url()),
            ..Default::default()
        })
        .unwrap();

        // Process batch without waiting for results
        let file_paths = vec![file_path1.to_str().unwrap(), file_path2.to_str().unwrap()];

        let result = client
            .process_batch(
                file_paths,
                BatchOptions {
                    max_concurrency: Some(2),
                    wait: Some(false),
                    timeout_seconds: Some(10),
                },
            )
            .await;

        assert!(result.is_ok());
        let batch_results = result.unwrap();
        assert_eq!(batch_results.len(), 2);

        // Check that results have request IDs but are in PROCESSING state
        assert_eq!(batch_results[0].request_id, "test-request-id-1");
        assert_eq!(batch_results[0].status, "PROCESSING");
        assert_eq!(batch_results[1].request_id, "test-request-id-2");
        assert_eq!(batch_results[1].status, "PROCESSING");

        mock1.assert_async().await;
        mock2.assert_async().await;
        mock_upload1.assert_async().await;
        mock_upload2.assert_async().await;
    }

    #[tokio::test]
    async fn test_detect_file() {
        let mut server = mockito::Server::new_async().await;

        // Create a temporary file for testing
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.jpg");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"test image data").unwrap();

        // Mock the presigned URL request
        let mock1 = server
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

        // Mock the upload endpoint
        let mock2 = server
            .mock("PUT", "/upload")
            .with_status(200)
            .create_async()
            .await;

        // Mock the result endpoint
        let mock3 = server
            .mock("GET", "/api/media/users/test-request-id")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "requestId": "test-request-id",
                    "overallStatus": "COMPLETED",
                    "finalScore": 0.75,
                    "models": []
                })
                .to_string(),
            )
            .create_async()
            .await;

        let client = Client::new(Config {
            api_key: "test_api_key".to_string(),
            base_url: Some(server.url()),
            ..Default::default()
        })
        .unwrap();

        // Test detect_file method
        let result = client.detect_file(file_path.to_str().unwrap()).await;

        assert!(result.is_ok());
        let analysis_result = result.unwrap();
        assert_eq!(analysis_result.request_id, "test-request-id");
        assert_eq!(analysis_result.status, "COMPLETED");
        assert_eq!(analysis_result.score, Some(0.75));

        mock1.assert_async().await;
        mock2.assert_async().await;
        mock3.assert_async().await;
    }
}
