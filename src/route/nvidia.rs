//! NVIDIA NIM API Provider

use crate::error::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// NVIDIA NIM API client.
pub struct NvidiaNimClient {
    /// HTTP client
    client: Client,
    /// API key
    api_key: String,
    /// Base URL
    base_url: String,
    /// Timeout
    timeout: Duration,
}

/// Chat message for NVIDIA API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Role: user, assistant, or system
    pub role: String,
    /// Message content
    pub content: String,
}

/// Chat completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    /// Model to use
    pub model: String,
    /// Messages
    pub messages: Vec<ChatMessage>,
    /// Maximum tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Temperature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Stream responses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// Chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    /// Response ID
    pub id: String,
    /// Model used
    pub model: String,
    /// Choices
    pub choices: Vec<ChatChoice>,
    /// Usage statistics
    pub usage: Usage,
}

/// A choice in the response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    /// Index
    pub index: u32,
    /// Message
    pub message: ChatMessage,
    /// Finish reason
    pub finish_reason: Option<String>,
}

/// Usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Prompt tokens
    pub prompt_tokens: u32,
    /// Completion tokens
    pub completion_tokens: u32,
    /// Total tokens
    pub total_tokens: u32,
}

impl NvidiaNimClient {
    /// Create a new NVIDIA NIM client.
    pub fn new(api_key: String, base_url: String, timeout_seconds: u64) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(timeout_seconds))
                .build()
                .expect("Failed to create HTTP client"),
            api_key,
            base_url,
            timeout: Duration::from_secs(timeout_seconds),
        }
    }
    
    /// Check if the API is accessible.
    pub async fn health_check(&self) -> Result<bool> {
        let response = self.client
            .get(format!("{}/v1/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;
        
        Ok(response.status().is_success())
    }
    
    /// Create a chat completion.
    pub async fn chat_completion(&self, request: &ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        let response = self.client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await?
            .json::<ChatCompletionResponse>()
            .await?;
        
        Ok(response)
    }
    
    /// Create a streaming chat completion.
    pub async fn chat_completion_stream(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<impl futures::Stream<Item = Result<String>>> {
        let request = ChatCompletionRequest {
            stream: Some(true),
            ..request.clone()
        };
        
        let response = self.client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(crate::error::TinaError::Inference(format!(
                "API error: {}",
                response.status()
            )));
        }
        
        let stream = response.bytes_stream()
            .map(|result| {
                result
                    .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
                    .map_err(|e| crate::error::TinaError::Other(e.to_string()))
            });
        
        Ok(stream)
    }
}