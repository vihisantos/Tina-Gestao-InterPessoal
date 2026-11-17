//! Dependency management for tasks

use crate::error::Result;
use std::collections::{HashMap, HashSet};

/// Manage task dependencies.
pub struct DependencyGraph {
    /// Task dependencies: task_id -> set of task_ids it depends on
    dependencies: HashMap<String, HashSet<String>>,
    /// Task dependents: task_id -> set of task_ids that depend on it
    dependents: HashMap<String, HashSet<String>>,
}

impl DependencyGraph {
    /// Create a new dependency graph.
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            dependents: HashMap::new(),
        }
    }
    
    /// Add a dependency: `task` depends on `dependency`.
    pub fn add_dependency(&mut self, task: &str, dependency: &str) -> Result<()> {
        self.dependencies
            .entry(task.to_string())
            .or_insert_with(HashSet::new)
            .insert(dependency.to_string());
        
        self.dependents
            .entry(dependency.to_string())
            .or_insert_with(HashSet::new)
            .insert(task.to_string());
        
        Ok(())
    }
    
    /// Get dependencies for a task.
    pub fn get_dependencies(&self, task: &str) -> Vec<String> {
        self.dependencies
            .get(task)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }
    
    /// Get tasks that depend on this task.
    pub fn get_dependents(&self, task: &str) -> Vec<String> {
        self.dependents
            .get(task)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }
    
    /// Check if all dependencies are satisfied.
    pub fn is_ready(&self, task: &str, completed: &HashSet<String>) -> bool {
        self.dependencies
            .get(task)
            .map(|deps| deps.iter().all(|d| completed.contains(d)))
            .unwrap_or(true)
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}