use crate::error::{Error, Result};

/// Default API base URL
pub const DEFAULT_BASE_URL: &str = "https://api.prd.realitydefender.xyz";

/// Configuration for the Reality Defender client
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// API key for authentication
    pub api_key: String,

    /// Base URL for the API
    pub base_url: Option<String>,

    /// Timeout in seconds for HTTP requests
    pub timeout_seconds: Option<u64>,
}

impl Config {
    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.api_key.trim().is_empty() {
            return Err(Error::InvalidConfig("API key is required".to_string()));
        }

        if let Some(url) = &self.base_url {
            if url.trim().is_empty() {
                return Err(Error::InvalidConfig("Base URL cannot be empty".to_string()));
            }
        }

        Ok(())
    }

    /// Get the base URL, falling back to the default if not set
    pub fn get_base_url(&self) -> String {
        self.base_url
            .clone()
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
    }

    /// Get the timeout in seconds, falling back to the default if not set
    pub fn get_timeout_seconds(&self) -> u64 {
        self.timeout_seconds.unwrap_or(30)
    }
}
