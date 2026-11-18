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
    /// Installation ID (from config)
    installation_id: String,
    /// Owner ID (from config)
    owner_id: String,
}

impl TinaCentral {
    /// Create a new Tina Central instance using configuration values.
    /// Installation ID and owner UUID come from the persisted config.
    pub fn new(config: Arc<AppConfig>) -> Self {
        let installation_id = config.installation_id.clone();
        let owner_id = config.owner_uuid.clone();

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_tina_central_uses_config_installation_id() {
        let config = AppConfig {
            data_dir: std::path::PathBuf::from("/test"),
            database_path: std::path::PathBuf::from("/test/db.sqlite"),
            log_path: std::path::PathBuf::from("/test/logs"),
            installation_id: "test-installation-id-12345".to_string(),
            owner_uuid: "test-owner-uuid-67890".to_string(),
            nvidia: crate::config::NvidiaConfig::default(),
            security: crate::config::SecurityConfig::default(),
            features: crate::config::FeatureFlags::default(),
        };
        let config = Arc::new(config);
        let tina = TinaCentral::new(config.clone());

        assert_eq!(tina.installation_id(), "test-installation-id-12345");
    }

    #[test]
    fn test_tina_central_uses_config_owner_uuid() {
        let config = AppConfig {
            data_dir: std::path::PathBuf::from("/test"),
            database_path: std::path::PathBuf::from("/test/db.sqlite"),
            log_path: std::path::PathBuf::from("/test/logs"),
            installation_id: "test-installation-id-12345".to_string(),
            owner_uuid: "test-owner-uuid-67890".to_string(),
            nvidia: crate::config::NvidiaConfig::default(),
            security: crate::config::SecurityConfig::default(),
            features: crate::config::FeatureFlags::default(),
        };
        let config = Arc::new(config);
        let tina = TinaCentral::new(config.clone());

        assert_eq!(tina.owner_id(), "test-owner-uuid-67890");
    }

    #[test]
    fn test_tina_central_identity_persists() {
        // Simulate what TinaApp::initialize does
        let installation_id = "persistent-installation-id";
        let owner_uuid = "persistent-owner-uuid";

        let config = AppConfig {
            data_dir: std::path::PathBuf::from("/test"),
            database_path: std::path::PathBuf::from("/test/db.sqlite"),
            log_path: std::path::PathBuf::from("/test/logs"),
            installation_id: installation_id.to_string(),
            owner_uuid: owner_uuid.to_string(),
            nvidia: crate::config::NvidiaConfig::default(),
            security: crate::config::SecurityConfig::default(),
            features: crate::config::FeatureFlags::default(),
        };
        let config = Arc::new(config);
        let tina1 = TinaCentral::new(config.clone());

        // Create another instance with the same config
        let tina2 = TinaCentral::new(config.clone());

        // Both should have the same identity
        assert_eq!(tina1.installation_id(), tina2.installation_id());
        assert_eq!(tina1.owner_id(), tina2.owner_id());
    }
}
