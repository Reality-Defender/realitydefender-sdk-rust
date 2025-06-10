use serde::{Deserialize, Serialize};

/// Options for uploading a file
#[derive(Debug, Clone, Serialize)]
pub struct UploadOptions {
    /// Path to the file to upload
    pub file_path: String,

    /// Optional metadata for the upload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Response containing a presigned URL for file upload
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SignedUrlResponse {
    /// Status code from the API
    #[serde(rename = "code")]
    pub code: String,

    /// Error number (0 if successful)
    #[serde(rename = "errno")]
    pub errno: i32,

    /// Unique identifier for the upload request
    #[serde(rename = "requestId")]
    pub request_id: String,

    /// Unique identifier for the media
    #[serde(rename = "mediaId")]
    pub media_id: String,

    /// Response details containing the signed URL
    pub response: SignedUrlDetails,
}

/// Details of the signed URL response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SignedUrlDetails {
    /// The presigned URL for uploading
    #[serde(rename = "signedUrl")]
    pub signed_url: String,
}

/// Result of an upload operation
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UploadResult {
    /// Unique identifier for the upload request
    pub request_id: String,

    /// Unique identifier for the media
    #[serde(default)]
    pub media_id: String,

    /// URL where the result can be accessed
    #[serde(default)]
    pub result_url: Option<String>,
}

/// Options for getting a result
#[derive(Debug, Clone, Default, Serialize)]
pub struct GetResultOptions {
    /// Whether to wait for the result to be ready
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait: Option<bool>,

    /// Maximum time to wait for the result in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
}

/// Model-specific detection results
#[derive(Debug, Clone, Deserialize)]
pub struct DetectionModel {
    /// Name of the model
    pub name: String,

    /// Status of the detection (COMPLETED, PROCESSING, ERROR, etc.)
    pub status: String,

    /// Detection score (0-1 range, normalized by the SDK, higher is more likely to be ARTIFICIAL)
    pub score: Option<f64>,

    /// Raw prediction number from the model (may be on 0-100 scale, used internally for normalization)
    #[serde(rename = "predictionNumber")]
    pub prediction_number: Option<f64>,

    /// Normalized prediction number (typically 0-100 scale, used internally for normalization)
    #[serde(rename = "normalizedPredictionNumber")]
    pub normalized_prediction_number: Option<f64>,

    /// Final score for this model (typically 0-100 scale, used internally for normalization)
    #[serde(rename = "finalScore")]
    pub final_score: Option<f64>,

    /// Additional information about the detection
    #[serde(default)]
    pub info: Option<serde_json::Value>,
}

/// Result of an analysis
#[derive(Debug, Clone, Deserialize)]
pub struct AnalysisResult {
    /// Unique identifier for the analysis request
    #[serde(rename = "requestId")]
    pub request_id: String,

    /// Status of the analysis (COMPLETED, PROCESSING, ERROR, etc.)
    #[serde(rename = "overallStatus")]
    pub status: String,

    /// Overall detection score (0-1 range, normalized by the SDK, higher is more likely to be ARTIFICIAL)
    #[serde(default)]
    #[serde(rename = "finalScore")]
    pub score: Option<f64>,

    /// Array of model-specific results
    #[serde(default)]
    pub models: Vec<DetectionModel>,

    /// Additional information about the analysis
    #[serde(default)]
    pub info: Option<serde_json::Value>,

    /// Timestamp when the analysis was created
    #[serde(default)]
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,

    /// Timestamp when the analysis was updated
    #[serde(default)]
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<String>,

    /// Results summary containing status and metadata
    #[serde(default)]
    #[serde(rename = "resultsSummary")]
    pub results_summary: Option<ResultsSummary>,
}

/// Summary of analysis results
#[derive(Debug, Clone, Deserialize)]
pub struct ResultsSummary {
    /// Status of the analysis
    pub status: String,

    /// Metadata containing score and other information
    pub metadata: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Options for batch processing
#[derive(Debug, Clone, Default)]
pub struct BatchOptions {
    /// Maximum number of concurrent uploads
    pub max_concurrency: Option<usize>,

    /// Whether to wait for results
    pub wait: Option<bool>,

    /// Maximum time to wait for results in seconds
    pub timeout_seconds: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
