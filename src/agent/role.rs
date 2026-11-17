//! Role definitions for employees

/// Role types in the Tina hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum RoleType {
    /// Base agent - no supervision
    Agent = 0,
    /// Team leader - can lead a team
    TeamLeader = 1,
    /// Section chief - leads multiple teams
    SectionChief = 2,
    /// Department head - leads a department
    DepartmentHead = 3,
    /// Executive - executive level
    Executive = 4,
    /// Owner - maximum authority (above all)
    Owner = 100,
}

impl RoleType {
    /// Get the authority level for this role.
    pub fn authority_level(&self) -> u32 {
        *self as u32
    }
    
    /// Check if this role can supervise another.
    pub fn can_supervise(&self, other: RoleType) -> bool {
        self.authority_level() > other.authority_level()
    }
    
    /// Parse from string.
    pub fn from_str(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}