//! Task decomposition utilities

use crate::error::Result;

/// Decomposes complex objectives into simpler tasks.
pub struct Decomposer {
    /// Maximum depth of decomposition
    max_depth: usize,
}

impl Decomposer {
    /// Create a new decomposer.
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }
    
    /// Decompose an objective into sub-tasks.
    pub async fn decompose(&self, objective: &str) -> Result<Vec<SubTask>> {
        // TODO: Implement decomposition logic using NLP
        Ok(vec![SubTask {
            id: uuid::Uuid::new_v4().to_string(),
            description: objective.to_string(),
            sub_tasks: vec![],
        }])
    }
}

impl Default for Decomposer {
    fn default() -> Self {
        Self::new(5)
    }
}

/// A sub-task in a decomposition.
#[derive(Debug, Clone)]
pub struct SubTask {
    /// Task ID
    pub id: String,
    /// Description
    pub description: String,
    /// Child tasks
    pub sub_tasks: Vec<SubTask>,
}