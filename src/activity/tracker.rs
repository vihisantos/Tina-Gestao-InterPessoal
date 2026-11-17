//! Activity tracker - tracks the current state of operations

use std::collections::HashMap;
use std::time::SystemTime;
use serde::{Serialize, Deserialize};

/// Tracked activity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    /// Activity ID
    pub id: String,
    /// Type of activity
    pub r#type: ActivityType,
    /// Current status
    pub status: ActivityStatus,
    /// Timestamps
    pub started_at: SystemTime,
    pub last_updated: SystemTime,
    /// Owner (delegante)
    pub owner: Option<String>,
    /// Current executor
    pub executor: Option<String>,
    /// Description
    pub description: String,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Type of activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum ActivityType {
    /// Task execution
    Task,
    /// Tool execution
    Tool,
    /// Delegation
    Delegation,
    /// Conversation
    Conversation,
    /// Planning
    Planning,
}

/// Status of an activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum ActivityStatus {
    /// Not started
    Pending,
    /// In progress
    InProgress,
    /// Paused
    Paused,
    /// Completed
    Completed,
    /// Interrupted
    Interrupted,
    /// Cancelled
    Cancelled,
    /// Error
    Error,
}

/// Activity tracker.
pub struct ActivityTracker {
    /// Tracked activities
    activities: HashMap<String, Activity>,
}

impl ActivityTracker {
    /// Create a new activity tracker.
    pub fn new() -> Self {
        Self {
            activities: HashMap::new(),
        }
    }
    
    /// Start tracking an activity.
    pub fn start(&mut self, activity: Activity) {
        self.activities.insert(activity.id.clone(), activity);
    }
    
    /// Update activity status.
    pub fn update_status(&mut self, id: &str, status: ActivityStatus) {
        if let Some(activity) = self.activities.get_mut(id) {
            activity.status = status;
            activity.last_updated = SystemTime::now();
        }
    }
    
    /// Get activity status.
    pub fn get(&self, id: &str) -> Option<&Activity> {
        self.activities.get(id)
    }
    
    /// List activities by status.
    pub fn list_by_status(&self, status: ActivityStatus) -> Vec<&Activity> {
        self.activities
            .values()
            .filter(|a| a.status == status)
            .collect()
    }
    
    /// List activities by owner.
    pub fn list_by_owner(&self, owner: &str) -> Vec<&Activity> {
        self.activities
            .values()
            .filter(|a| a.owner.as_deref() == Some(owner))
            .collect()
    }
}

impl Default for ActivityTracker {
    fn default() -> Self {
        Self::new()
    }
}