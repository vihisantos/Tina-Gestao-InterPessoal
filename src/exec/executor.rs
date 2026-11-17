//! Execution Engine - executes tasks and tools

use crate::error::Result;
use super::{ToolRegistry, Sandbox, SandboxConfig};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Execution engine state.
#[derive(Debug, Clone)]
pub struct ExecutionState {
    /// Unique identifier
    pub id: String,
    /// Current status
    pub status: ExecutionStatus,
    /// Progress (0.0 to 1.0)
    pub progress: f32,
    /// Last error
    pub last_error: Option<String>,
}

/// Execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum ExecutionStatus {
    /// Not started
    Pending,
    /// Running
    Running,
    /// Paused
    Paused,
    /// Completed
    Completed,
    /// Error
    Error,
    /// Cancelled
    Cancelled,
}

/// Execution engine.
pub struct ExecutionEngine {
    /// Tool registry
    tools: ToolRegistry,
    /// Sandbox
    sandbox: Sandbox,
    /// Active executions
    state: RwLock<std::collections::HashMap<String, ExecutionState>>,
}

impl ExecutionEngine {
    /// Create a new execution engine.
    pub fn new() -> Self {
        Self {
            tools: ToolRegistry::new(),
            sandbox: Sandbox::new(SandboxConfig::default()),
            state: RwLock::new(std::collections::HashMap::new()),
        }
    }
    
    /// Register a tool.
    pub fn register_tool<T: super::tool_registry::Tool + 'static>(&mut self, tool: T) {
        self.tools.register(tool);
    }
    
    /// Execute a tool synchronously.
    pub async fn execute_tool(&self, tool: &str, params: &str) -> Result<String> {
        self.sandbox.execute(tool, params).await
    }
    
    /// Create an execution with state tracking.
    pub async fn create_execution(&self, id: String) -> Result<()> {
        let mut state = self.state.write().await;
        state.insert(id, ExecutionState {
            id: id.clone(),
            status: ExecutionStatus::Pending,
            progress: 0.0,
            last_error: None,
        });
        Ok(())
    }
    
    /// Start an execution.
    pub async fn start_execution(&self, id: &str) -> Result<()> {
        let mut state = self.state.write().await;
        if let Some(s) = state.get_mut(id) {
            s.status = ExecutionStatus::Running;
            s.progress = 0.1;
        }
        Ok(())
    }
    
    /// Pause an execution.
    pub async fn pause_execution(&self, id: &str) -> Result<()> {
        let mut state = self.state.write().await;
        if let Some(s) = state.get_mut(id) {
            s.status = ExecutionStatus::Paused;
        }
        Ok(())
    }
    
    /// Cancel an execution.
    pub async fn cancel_execution(&self, id: &str) -> Result<()> {
        let mut state = self.state.write().await;
        if let Some(s) = state.get_mut(id) {
            s.status = ExecutionStatus::Cancelled;
        }
        Ok(())
    }
    
    /// Get execution state.
    pub async fn get_state(&self, id: &str) -> Option<ExecutionState> {
        self.state.read().await.get(id).cloned()
    }
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self::new()
    }
}