//! Memory service - main memory management

use crate::error::Result;
use sqlx::SqlitePool;
use std::collections::HashMap;

/// Memory scope levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum MemoryScope {
    /// Personal memories
    Personal,
    /// Professional memories
    Professional,
    /// Organizational memories
    Organizational,
}

impl MemoryScope {
    /// Get the database filter for this scope.
    pub fn filter_clause(&self) -> &'static str {
        match self {
            MemoryScope::Personal => "scope = 'personal'",
            MemoryScope::Professional => "scope = 'professional'",
            MemoryScope::Organizational => "scope = 'organizational'",
        }
    }
}

/// Memory entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryEntry {
    /// Memory ID
    pub id: String,
    /// Key for retrieval
    pub key: String,
    /// Content
    pub content: String,
    /// Embedding (optional)
    pub embedding: Option<Vec<f32>>,
    /// Scope
    pub scope: MemoryScope,
    /// Category
    pub category: String,
    /// Created at
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Expires at (optional)
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Is encrypted
    pub is_encrypted: bool,
}

/// Memory service for storing and retrieving memories.
pub struct MemoryService {
    /// Database pool
    db: SqlitePool,
    /// Scope cache
    scope_cache: HashMap<String, MemoryEntry>,
}

impl MemoryService {
    /// Create a new memory service.
    pub fn new(db: SqlitePool) -> Self {
        Self {
            db,
            scope_cache: HashMap::new(),
        }
    }
    
    /// Store a memory entry.
    pub async fn store(&self, entry: &MemoryEntry) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO memories (id, employee_id, company_id, department_id, scope, key, content, embedding, created_at, expires_at, is_encrypted)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), ?, ?)
            "#,
            entry.id,
            "" as &str, // employee_id
            "" as &str, // company_id
            "" as &str, // department_id
            format!("{:?}", entry.scope),
            entry.key,
            entry.content,
            serde_json::to_string(&entry.embedding).unwrap_or_default(),
            entry.expires_at.map(|t| t.to_rfc3339()),
            if entry.is_encrypted { 1 } else { 0 }
        ).execute(&self.db).await?;
        
        self.scope_cache.insert(entry.id.clone(), entry.clone());
        
        Ok(())
    }
    
    /// Retrieve a memory by key.
    pub async fn get(&self, key: &str, scope: MemoryScope) -> Result<Option<MemoryEntry>> {
        // Check cache first
        if let Some(entry) = self.scope_cache.get(key) {
            if format!("{:?}", entry.scope) == format!("{:?}", scope) {
                return Ok(Some(entry.clone()));
            }
        }
        
        let row = sqlx::query!(
            r#"
            SELECT id, key, content, scope, category, created_at, expires_at, is_encrypted
            FROM memories
            WHERE key = ? AND scope = ?
            "#,
            key,
            format!("{:?}", scope)
        ).fetch_optional(&self.db).await?;
        
        if let Some(row) = row {
            let entry = MemoryEntry {
                id: row.id,
                key: row.key,
                content: row.content,
                embedding: None,
                scope,
                category: row.category.unwrap_or_default(),
                created_at: chrono::Utc::now(),
                expires_at: row.expires_at.map(|s| chrono::DateTime::parse_from_rfc3339(&s).map(|t| t.with_timezone(&chrono::Utc)).unwrap()),
                is_encrypted: row.is_encrypted != 0,
            };
            
            self.scope_cache.insert(entry.id.clone(), entry.clone());
            Ok(Some(entry))
        } else {
            Ok(None)
        }
    }
    
    /// Search memories by semantic similarity.
    pub async fn search(&self, query: &str, scope: MemoryScope, limit: usize) -> Result<Vec<MemoryEntry>> {
        // TODO: Implement vector search
        // For now, use simple keyword matching
        let rows = sqlx::query!(
            r#"
            SELECT id, key, content, scope, category, created_at, expires_at, is_encrypted
            FROM memories
            WHERE scope = ? AND content LIKE ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
            format!("{:?}", scope),
            format!("%{}%", query),
            limit
        ).fetch_all(&self.db).await?;
        
        Ok(rows.into_iter().map(|row| MemoryEntry {
            id: row.id,
            key: row.key,
            content: row.content,
            embedding: None,
            scope,
            category: row.category.unwrap_or_default(),
            created_at: chrono::Utc::now(),
            expires_at: row.expires_at.map(|s| chrono::DateTime::parse_from_rfc3339(&s).map(|t| t.with_timezone(&chrono::Utc)).unwrap()),
            is_encrypted: row.is_encrypted != 0,
        }).collect())
    }
}