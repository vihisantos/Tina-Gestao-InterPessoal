//! Model Router - selects the best model for a request

use crate::error::Result;
use crate::route::NvidiaNimClient;
use std::collections::HashMap;

/// Model Router selects the appropriate model for each task.
pub struct ModelRouter {
    /// NVIDIA client
    nvidia: NvidiaNimClient,
    /// Model registry by role
    model_registry: HashMap<String, String>,
    /// Current model selections (for override)
    overrides: HashMap<String, String>,
}

/// Model selection criteria.
#[derive(Debug, Clone)]
pub struct ModelCriteria {
    /// Task complexity: low, medium, high
    pub complexity: Complexity,
    /// Special requirements: reasoning, coding, etc.
    pub requirements: Vec<Requirement>,
    /// Cost sensitivity
    pub cost_sensitive: bool,
}

/// Task complexity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Complexity {
    /// Simple - quick responses
    Low,
    /// Medium - requires reasoning
    Medium,
    /// High - complex, multi-step
    High,
}

/// Model requirements.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Requirement {
    /// Needs reasoning capabilities
    Reasoning,
    /// Needs tool calling
    ToolCalling,
    /// Needs coding capabilities
    Coding,
    /// Needs vision (multimodal)
    Vision,
}

impl ModelRouter {
    /// Create a new model router.
    pub fn new(nvidia: NvidiaNimClient) -> Self {
        Self {
            nvidia,
            model_registry: HashMap::new(),
            overrides: HashMap::new(),
        }
    }
    
    /// Register a default model for a role/position.
    pub fn register_model(&mut self, role: &str, model: &str) {
        self.model_registry.insert(role.to_string(), model.to_string());
    }
    
    /// Set a temporary override for a task.
    pub fn set_override(&mut self, employee_id: &str, model: String) {
        self.overrides.insert(employee_id.to_string(), model);
    }
    
    /// Clear override for an employee.
    pub fn clear_override(&mut self, employee_id: &str) {
        self.overrides.remove(employee_id);
    }
    
    /// Select the best model for an employee and task.
    pub fn select_model(&self, employee_id: &str, role: &str, criteria: &ModelCriteria) -> String {
        // Check for override first
        if let Some(model) = self.overrides.get(employee_id) {
            return model.clone();
        }
        
        // Get default model for role
        let default_model = self.model_registry.get(role)
            .cloned()
            .unwrap_or_else(|| "nvidia/nemotron-3.5-bolt-e4b".to_string());
        
        // Adjust based on criteria
        if matches!(criteria.complexity, Complexity::High) || 
           criteria.requirements.contains(&Requirement::Reasoning) {
            "nvidia/nemotron-3.5-lightning-30b-a3b".to_string()
        } else {
            default_model
        }
    }
    
    /// Get the NVIDIA client (for direct API calls).
    pub fn nvidia(&self) -> &NvidiaNimClient {
        &self.nvidia
    }
}

impl Default for ModelRouter {
    fn default() -> Self {
        let nvidia = NvidiaNimClient::new(
            std::env::var("NVIDIA_API_KEY").unwrap_or_default(),
            "https://integrate.api.nvidia.com/v1".to_string(),
            60,
        );
        
        let mut router = Self::new(nvidia);
        
        // Register default models by role
        router.register_model("agent_base", "nvidia/nemotron-3.5-bolt-e4b");
        router.register_model("team_lead", "nvidia/nemotron-3.5-lightning-30b-a3b");
        router.register_model("dept_head", "nvidia/nemotron-3.5-super-vl-120b-a12b");
        router.register_model("executive", "nvidia/nemotron-3-ultra-550b-a55b");
        
        router
    }
}