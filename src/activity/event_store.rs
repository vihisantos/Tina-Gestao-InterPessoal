//! Event store for persisting activities

use crate::error::Result;
use sqlx::SqlitePool;
use super::{ActivityEvent, EventType, EventSeverity};

/// Event store for persisting events.
pub struct EventStore {
    /// Database pool
    db: SqlitePool,
}

impl EventStore {
    /// Create a new event store.
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
    
    /// Store an event.
    pub async fn store(&self, event: &ActivityEvent) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO events (id, installation_id, employee_id, delegation_id, event_type, severity, payload, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))
            "#,
            event.id,
            "" as &str, // installation_id
            event.employee_id.clone().unwrap_or_default(),
            event.delegation_id.clone().unwrap_or_default(),
            format!("{:?}", event.event_type),
            format!("{:?}", event.severity),
            serde_json::to_string(&event.data)
        ).execute(&self.db).await?;
        
        Ok(())
    }
    
    /// Get events by delegation.
    pub async fn get_by_delegation(&self, delegation_id: &str, limit: usize) -> Result<Vec<ActivityEvent>> {
        sqlx::query!(
            r#"
            SELECT id, event_type, employee_id, delegation_id, severity, payload, created_at
            FROM events
            WHERE delegation_id = ?
            ORDER BY created_at ASC
            LIMIT ?
            "#,
            delegation_id,
            limit
        ).fetch_all(&self.db).await?;
        
        // TODO: Map to ActivityEvent
        Ok(vec![])
    }
    
    /// Get events by employee.
    pub async fn get_by_employee(&self, employee_id: &str, limit: usize) -> Result<Vec<ActivityEvent>> {
        sqlx::query!(
            r#"
            SELECT id, event_type, employee_id, delegation_id, severity, payload, created_at
            FROM events
            WHERE employee_id = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
            employee_id,
            limit
        ).fetch_all(&self.db).await?;
        
        Ok(vec![])
    }
}