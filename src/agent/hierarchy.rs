//! Hierarchy engine - manages organizational hierarchy

use crate::error::Result;
use std::collections::HashMap;

/// Manages organizational hierarchy.
pub struct HierarchyEngine {
    /// Hierarchy tree: employee_id -> children
    children: HashMap<String, Vec<String>>,
    /// Parent mapping: employee_id -> parent_id
    parents: HashMap<String, Option<String>>,
}

impl HierarchyEngine {
    /// Create a new hierarchy engine.
    pub fn new() -> Self {
        Self {
            children: HashMap::new(),
            parents: HashMap::new(),
        }
    }
    
    /// Add a subordinate relationship.
    pub fn add_subordinate(&mut self, manager_id: &str, subordinate_id: &str) {
        self.children
            .entry(manager_id.to_string())
            .or_insert_with(Vec::new)
            .push(subordinate_id.to_string());
        
        self.parents.insert(subordinate_id.to_string(), Some(manager_id.to_string()));
    }
    
    /// Remove a subordinate relationship.
    pub fn remove_subordinate(&mut self, manager_id: &str, subordinate_id: &str) {
        if let Some(children) = self.children.get_mut(manager_id) {
            children.retain(|id| id != subordinate_id);
        }
        self.parents.insert(subordinate_id.to_string(), None);
    }
    
    /// Get subordinates of a manager.
    pub fn get_subordinates(&self, manager_id: &str) -> Vec<String> {
        self.children
            .get(manager_id)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Get supervisor of an employee.
    pub fn get_supervisor(&self, employee_id: &str) -> Option<String> {
        self.parents.get(employee_id).cloned().flatten()
    }
    
    /// Get the hierarchy path to root.
    pub fn get_path_to_root(&self, employee_id: &str) -> Result<Vec<String>> {
        let mut path = vec![employee_id.to_string()];
        let mut current = employee_id.to_string();
        
        while let Some(parent) = self.get_supervisor(&current) {
            if path.len() > 100 {
                return Err(crate::error::TinaError::Other("Hierarchy too deep".to_string()));
            }
            path.push(parent.clone());
            current = parent;
        }
        
        Ok(path)
    }
}

impl Default for HierarchyEngine {
    fn default() -> Self {
        Self::new()
    }
}