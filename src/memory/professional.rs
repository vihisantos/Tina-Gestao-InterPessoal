//! Professional memory management

use super::{MemoryEntry, MemoryScope};

/// Professional memory category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum ProfessionalMemoryCategory {
    /// Work preferences and conventions
    WorkPreferences,
    /// Project knowledge
    Projects,
    /// Meeting notes
    Meetings,
    /// Client information
    Clients,
    /// Industry knowledge
    Industry,
    /// Skills and competencies
    Skills,
}

impl ProfessionalMemoryCategory {
    /// Get the category name as string.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProfessionalMemoryCategory::WorkPreferences => "work_preferences",
            ProfessionalMemoryCategory::Projects => "projects",
            ProfessionalMemoryCategory::Meetings => "meetings",
            ProfessionalMemoryCategory::Clients => "clients",
            ProfessionalMemoryCategory::Industry => "industry",
            ProfessionalMemoryCategory::Skills => "skills",
        }
    }
}