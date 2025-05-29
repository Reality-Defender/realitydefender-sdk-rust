use realitydefender::{BatchOptions, Client, Config, Error, GetResultOptions, UploadOptions};
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
