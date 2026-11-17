//! Tina Platform — inteligência pessoal e operacional.
//!
//! Este crate é o **núcleo** da aplicação desktop. Ele não contém entry point:
//! o executável é `src-tauri/src/main.rs`, que o utiliza como biblioteca.
//!
//! Nesta fase do projeto apenas os módulos necessários ao bootstrap desktop
//! são declarados: configuração, erros e banco SQLite. Os demais módulos de
//! arquitetura (`core`, `planner`, `agent`, `route`, `memory`, `auth`, `eval`,
//! `exec`, `activity`, `adapter`, `mcp`) seguem preservados em `src/` e serão
//! declarados nas fases seguintes, quando suas implementações existirem.

#![deny(missing_docs)]

use std::sync::Arc;

use crate::config::AppConfig;
use crate::error::Result;

pub mod config;
pub mod error;

/// Camada de persistência SQLite (arquivo preservado de `src/infra/db.rs`).
#[path = "infra/db.rs"]
pub mod db;

pub use db::Database;
pub use error::{Result as TinaResult, TinaError};

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
    /// e aplica as migrations. Não faz qualquer chamada de rede.
    pub async fn initialize() -> Result<Self> {
        let config = Arc::new(config::load_config()?);
        config.ensure_directories()?;

        let database = Database::open(&config.database_path).await?;

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
