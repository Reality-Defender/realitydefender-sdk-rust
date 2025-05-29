//! # Reality Defender SDK
//!
//! The Reality Defender SDK provides tools for detecting deepfakes and manipulated media
//! through the Reality Defender API.
//!
//! ## Example
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
//!         metadata: None,
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

mod client;
mod config;
mod error;
mod http;
mod models;
pub mod utils;

// Re-exports
pub use client::Client;
pub use config::Config;
pub use error::{Error, Result};
pub use models::{
    AnalysisResult, BatchOptions, DetectionModel, GetResultOptions, ResultsSummary, UploadOptions,
    UploadResult,
};
