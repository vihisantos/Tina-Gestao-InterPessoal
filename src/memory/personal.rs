//! Personal memory management

use super::{MemoryEntry, MemoryScope};

/// Personal memory category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum PersonalMemoryCategory {
    /// User preferences
    Preferences,
    /// Contact information
    Contacts,
    /// Daily schedules
    Schedule,
    /// Hobbies and interests
    Hobbies,
    /// Reminders and notes
    Notes,
    /// Documents
    Documents,
}

impl PersonalMemoryCategory {
    /// Get the category name as string.
    pub fn as_str(&self) -> &'static str {
        match self {
            PersonalMemoryCategory::Preferences => "preferences",
            PersonalMemoryCategory::Contacts => "contacts",
            PersonalMemoryCategory::Schedule => "schedule",
            PersonalMemoryCategory::Hobbies => "hobbies",
            PersonalMemoryCategory::Notes => "notes",
            PersonalMemoryCategory::Documents => "documents",
        }
    }
}