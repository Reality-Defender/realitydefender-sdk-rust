//! # Reality Defender SDK
//!
//! The Reality Defender SDK provides tools for detecting deepfakes and manipulated media
//! through the Reality Defender API.
//!
//! ## Basic Usage Example
//!
//! ```no_run
//! use realitydefender::{Client, Config, UploadOptions};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize with API key
//!     let client = Client::new(Config {
//!         api_key: std::env::var("REALITY_DEFENDER_API_KEY")?,
//!         ..Default::default()
//!     })?;
//!
//!     // Upload a file for analysis
//!     let upload_result = client.upload(UploadOptions {
//!         file_path: "./image.jpg".to_string(),
//!     }).await?;
//!
//!     // Get the analysis result
//!     let result = client.get_result(&upload_result.request_id, None).await?;
//!     
//!     println!("Status: {}", result.status);
//!     if let Some(score) = result.score {
//!         println!("Score: {:.4} ({:.1}%)", score, score * 100.0);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Getting Results with Pagination
//!
//! ```no_run
//! use realitydefender::{Client, Config, GetResultsOptions};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = Client::new(Config {
//!         api_key: std::env::var("REALITY_DEFENDER_API_KEY")?,
//!         ..Default::default()
//!     })?;
//!
//!     // Get first page of results with filtering
//!     let options = GetResultsOptions {
//!         page_number: Some(0),
//!         size: Some(10),
//!         name: Some("test".to_string()),
//!         start_date: Some("2024-01-01".to_string()),
//!         end_date: Some("2024-12-31".to_string()),
//!         ..Default::default()
//!     };
//!
//!     let results = client.get_results(Some(options)).await?;
//!     
//!     println!("Total Results: {}", results.total_items);
//!     println!("Current Page: {} of {}", results.current_page + 1, results.total_pages);
//!     
//!     for result in &results.items {
//!         println!("Request ID: {}, Status: {}", result.request_id, result.status);
//!         if let Some(score) = result.score {
//!             println!("Score: {:.4}", score);
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```

mod client;
mod config;
mod error;
mod http;
mod models;
mod utils;

// Re-exports
pub use client::Client;
pub use config::Config;
pub use error::{Error, Result};
pub use models::{
    AnalysisResult, BatchOptions, DetectionModel, DetectionResult, DetectionResultList, 
    FormattedDetectionResultList, GetResultOptions, GetResultsOptions, ResultsSummary, 
    UploadOptions, UploadResult,
};
