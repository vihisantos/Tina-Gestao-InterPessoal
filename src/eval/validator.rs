//! Result validator

use crate::error::Result;

/// Validator for checking result quality.
pub struct Validator {
    /// Minimum length for valid results
    min_length: usize,
    /// Maximum length for valid results
    max_length: usize,
    /// Reject empty content
    reject_empty: bool,
}

impl Validator {
    /// Create a new validator.
    pub fn new() -> Self {
        Self {
            min_length: 10,
            max_length: 100_000,
            reject_empty: true,
        }
    }
    
    /// Validate a result.
    pub fn validate(&self, content: &str) -> Result<bool> {
        if content.len() < self.min_length {
            return Ok(false);
        }
        
        if content.len() > self.max_length {
            return Ok(false);
        }
        
        if self.reject_empty && content.trim().is_empty() {
            return Ok(false);
        }
        
        Ok(true)
    }
    
    /// Validate with detailed feedback.
    pub fn validate_detailed(&self, content: &str) -> ValidationResult {
        let issues = Vec::new();
        
        if content.len() < self.min_length {
            return ValidationResult {
                valid: false,
                issues: vec![format!("Content too short ({} < {})", content.len(), self.min_length)],
            };
        }
        
        if content.len() > self.max_length {
            return ValidationResult {
                valid: false,
                issues: vec![format!("Content too long ({} > {})", content.len(), self.max_length)],
            };
        }
        
        if self.reject_empty && content.trim().is_empty() {
            return ValidationResult {
                valid: false,
                issues: vec!["Content is empty".to_string()],
            };
        }
        
        ValidationResult {
            valid: true,
            issues: Vec::new(),
        }
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

/// Validation result with details.
pub struct ValidationResult {
    /// Whether the content is valid
    pub valid: bool,
    /// Issues found
    pub issues: Vec<String>,
}