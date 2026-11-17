//! Agent Manager - creates and manages employee agents

use crate::error::Result;
use crate::agent::{Employee, Position};
use std::sync::Arc;
use sqlx::SqlitePool;
use dashmap::DashMap;

/// Manages employee agents.
pub struct AgentManager {
    /// Database pool
    db: SqlitePool,
    /// Active employees cache
    employees: DashMap<String, Employee>,
}

impl AgentManager {
    /// Create a new agent manager.
    pub fn new(db: SqlitePool) -> Self {
        Self {
            db,
            employees: DashMap::new(),
        }
    }
    
    /// Create a new employee.
    pub async fn create_employee(&self, name: &str, position_id: &str) -> Result<Employee> {
        let id = uuid::Uuid::new_v4().to_string();
        
        let employee = Employee::new(id.clone(), name.to_string(), position_id.to_string());
        
        // Store in database
        sqlx::query!(
            r#"
            INSERT INTO employees (id, name, position_id, model_config, created_at)
            VALUES (?, ?, ?, ?, datetime('now'))
            "#,
            id, name, position_id, "nvidia/nemotron-3.5-bolt-e4b"
        ).execute(&self.db).await?;
        
        // Cache
        self.employees.insert(id.clone(), employee.clone());
        
        Ok(employee)
    }
    
    /// Get an employee by ID.
    pub async fn get_employee(&self, id: &str) -> Result<Option<Employee>> {
        // Check cache first
        if let Some(employee) = self.employees.get(id) {
            return Ok(Some(employee.clone()));
        }
        
        // Query database
        let employee = sqlx::query_as!(
            Employee,
            r#"
            SELECT id, name, position_id, position_name, department_name, 
                   company_name, model_config, specialty, description, 
                   supervisor_id, is_active, authority_override, created_at
            FROM employees 
            WHERE id = ? AND is_active = 1
            "#,
            id
        ).fetch_optional(&self.db).await?;
        
        if let Some(ref emp) = employee {
            self.employees.insert(id.to_string(), emp.clone());
        }
        
        Ok(employee)
    }
    
    /// List employees by position.
    pub async fn list_employees(&self, position_id: &str) -> Result<Vec<Employee>> {
        sqlx::query_as!(
            Employee,
            r#"
            SELECT id, name, position_id, position_name, department_name,
                   company_name, model_config, specialty, description,
                   supervisor_id, is_active, authority_override, created_at
            FROM employees
            WHERE position_id = ? AND is_active = 1
            "#,
            position_id
        ).fetch_all(&self.db).await.map_err(|e| e.into())
    }
    
    /// Update employee's model configuration.
    pub async fn update_model(&self, employee_id: &str, model: &str) -> Result<()> {
        sqlx::query!(
            r#"UPDATE employees SET model_config = ? WHERE id = ?"#,
            model, employee_id
        ).execute(&self.db).await?;
        
        if let Some(mut employee) = self.employees.get_mut(employee_id) {
            employee.model_config = model.to_string();
        }
        
        Ok(())
    }
}