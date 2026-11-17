//! Employee entity - represents a worker agent

use serde::{Deserialize, Serialize};

/// An employee in the Tina system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Employee {
    /// Unique employee ID
    pub id: String,
    /// Employee name
    pub name: String,
    /// Position ID
    pub position_id: String,
    /// Position details (denormalized for read efficiency)
    pub position_name: String,
    /// Department name
    pub department_name: String,
    /// Company name
    pub company_name: String,
    /// Model configuration (provider:model or provider:model:version)
    pub model_config: String,
    /// Specialty
    pub specialty: Option<String>,
    /// Description/Personality
    pub description: Option<String>,
    /// Supervisor ID (for hierarchy)
    pub supervisor_id: Option<String>,
    /// Is active
    pub is_active: bool,
    /// Authority override (can access any employee directly)
    pub authority_override: bool,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Employee {
    /// Create a new employee.
    pub fn new(id: String, name: String, position_id: String) -> Self {
        Self {
            id,
            name,
            position_id,
            position_name: String::new(),
            department_name: String::new(),
            company_name: String::new(),
            model_config: "nvidia/nemotron-3.5-bolt-e4b".to_string(),
            specialty: None,
            description: None,
            supervisor_id: None,
            is_active: true,
            authority_override: false,
            created_at: chrono::Utc::now(),
        }
    }
    
    /// Check if this employee can access another employee.
    pub fn can_access(&self, target: &Employee, owner_id: &str) -> bool {
        // Owner has maximum authority
        if self.id == owner_id {
            return true;
        }
        
        // Active employees in same company can coordinate
        self.company_name == target.company_name && self.is_active && target.is_active
    }
}

/// Position (cargo) definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Position ID
    pub id: String,
    /// Position name (cargo)
    pub name: String,
    /// Level in hierarchy
    pub level: u32,
    /// Authority level (higher = more authority)
    pub authority_level: u32,
    /// Department this belongs to
    pub department_id: String,
    /// Standard model for this position
    pub standard_model: String,
    /// Description
    pub description: Option<String>,
}

impl Position {
    /// Create a new position.
    pub fn new(id: String, name: String, level: u32, authority_level: u32) -> Self {
        Self {
            id,
            name,
            level,
            authority_level,
            department_id: String::new(),
            standard_model: "nvidia/nemotron-3.5-bolt-e4b".to_string(),
            description: None,
        }
    }
}

/// Role definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// Base agent
    Agent,
    /// Team leader
    TeamLead,
    /// Department head
    DepartmentHead,
    /// Executive
    Executive,
    /// Specialist
    Specialist,
    /// Specialist analyst
    Analyst,
    /// Intern
    Intern,
}

impl Role {
    /// Get the authority level for this role.
    pub fn authority_level(&self) -> u32 {
        match self {
            Role::Agent => 1,
            Role::TeamLead => 5,
            Role::DepartmentHead => 10,
            Role::Executive => 20,
            Role::Specialist => 3,
            Role::Analyst => 2,
            Role::Intern => 1,
        }
    }
}