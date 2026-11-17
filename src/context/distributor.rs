//! Context distributor - distributes context to agents

use crate::error::Result;
use crate::context::ContextItem;

/// Distributes context to appropriate agents.
pub struct ContextDistributor {
    /// Maximum context size per agent (tokens)
    max_context_tokens: usize,
}

impl ContextDistributor {
    /// Create a new context distributor.
    pub fn new(max_context_tokens: usize) -> Self {
        Self {
            max_context_tokens,
        }
    }
    
    /// Distribute context to an agent.
    pub async fn distribute(&self, items: &[ContextItem], total_tokens: usize) -> Result<Vec<ContextItem>> {
        // TODO: Implement context distribution with token limits
        let mut result = Vec::new();
        let mut current_tokens = 0;
        
        for item in items {
            let item_tokens = item.content.len() / 4; // Rough estimate
            if current_tokens + item_tokens > self.max_context_tokens {
                break;
            }
            result.push(item.clone());
            current_tokens += item_tokens;
        }
        
        Ok(result)
    }
}

impl Default for ContextDistributor {
    fn default() -> Self {
        Self::new(8000) // Default to 8000 tokens
    }
}