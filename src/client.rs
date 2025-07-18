use crate::config::Config;
use crate::error::{Error, Result};
use crate::http::{api_paths, HttpClient};
use crate::models::{
    AnalysisResult, BatchOptions, DetectionModelResult, DetectionResult, DetectionResultList,
    FloatOrObject, FormattedDetectionResultList, GetResultOptions, GetResultsOptions,
    UploadOptions, UploadResult,
};
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
    ) -> Result<DetectionResult> {
        let opts = options.unwrap_or_default();
        let should_wait =
            opts.max_attempts.unwrap_or(0) > 0 && opts.polling_interval.unwrap_or(0) > 0;

        if should_wait {
            self.wait_for_result(
                request_id,
                opts.max_attempts.unwrap(),
                opts.polling_interval.unwrap(),
            )
            .await
        } else {
            self.fetch_result(request_id).await
        }
    }

    /// Fetch a result without waiting
    async fn fetch_result(&self, request_id: &str) -> Result<DetectionResult> {
        let endpoint = format!("{}/{}", api_paths::MEDIA_RESULT, request_id);
        let result = self.http_client.get::<AnalysisResult>(&endpoint).await?;

        // Normalize scores from 0-100 to 0-1 range if needed
        Ok(self.normalize_scores(&result))
    }

    /// Normalize scores from 0-100 to 0-1 range
    fn normalize_scores(&self, result: &AnalysisResult) -> DetectionResult {
        let mut detection_result = DetectionResult {
            // Replace FAKE with MANIPULATED in overall status
            status: if result.status == "FAKE" {
                "MANIPULATED".to_string()
            } else {
                result.status.clone()
            },
            request_id: result.request_id.clone(),
            score: result.final_score.map(|final_score| final_score / 100.0),
            models: vec![],
        };

        // Check if we have a score in resultsSummary metadata
        if result.results_summary.is_some() {
            if let Some(metadata) = &result.results_summary.as_ref().unwrap().metadata {
                if let Some(final_score) = metadata.get("finalScore") {
                    if let Some(score_value) = final_score.as_f64() {
                        detection_result.score = Some(score_value / 100.0)
                    }
                }
            }
        }

        // Normalize model scores and handle missing scores
        detection_result.models = result
            .models
            .iter()
            .filter(|model| model.status != "NOT_APPLICABLE")
            .map(|model| DetectionModelResult {
                name: model.name.clone(),
                status: if model.status == "FAKE" {
                    "MANIPULATED".to_string()
                } else {
                    model.status.clone()
                },
                score: match model.prediction_number {
                    Some(FloatOrObject::Float(val)) => Some(val),
                    _ => None,
                },
            })
            .collect();

        detection_result
    }

    /// Wait for a result to be ready
    async fn wait_for_result(
        &self,
        request_id: &str,
        max_attempts: u64,
        polling_interval: u64,
    ) -> Result<DetectionResult> {
        let start_time = Instant::now();

        for _ in 0..max_attempts {
            let result = self.fetch_result(request_id).await?;

            // Check if analysis is complete. The API uses "ANALYZING" while processing
            // and various status values when complete.
            match result.status.as_str() {
                "ANALYZING" => sleep(Duration::from_millis(polling_interval)).await,
                // Any other status means the analysis is done (COMPLETED, ERROR, etc.)
                _ => {
                    return Ok(result);
                }
            }
        }

        Err(Error::UnknownError(format!(
            "Timed out waiting for result after {} seconds",
            (Instant::now() - start_time).as_secs()
        )))
    }

    /// Process a batch of files concurrently
    pub async fn process_batch(
        &self,
        file_paths: Vec<&str>,
        options: BatchOptions,
    ) -> Result<Vec<DetectionResult>> {
        if file_paths.is_empty() {
            return Ok(Vec::new());
        }

        let max_concurrency = options.max_concurrency.unwrap_or(5);
        let should_wait =
            options.max_attempts.unwrap_or(0) > 0 && options.polling_interval.unwrap_or(0) > 0;

        // Upload all files concurrently with limited concurrency
        let uploads = future::join_all(
            file_paths
                .chunks(max_concurrency)
                .map(|chunk| {
                    let chunk_futures = chunk.iter().map(|&path| {
                        let upload_options = UploadOptions {
                            file_path: path.to_string(),
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
                max_attempts: options.max_attempts,
                polling_interval: options.polling_interval,
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
            .collect::<Vec<Result<DetectionResult>>>();

            // Filter out errors and return successful results
            Ok(results.into_iter().filter_map(|r| r.ok()).collect())
        } else {
            // Just return empty results with request IDs if not waiting
            Ok(request_ids
                .into_iter()
                .map(|id| DetectionResult {
                    request_id: id,
                    status: "PROCESSING".to_string(),
                    score: None,
                    models: Vec::new(),
                })
                .collect())
        }
    }

    /// Get a paginated list of detection results with optional filters
    pub async fn get_results(
        &self,
        options: Option<GetResultsOptions>,
    ) -> Result<FormattedDetectionResultList> {
        let opts = options.unwrap_or_default();
        let should_wait =
            opts.max_attempts.unwrap_or(0) > 0 && opts.polling_interval.unwrap_or(0) > 0;

        if should_wait {
            self.wait_for_results(opts).await
        } else {
            self.fetch_results(opts).await
        }
    }

    /// Fetch results without waiting
    async fn fetch_results(
        &self,
        options: GetResultsOptions,
    ) -> Result<FormattedDetectionResultList> {
        let page_number = options.page_number.unwrap_or(0);
        let endpoint = format!("{}/{}", api_paths::ALL_MEDIA_RESULTS, page_number);

        let mut params = Vec::new();

        if let Some(size) = options.size {
            params.push(("size", size.to_string()));
        }
        if let Some(ref name) = options.name {
            params.push(("name", name.to_string()));
        }
        if let Some(ref start_date) = options.start_date {
            params.push(("startDate", start_date.to_string()));
        }
        if let Some(ref end_date) = options.end_date {
            params.push(("endDate", end_date.to_string()));
        }

        // Convert to string references for the API call
        let param_refs: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        let raw_result = self
            .http_client
            .get_with_params::<DetectionResultList>(&endpoint, &param_refs)
            .await?;

        // Convert to formatted result
        Ok(self.format_results_list(&raw_result))
    }

    /// Wait for results with retry logic
    async fn wait_for_results(
        &self,
        options: GetResultsOptions,
    ) -> Result<FormattedDetectionResultList> {
        let max_attempts = options.max_attempts.unwrap_or(5);
        let polling_interval = options.polling_interval.unwrap_or(2000);

        let start_time = Instant::now();

        for _ in 0..max_attempts {
            let result = self.fetch_results(options.clone()).await?;

            // Check if any results are still analyzing
            let still_analyzing = result.items.iter().any(|item| item.status == "ANALYZING");

            if !still_analyzing {
                return Ok(result);
            }

            sleep(Duration::from_millis(polling_interval)).await;
        }

        Err(Error::UnknownError(format!(
            "Timed out waiting for results after {} seconds",
            (Instant::now() - start_time).as_secs()
        )))
    }

    /// Format raw results list into user-friendly format
    fn format_results_list(
        &self,
        raw_result: &DetectionResultList,
    ) -> FormattedDetectionResultList {
        let formatted_items = raw_result
            .items
            .iter()
            .map(|item| self.normalize_scores(item))
            .collect();

        FormattedDetectionResultList {
            total_items: raw_result.total_items,
            total_pages: raw_result.total_pages,
            current_page: raw_result.current_page,
            current_page_items_count: raw_result.current_page_items_count,
            items: formatted_items,
        }
    }

    /// Simplified method to detect a file
    pub async fn detect_file(&self, file_path: &str) -> Result<DetectionResult> {
        let upload_result = self
            .upload(UploadOptions {
                file_path: file_path.to_string(),
            })
            .await?;

        self.get_result(
            &upload_result.request_id,
            Some(GetResultOptions {
                max_attempts: Some(150),
                polling_interval: Some(2000),
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
                    "models": [
                        {
                            "name": "TestModel",
                            "status": "COMPLETED",
                            "predictionNumber": 0.27,
                            "normalizedPredictionNumber": 27,
                            "finalScore": null
                        },
                        {
                          "name": "TestModel2",
                          "status": "COMPLETED",
                          "predictionNumber": {
                            "reason": "relevance: no faces detected/faces too small",
                            "decision": "NOT_EVALUATED"
                          },
                          "normalizedPredictionNumber": null,
                          "rollingAvgNumber": null,
                          "finalScore": null
                        },
                        {
                          "name": "TestModel3",
                          "status": "NOT_APPLICABLE",
                          "predictionNumber": {
                            "reason": "relevance: no faces detected/faces too small",
                            "decision": "NOT_EVALUATED"
                          },
                          "normalizedPredictionNumber": null,
                          "rollingAvgNumber": null,
                          "finalScore": null
                        },
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
        assert_eq!(result.models.len(), 2);

        assert_eq!(result.models[0].name, "TestModel");
        assert_eq!(result.models[0].score, Some(0.27));
        assert_eq!(result.models[0].status, "COMPLETED");

        assert_eq!(result.models[1].name, "TestModel2");
        assert_eq!(result.models[1].score, None);
        assert_eq!(result.models[1].status, "COMPLETED");

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
                            "predictionNumber": 0.92
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
        assert_eq!(result.models[0].score, None); // Should be normalized

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
        assert_eq!(result.models[0].score, None); // Should be normalized

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
                    "finalScore": 75,
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
                    max_attempts: Some(5),
                    polling_interval: Some(1000), // Short timeout for test
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
                    max_attempts: Some(10),
                    polling_interval: Some(1000),
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
                    max_attempts: None,
                    polling_interval: None,
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
                    "finalScore": 75,
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
