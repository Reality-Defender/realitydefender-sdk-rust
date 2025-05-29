use realitydefender::{
    AnalysisResult, BatchOptions, DetectionModel, GetResultOptions, UploadOptions,
};
use serde_json::{json, Value};

#[test]
fn test_upload_options_serialization() {
    let options = UploadOptions {
        file_path: "path/to/file.jpg".to_string(),
        metadata: Some(json!({ "key": "value" })),
    };

    // Ensure we can serialize to JSON
    let json_str = serde_json::to_string(&options).unwrap();
    let json_value: Value = serde_json::from_str(&json_str).unwrap();

    // Check fields
    assert_eq!(json_value["file_path"], "path/to/file.jpg");
    assert_eq!(json_value["metadata"]["key"], "value");
}

#[test]
fn test_upload_options_default_metadata() {
    let options = UploadOptions {
        file_path: "path/to/file.jpg".to_string(),
        metadata: None,
    };

    let json_str = serde_json::to_string(&options).unwrap();
    let json_value: Value = serde_json::from_str(&json_str).unwrap();

    assert_eq!(json_value["file_path"], "path/to/file.jpg");
    assert!(!json_value.as_object().unwrap().contains_key("metadata"));
}

#[test]
fn test_get_result_options_defaults() {
    let options = GetResultOptions::default();
    assert_eq!(options.wait, None);
    assert_eq!(options.timeout_seconds, None);
}

#[test]
fn test_get_result_options_serialization() {
    let options = GetResultOptions {
        wait: Some(true),
        timeout_seconds: Some(60),
    };

    let json_str = serde_json::to_string(&options).unwrap();
    let json_value: Value = serde_json::from_str(&json_str).unwrap();

    assert_eq!(json_value["wait"], true);
    assert_eq!(json_value["timeout_seconds"], 60);
}

#[test]
fn test_batch_options_defaults() {
    let options = BatchOptions::default();
    assert_eq!(options.max_concurrency, None);
    assert_eq!(options.wait, None);
    assert_eq!(options.timeout_seconds, None);
}

#[test]
fn test_detection_model_deserialization() {
    let json_data = json!({
        "name": "TestModel",
        "status": "COMPLETED",
        "score": 0.85,
        "predictionNumber": 92.5,
        "normalizedPredictionNumber": 85.0,
        "finalScore": 80.0,
        "info": {
            "confidence": "high",
            "details": "Model specific details"
        }
    });

    let model: DetectionModel = serde_json::from_value(json_data).unwrap();

    assert_eq!(model.name, "TestModel");
    assert_eq!(model.status, "COMPLETED");
    assert_eq!(model.score, Some(0.85));
    assert_eq!(model.prediction_number, Some(92.5));
    assert_eq!(model.normalized_prediction_number, Some(85.0));
    assert_eq!(model.final_score, Some(80.0));

    let info = model.info.unwrap();
    assert_eq!(info["confidence"], "high");
    assert_eq!(info["details"], "Model specific details");
}

#[test]
fn test_analysis_result_deserialization() {
    let json_data = json!({
        "requestId": "test-request-123",
        "overallStatus": "COMPLETED",
        "finalScore": 0.75,
        "models": [
            {
                "name": "ModelA",
                "status": "COMPLETED",
                "score": 0.8
            },
            {
                "name": "ModelB",
                "status": "NOT_APPLICABLE"
            }
        ],
        "info": {
            "additionalInfo": "Test info"
        },
        "createdAt": "2023-01-01T12:00:00Z",
        "updatedAt": "2023-01-01T12:05:00Z",
        "resultsSummary": {
            "status": "COMPLETED",
            "metadata": {
                "finalScore": 75,
                "modelCount": 2
            }
        }
    });

    let result: AnalysisResult = serde_json::from_value(json_data).unwrap();

    assert_eq!(result.request_id, "test-request-123");
    assert_eq!(result.status, "COMPLETED");
    assert_eq!(result.score, Some(0.75));
    assert_eq!(result.models.len(), 2);
    assert_eq!(result.models[0].name, "ModelA");
    assert_eq!(result.models[0].status, "COMPLETED");
    assert_eq!(result.models[0].score, Some(0.8));
    assert_eq!(result.models[1].name, "ModelB");
    assert_eq!(result.models[1].status, "NOT_APPLICABLE");
    assert_eq!(result.models[1].score, None);

    assert_eq!(result.created_at, Some("2023-01-01T12:00:00Z".to_string()));
    assert_eq!(result.updated_at, Some("2023-01-01T12:05:00Z".to_string()));

    let results_summary = result.results_summary.unwrap();
    assert_eq!(results_summary.status, "COMPLETED");
    assert_eq!(results_summary.metadata.unwrap()["finalScore"], 75);
}
