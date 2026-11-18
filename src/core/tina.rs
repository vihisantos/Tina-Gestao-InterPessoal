//! Tina Central Coordinator

use crate::config::AppConfig;
use crate::error::Result;
use std::sync::Arc;

/// Tina Central - the executive coordinator.
///
/// Tina Central is the executive layer that coordinates all agents
/// in the installation. It operates BELOW the owner's authority.
pub struct TinaCentral {
    /// Configuration
    config: Arc<AppConfig>,
    /// Installation ID
    installation_id: String,
    /// Owner ID
    owner_id: String,
}

impl TinaCentral {
    /// Create a new Tina Central instance.
    pub fn new(config: Arc<AppConfig>) -> Self {
        let installation_id = uuid::Uuid::new_v4().to_string();
        let owner_id = format!("owner_{}", uuid::Uuid::new_v4());

        Self {
            config,
            installation_id,
            owner_id,
        }
    }

    /// Get the installation ID.
    pub fn installation_id(&self) -> &str {
        &self.installation_id
    }

    /// Get the owner ID.
    pub fn owner_id(&self) -> &str {
        &self.owner_id
    }
}

impl Default for TinaCentral {
    fn default() -> Self {
        use std::sync::Arc;
        Self::new(Arc::new(AppConfig::default()))
    }
}
