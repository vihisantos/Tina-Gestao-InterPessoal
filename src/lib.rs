//! Tina Platform — inteligência pessoal e operacional.
//!
//! Este crate é o **núcleo** da aplicação desktop. Ele não contém entry point:
//! o executável é `src-tauri/src/main.rs`, que o utiliza como biblioteca.
//!
//! Nesta fase do projeto os módulos necessários ao bootstrap desktop
//! são declarados: configuração, erros, banco SQLite e instalação.

#![deny(missing_docs)]

use std::sync::Arc;

use crate::config::AppConfig;
use crate::error::Result;

pub mod config;
pub mod core;
pub mod error;

pub mod infra;

#[cfg(test)]
mod tests;

pub use error::{Result as TinaResult, TinaError};
pub use infra::Database;

/// Versão da plataforma, obtida do manifesto Cargo.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Estado inicial da Tina, compartilhado com os comandos Tauri via `State`.
#[derive(Debug, Clone)]
pub struct TinaApp {
    config: Arc<AppConfig>,
    database: Database,
}

impl TinaApp {
    /// Carrega configuração, garante os diretórios locais, abre/cria o SQLite
    /// aplica as migrations e garante que a instalação exista no banco.
    pub async fn initialize() -> Result<Self> {
        let config = Arc::new(config::load_config()?);
        config.ensure_directories()?;

        let database = Database::open(&config.database_path).await?;

        // Garantir que a instalação exista no banco (inicialização idempotente)
        let repo = infra::InstallationRepository::new(database.pool());

        // Verificar se a instalação do config existe no banco
        // O ID da instalação é o principal-chave (identificador da instalação como um todo)
        // O owner_uuid é o ID do proprietário que criou a instalação
        if let Some(_existing_installation) = repo.find_by_id(&config.installation_id).await? {
            // Instalação já existe - tudo certo
        } else {
            // Instalação não existe - criar com o ID da configuração
            let _ = repo
                .create_with_id("Tina", &config.installation_id, &config.owner_uuid)
                .await?;
        }

        Ok(Self { config, database })
    }

    /// Initialize TinaApp at a specific path (for testing).
    /// This allows isolated testing with temporary databases.
    pub async fn initialize_at(database_path: &std::path::Path, installation_id: &str, owner_uuid: &str) -> Result<Self> {
        let data_dir = database_path
            .parent()
            .expect("Database path must have parent directory")
            .to_path_buf();

        let config = Arc::new(AppConfig {
            data_dir: data_dir.clone(),
            database_path: database_path.to_path_buf(),
            log_path: data_dir.join("logs"),
            installation_id: installation_id.to_string(),
            owner_uuid: owner_uuid.to_string(),
            nvidia: config::NvidiaConfig::default(),
            security: config::SecurityConfig::default(),
            features: config::FeatureFlags::default(),
        });

        config.ensure_directories()?;

        let database = Database::open(&config.database_path).await?;

        // Garantir que a instalação exista no banco (inicialização idempotente)
        let repo = infra::InstallationRepository::new(database.pool());

        // Verificar se a instalação do config existe no banco
        if let Some(_existing_installation) = repo.find_by_id(&config.installation_id).await? {
            // Instalação já existe - tudo certo
        } else {
            // Instalação não existe - criar com o ID da configuração
            let _ = repo
                .create_with_id("Tina", &config.installation_id, &config.owner_uuid)
                .await?;
        }

        Ok(Self { config, database })
    }

    /// Configuração carregada da instalação.
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    /// Banco de dados local já inicializado.
    pub fn database(&self) -> &Database {
        &self.database
    }
}
