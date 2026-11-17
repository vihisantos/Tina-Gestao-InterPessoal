//! Session management

use super::Identity;
use std::time::{SystemTime, Duration};

/// Active session.
#[derive(Debug, Clone)]
pub struct Session {
    /// Session token
    pub token: String,
    /// Identity
    pub identity: Identity,
    /// Created at
    pub created_at: SystemTime,
    /// Expires at
    pub expires_at: SystemTime,
    /// Last activity
    pub last_activity: SystemTime,
}

impl Session {
    /// Create a new session.
    pub fn new(token: String, identity: Identity, timeout_seconds: u64) -> Self {
        let now = SystemTime::now();
        Self {
            token,
            identity,
            created_at: now,
            expires_at: now + Duration::from_secs(timeout_seconds),
            last_activity: now,
        }
    }
    
    /// Check if session is still valid.
    pub fn is_valid(&self) -> bool {
        SystemTime::now() < self.expires_at
    }
    
    /// Extend session timeout.
    pub fn extend(&mut self, timeout_seconds: u64) {
        self.expires_at = SystemTime::now() + Duration::from_secs(timeout_seconds);
        self.last_activity = SystemTime::now();
    }
    
    /// Touch the session (update last activity).
    pub fn touch(&mut self) {
        self.last_activity = SystemTime::now();
    }
}