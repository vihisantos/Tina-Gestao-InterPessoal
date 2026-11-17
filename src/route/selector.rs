//! Model selector - chooses model based on task requirements

use super::{ModelCriteria, Complexity, Requirement};

/// Selects the most appropriate model.
pub struct ModelSelector {
    /// Cost per 1M tokens by provider
    cost_per_1m: std::collections::HashMap<String, (f64, f64)>, // input, output
}

impl ModelSelector {
    /// Create a new model selector.
    pub fn new() -> Self {
        Self {
            cost_per_1m: [(
                "nvidia/nemotron-3.5-bolt-e4b",
                (0.0001, 0.0001),
            )].iter().cloned().collect(),
        }
    }
    
    /// Score a model for given criteria.
    pub fn score_model(&self, model: &str, criteria: &ModelCriteria) -> f32 {
        let (input_cost, output_cost) = self.cost_per_1m.get(model).copied().unwrap_or((0.001, 0.001));
        
        let mut score = 1.0;
        
        // Penalize expensive models if cost-sensitive
        if criteria.cost_sensitive {
            score -= (input_cost + output_cost) * 0.1;
        }
        
        // Penalize smaller models for complex tasks
        if matches!(criteria.complexity, Complexity::High) && model.contains("nano") {
            score -= 0.3;
        }
        
        score.clamp(0.1, 1.0)
    }
    
    /// Select model based on criteria.
    pub fn select(&self, models: &[&str], criteria: &ModelCriteria) -> &str {
        models
            .iter()
            .max_by(|a, b| self.score_model(a, criteria).partial_cmp(&self.score_model(b, criteria)).unwrap())
            .copied()
            .unwrap_or("nvidia/nemotron-3.5-bolt-e4b")
    }
}

impl Default for ModelSelector {
    fn default() -> Self {
        Self::new()
    }
}