//! Organizational memory management

use super::{MemoryEntry, MemoryScope};

/// Organizational memory category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum OrganizationalMemoryCategory {
    /// Company policies
    Policies,
    /// Department procedures
    Procedures,
    /// Organizational structure
    Structure,
    /// Shared documents
    SharedDocs,
    /// Knowledge base
    KnowledgeBase,
    /// Training materials
    Training,
}

impl OrganizationalMemoryCategory {
    /// Get the category name as string.
    pub fn as_str(&self) -> &'static str {
        match self {
            OrganizationalMemoryCategory::Policies => "policies",
            OrganizationalMemoryCategory::Procedures => "procedures",
            OrganizationalMemoryCategory::Structure => "structure",
            OrganizationalMemoryCategory::SharedDocs => "shared_docs",
            OrganizationalMemoryCategory::KnowledgeBase => "knowledge_base",
            OrganizationalMemoryCategory::Training => "training",
        }
    }
}