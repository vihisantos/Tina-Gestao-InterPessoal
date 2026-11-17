//! Identity and user management

use serde::{Serialize, Deserialize};
use std::time::SystemTime;

/// User identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// Unique identifier
    pub id: String,
    /// Username
    pub username: String,
    /// Email (optional)
    pub email: Option<String>,
    /// Display name
    pub display_name: String,
    /// Role
    pub role: IdentityRole,
    /// Created at
    pub created_at: SystemTime,
    /// Last login
    pub last_login: Option<SystemTime>,
}

/// Identity role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum IdentityRole {
    /// Owner (maximum authority)
    Owner,
    /// Administrator
    Administrator,
    /// User
    User,
    /// Guest
    Guest,
}

impl Identity {
    /// Create a new identity.
    pub fn new(id: String, username: String, role: IdentityRole) -> Self {
        Self {
            id,
            username,
            email: None,
            display_name: username.clone(),
            role,
            created_at: SystemTime::now(),
            last_login: None,
        }
    }
}