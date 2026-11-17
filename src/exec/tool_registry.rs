//! Tool registry - registry of available tools

use crate::error::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Tool trait - all tools must implement this.
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool name.
    fn name(&self) -> &str;
    
    /// Get tool description.
    fn description(&self) -> &str;
    
    /// Execute the tool with given parameters.
    async fn execute(&self, params: &str) -> Result<ToolResult>;
    
    /// Get required permissions.
    fn required_permissions(&self) -> Vec<String> {
        vec![]
    }
}

/// Tool execution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Whether execution was successful
    pub success: bool,
    /// Result content
    pub content: String,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Execution time in ms
    pub execution_time_ms: u64,
}

/// Tool registry for managing available tools.
pub struct ToolRegistry {
    /// Registered tools
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a new tool registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }
    
    /// Register a tool.
    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.name().to_string(), Box::new(tool));
    }
    
    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<&Box<dyn Tool>> {
        self.tools.get(name)
    }
    
    /// List all tools.
    pub fn list(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }
    
    /// Execute a tool.
    pub async fn execute(&self, name: &str, params: &str) -> Result<ToolResult> {
        self.tools
            .get(name)
            .ok_or_else(|| crate::error::TinaError::Other(format!("Tool not found: {}", name)))
            .and_then(|t| t.execute(params).map_err(|e| e))
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}