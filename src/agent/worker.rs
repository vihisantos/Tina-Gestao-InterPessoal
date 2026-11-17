//! Worker agent implementation

/// Represents a running worker agent.
pub struct WorkerAgent {
    /// Agent ID
    pub id: String,
    /// Current task (if any)
    pub current_task: Option<String>,
    /// Status
    pub status: WorkerStatus,
    /// Last activity timestamp
    pub last_active: chrono::DateTime<chrono::Utc>,
}

/// Worker status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum WorkerStatus {
    /// Idle - waiting for work
    Idle,
    /// Running - executing a task
    Running,
    /// Paused - temporarily stopped
    Paused,
    /// Busy - performing multiple tasks
    Busy,
    /// Error - encountered an error
    Error,
}

impl WorkerAgent {
    /// Create a new worker agent.
    pub fn new(id: String) -> Self {
        Self {
            id,
            current_task: None,
            status: WorkerStatus::Idle,
            last_active: chrono::Utc::now(),
        }
    }
    
    /// Check if the worker is available for new tasks.
    pub fn is_available(&self) -> bool {
        matches!(self.status, WorkerStatus::Idle | WorkerStatus::Paused)
    }
    
    /// Update last active timestamp.
    pub fn touch(&mut self) {
        self.last_active = chrono::Utc::now();
    }
}