use serde::{Deserialize, Serialize};

/// Base API response
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct BaseResponse {
    /// Status code from the API
    #[serde(rename = "code")]
    pub code: String,

    /// Error number (0 if successful)
    #[serde(rename = "errno")]
    pub errno: i32,

    /// Response message.
    #[serde(rename = "response")]
    pub response: String,

    /// Response message.
    #[serde(rename = "requestId")]
    pub request_id: Option<String>,
}

/// Options for uploading a file
#[derive(Debug, Clone, Serialize)]
pub struct UploadOptions {
    /// Path to the file to upload
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UploadSocialMediaOptions {
    /// Path to the file to upload
    #[serde(rename = "socialLink")]
    pub social_link: String,
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
    pub media_id: Option<String>,

    /// URL where the result can be accessed
    #[serde(default)]
    pub result_url: Option<String>,
}

/// Options for getting a result
#[derive(Debug, Clone, Default, Serialize)]
pub struct GetResultOptions {
    /// Maximum number of attempts to get results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_attempts: Option<u64>,

    /// How long to wait between attempts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub polling_interval: Option<u64>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum FloatOrObject {
    Float(f64),
    Object(serde_json::Map<String, serde_json::Value>),
}

/// Model-specific detection results
#[derive(Debug, Clone, Deserialize)]
pub struct DetectionModel {
    /// Name of the model
    pub name: String,

    /// Status of the detection (COMPLETED, PROCESSING, ERROR, etc.)
    pub status: String,

    /// Raw prediction number from the model (may be on 0-100 scale, used internally for normalization)
    #[serde(rename = "predictionNumber")]
    pub prediction_number: Option<FloatOrObject>,

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

    /// Overall detection score (0-1 range, normalized by the SDK, higher is more likely to be MANIPULATED)
    #[serde(default)]
    #[serde(rename = "finalScore")]
    pub final_score: Option<f64>,

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

    /// Maximum number of attempts to get results
    pub max_attempts: Option<u64>,

    /// How long to wait between attempts
    pub polling_interval: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DetectionModelResult {
    /// Name of the model
    pub name: String,

    /// Status of the detection (COMPLETED, PROCESSING, ERROR, etc.)
    pub status: String,

    /// Detection score (0-1 range, normalized by the SDK, higher is more likely to be MANIPULATED)
    pub score: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DetectionResult {
    /// Unique identifier for the upload request
    #[serde(rename = "requestId")]
    pub request_id: String,

    /// Status of the analysis (COMPLETED, PROCESSING, ERROR, etc.)
    pub status: String,

    /// Confidence score (0-1 range, null if processing)
    pub score: Option<f64>,

    /// Results from individual detection models
    pub models: Vec<DetectionModelResult>,
}

/// Options for getting results with pagination and filtering
#[derive(Debug, Clone, Default, Serialize)]
pub struct GetResultsOptions {
    /// Page number (0-based)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<u32>,

    /// Number of items per page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u32>,

    /// Filter by name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Start date filter (YYYY-MM-DD format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,

    /// End date filter (YYYY-MM-DD format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,

    /// Maximum number of attempts to get results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_attempts: Option<u64>,

    /// How long to wait between attempts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub polling_interval: Option<u64>,
}

/// Paginated list of detection results
#[derive(Debug, Clone, Deserialize)]
pub struct DetectionResultList {
    /// Total number of items across all pages
    #[serde(rename = "totalItems")]
    pub total_items: u32,

    /// Total number of pages
    #[serde(rename = "totalPages")]
    pub total_pages: u32,

    /// Current page number (0-based)
    #[serde(rename = "currentPage")]
    pub current_page: u32,

    /// Number of items on current page
    #[serde(rename = "currentPageItemsCount")]
    pub current_page_items_count: u32,

    /// List of detection results for this page
    #[serde(rename = "mediaList")]
    pub items: Vec<AnalysisResult>,
}

/// Formatted detection result list for user consumption
#[derive(Debug, Clone)]
pub struct FormattedDetectionResultList {
    /// Total number of items across all pages
    pub total_items: u32,

    /// Total number of pages
    pub total_pages: u32,

    /// Current page number (0-based)
    pub current_page: u32,

    /// Number of items on current page
    pub current_page_items_count: u32,

    /// List of formatted detection results for this page
    pub items: Vec<DetectionResult>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn test_upload_options_serialization() {
        let options = UploadOptions {
            file_path: "path/to/file.jpg".to_string(),
        };

        // Ensure we can serialize to JSON
        let json_str = serde_json::to_string(&options).unwrap();
        let json_value: Value = serde_json::from_str(&json_str).unwrap();

        // Check fields
        assert_eq!(json_value["file_path"], "path/to/file.jpg");
    }

    #[test]
    fn test_upload_options_default_metadata() {
        let options = UploadOptions {
            file_path: "path/to/file.jpg".to_string(),
        };

        let json_str = serde_json::to_string(&options).unwrap();
        let json_value: Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json_value["file_path"], "path/to/file.jpg");
    }

    #[test]
    fn test_get_result_options_defaults() {
        let options = GetResultOptions::default();
        assert_eq!(options.max_attempts, None);
        assert_eq!(options.polling_interval, None);
    }

    #[test]
    fn test_get_result_options_serialization() {
        let options = GetResultOptions {
            max_attempts: Some(30),
            polling_interval: Some(2000),
        };

        let json_str = serde_json::to_string(&options).unwrap();
        let json_value: Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json_value["max_attempts"], 30);
        assert_eq!(json_value["polling_interval"], 2000);
    }

    #[test]
    fn test_batch_options_defaults() {
        let options = BatchOptions::default();
        assert_eq!(options.max_concurrency, None);
        assert_eq!(options.max_attempts, None);
        assert_eq!(options.polling_interval, None);
    }

    #[test]
    fn test_detection_model_deserialization() {
        let json_data = json!({
            "name": "TestModel",
            "status": "COMPLETED",
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
            "finalScore": 75,
            "models": [
                {
                    "name": "ModelA",
                    "status": "COMPLETED",
                    "finalScore": 80.0,
                },
                {
                    "name": "ModelB",
                    "status": "NOT_APPLICABLE"
                },
                {
                  "name": "ModelC",
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
        assert_eq!(result.final_score, Some(75.0));
        assert_eq!(result.models.len(), 3);

        assert_eq!(result.models[0].name, "ModelA");
        assert_eq!(result.models[0].status, "COMPLETED");
        assert_eq!(result.models[0].final_score, Some(80.0));
        assert_eq!(result.models[0].prediction_number, None);

        assert_eq!(result.models[1].name, "ModelB");
        assert_eq!(result.models[1].status, "NOT_APPLICABLE");
        assert_eq!(result.models[1].final_score, None);
        assert_eq!(result.models[1].prediction_number, None);

        assert_eq!(result.models[2].name, "ModelC");
        assert_eq!(result.models[2].status, "NOT_APPLICABLE");
        assert_eq!(result.models[2].final_score, None);
        assert!(matches!(
            result.models[2].prediction_number,
            Some(FloatOrObject::Object { .. })
        ));

        assert_eq!(result.created_at, Some("2023-01-01T12:00:00Z".to_string()));
        assert_eq!(result.updated_at, Some("2023-01-01T12:05:00Z".to_string()));

        let results_summary = result.results_summary.unwrap();
        assert_eq!(results_summary.status, "COMPLETED");
        assert_eq!(results_summary.metadata.unwrap()["finalScore"], 75);
    }
}
