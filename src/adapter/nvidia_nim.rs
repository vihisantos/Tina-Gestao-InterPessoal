//! NVIDIA NIM API adapter

use crate::error::Result;
use reqwest::Client;
use std::time::Duration;

/// NVIDIA NIM adapter configuration.
#[derive(Debug, Clone)]
pub struct NvidiaNimAdapterConfig {
    /// API key (should be loaded from secure storage)
    pub api_key: String,
    /// Base URL
    pub base_url: String,
    /// Request timeout
    pub timeout_seconds: u64,
    /// Default model
    pub default_model: String,
}

impl Default for NvidiaNimAdapterConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("NVIDIA_API_KEY").unwrap_or_default(),
            base_url: "https://integrate.api.nvidia.com/v1".to_string(),
            timeout_seconds: 60,
            default_model: "nvidia/nemotron-3.5-bolt-e4b".to_string(),
        }
    }
}

/// NVIDIA NIM API adapter.
pub struct NvidiaNimAdapter {
    config: NvidiaNimAdapterConfig,
    client: Client,
}

impl NvidiaNimAdapter {
    /// Create a new NVIDIA NIM adapter.
    pub fn new(config: NvidiaNimAdapterConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");
        
        Self { config, client }
    }
    
    /// Check if the adapter is configured and available.
    pub async fn is_available(&self) -> bool {
        if self.config.api_key.is_empty() {
            return false;
        }
        
        self.health_check().await.is_ok()
    }
    
    /// Health check.
    pub async fn health_check(&self) -> Result<()> {
        let response = self.client
            .get(format!("{}/v1/models", self.config.base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await?
            .error_for_status()?;
        
        let models: serde_json::Value = response.json().await?;
        if models["data"].as_array().map_or(false, |m| !m.is_empty()) {
            Ok(())
        } else {
            Err(crate::error::TinaError::Other("No models available".to_string()))
        }
    }
    
    /// Get the API base URL.
    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }
    
    /// Get the default model.
    pub fn default_model(&self) -> &str {
        &self.config.default_model
    }
}

impl Default for NvidiaNimAdapter {
    fn default() -> Self {
        Self::new(NvidiaNimAdapterConfig::default())
    }
}