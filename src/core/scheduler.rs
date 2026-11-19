//! Task scheduler for Tina Core

use crate::error::Result;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::RwLock;

/// Task priority levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// Critical - execute immediately
    Critical = 0,
    /// High priority
    High = 1,
    /// Normal priority
    Normal = 2,
    /// Low priority
    Low = 3,
    /// Background task
    Background = 4,
}

/// Scheduled task representation.
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    /// Task ID
    pub id: String,
    /// Task description
    pub description: String,
    /// Target employee ID
    pub target: Option<String>,
    /// Priority
    pub priority: Priority,
    /// Due time
    pub due_at: Option<std::time::SystemTime>,
    /// Interval for recurring tasks
    pub interval: Option<Duration>,
    /// Last execution time
    pub last_run: Option<std::time::SystemTime>,
}

/// Task scheduler for Tina Core.
pub struct Scheduler {
    /// Scheduled tasks
    tasks: RwLock<HashMap<String, ScheduledTask>>,
    /// Running tasks
    running: RwLock<HashMap<String, bool>>,
}

impl Scheduler {
    /// Create a new scheduler.
    pub fn new() -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
            running: RwLock::new(HashMap::new()),
        }
    }

    /// Schedule a new task.
    pub async fn schedule(&self, task: ScheduledTask) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        tasks.insert(task.id.clone(), task);
        Ok(())
    }

    /// Cancel a scheduled task.
    pub async fn cancel(&self, task_id: &str) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        tasks.remove(task_id);
        Ok(())
    }

    /// Get next tasks to execute.
    pub async fn next_tasks(&self, limit: usize) -> Vec<ScheduledTask> {
        let tasks = self.tasks.read().await;
        let now = std::time::SystemTime::now();

        let mut pending: Vec<_> = tasks
            .values()
            .filter_map(|t| {
                if t.due_at.map_or(true, |d| d <= now) {
                    Some(t.clone())
                } else {
                    None
                }
            })
            .collect();

        pending.sort_by_key(|t| t.priority as i32);
        pending.truncate(limit);
        pending
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}
