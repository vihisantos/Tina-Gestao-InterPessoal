//! Provider trait and implementations

use crate::error::Result;

/// Provider interface for model inference.
#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    /// Provider name
    fn name(&self) -> &'static str;
    
    /// Check if provider is available
    async fn is_available(&self) -> bool;
    
    /// Create a chat completion request
    async fn chat_completion(
        &self,
        model: &str,
        messages: Vec<super::route::ChatMessage>,
    ) -> Result<String>;
}

/// Provider error types.
#[derive(Debug, Clone)]
pub enum ProviderError {
    /// Authentication failed
    AuthFailed,
    /// Rate limit exceeded
    RateLimited,
    /// Model not found
    ModelNotFound,
    /// Provider unavailable
    Unavailable,
    /// Other error
    Other(String),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderError::AuthFailed => write!(f, "Authentication failed"),
            ProviderError::RateLimited => write!(f, "Rate limit exceeded"),
            ProviderError::ModelNotFound => write!(f, "Model not found"),
            ProviderError::Unavailable => write!(f, "Provider unavailable"),
            ProviderError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for ProviderError {}

impl From<ProviderError> for crate::error::TinaError {
    fn from(err: ProviderError) -> Self {
        crate::error::TinaError::Other(err.to_string())
    }
}