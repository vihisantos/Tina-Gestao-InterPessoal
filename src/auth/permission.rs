//! Permission engine - autorização de operações

use super::super::agent::{Employee, RoleType};
use crate::error::Result;

/// Permission levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum PermissionLevel {
    /// No access
    None = 0,
    /// Read only
    Read = 1,
    /// Read/Write
    ReadWrite = 2,
    /// Full access
    Full = 3,
}

/// Permission check result.
#[derive(Debug, Clone)]
pub struct PermissionCheck {
    /// Whether access is granted
    pub granted: bool,
    /// Reason for denial (if applicable)
    pub reason: Option<String>,
}

/// Permission engine.
pub struct PermissionEngine {
    /// Owner ID (maximum authority)
    owner_id: String,
}

impl PermissionEngine {
    /// Create a new permission engine.
    pub fn new(owner_id: String) -> Self {
        Self { owner_id }
    }
    
    /// Check if an employee has permission to access another employee.
    pub fn check_employee_access(&self, requester: &Employee, target: &Employee) -> PermissionCheck {
        // Owner has maximum authority
        if requester.id == self.owner_id {
            return PermissionCheck {
                granted: true,
                reason: None,
            };
        }
        
        // Target has authority override
        if target.authority_override {
            return PermissionCheck {
                granted: true,
                reason: None,
            };
        }
        
        // Check company and hierarchy
        if requester.company_name == target.company_name {
            PermissionCheck {
                granted: true,
                reason: None,
            }
        } else {
            PermissionCheck {
                granted: false,
                reason: Some("Cross-company access denied".to_string()),
            }
        }
    }
    
    /// Check if an employee can perform an action.
    pub fn check_action(&self, employee: &Employee, action: &str) -> PermissionCheck {
        // Owner can do anything
        if employee.id == self.owner_id {
            return PermissionCheck {
                granted: true,
                reason: None,
            };
        }
        
        // Action-specific checks would go here
        // For now, grant access if active
        if employee.is_active {
            PermissionCheck {
                granted: true,
                reason: None,
            }
        } else {
            PermissionCheck {
                granted: false,
                reason: Some("Employee is inactive".to_string()),
            }
        }
    }
    
    /// Check if delegation is allowed from source to target.
    pub fn check_delegation(&self, source: &Employee, target: &Employee) -> PermissionCheck {
        let source_check = self.check_employee_access(source, target);
        if !source_check.granted {
            return source_check;
        }
        
        // Check if source is target's supervisor or above
        // This would require checking the hierarchy
        PermissionCheck {
            granted: true,
            reason: None,
        }
    }
}

impl Default for PermissionEngine {
    fn default() -> Self {
        Self::new("owner_default".to_string())
    }
}