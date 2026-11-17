//! Event bus for activity tracking

use tokio::sync::broadcast::{self, Sender, Receiver};
use serde::{Serialize, Deserialize};
use std::time::SystemTime;

/// Event types in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    /// Delegation created
    DelegationCreated,
    /// Delegation started
    DelegationStarted,
    /// Delegation completed
    DelegationCompleted,
    /// Delegation paused
    DelegationPaused,
    /// Delegation interrupted
    DelegationInterrupted,
    /// Delegation cancelled
    DelegationCancelled,
    /// Tool executed
    ToolExecuted,
    /// Permission granted
    PermissionGranted,
    /// Error occurred
    ErrorOccurred,
    /// Owner command
    OwnerCommand,
}

/// Severity levels for events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventSeverity {
    /// Informational
    Info,
    /// Warning
    Warning,
    /// Error
    Error,
    /// Critical
    Critical,
}

/// Activity event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEvent {
    /// Event ID
    pub id: String,
    /// Event type
    pub event_type: EventType,
    /// Related entity ID
    pub entity_id: Option<String>,
    /// Related delegation ID
    pub delegation_id: Option<String>,
    /// Employee ID
    pub employee_id: Option<String>,
    /// Severity
    pub severity: EventSeverity,
    /// Event data
    pub data: serde_json::Value,
    /// Timestamp
    pub created_at: SystemTime,
}

impl ActivityEvent {
    /// Create a new event.
    pub fn new(event_type: EventType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            event_type,
            entity_id: None,
            delegation_id: None,
            employee_id: None,
            severity: EventSeverity::Info,
            data: serde_json::Value::Null,
            created_at: SystemTime::now(),
        }
    }
}

/// Event bus for publishing and subscribing to events.
pub struct EventBus {
    /// Broadcast sender for events
    sender: Sender<ActivityEvent>,
}

impl EventBus {
    /// Create a new event bus.
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self { sender }
    }
    
    /// Publish an event.
    pub fn publish(&self, event: ActivityEvent) {
        let _ = self.sender.send(event);
    }
    
    /// Subscribe to events.
    pub fn subscribe(&self) -> Receiver<ActivityEvent> {
        self.sender.subscribe()
    }
    
    /// Subscribe to specific event types.
    pub fn subscribe_type(&self, event_type: EventType) -> Receiver<ActivityEvent> {
        let mut receiver = self.sender.subscribe();
        // Note: Filtering should be done by the consumer
        receiver
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}