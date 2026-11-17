//! Banco de dados SQLite local (local-first, sem serviço externo).
//!
//! O arquivo original `src/infra/db.rs` foi preservado e corrigido: ele usava
//! uma URL `sqlite://` inadequada para caminhos absolutos do Windows e um
//! `builder()` inexistente em `SqlitePool`. Agora usamos
//! `SqliteConnectOptions::filename()`, que lida corretamente com
//! `%APPDATA%\tina\tina.db`.

use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::error::Result;

/// Conexão com o banco local já com as migrations aplicadas.
#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Abre (ou cria) o banco em `path` e aplica as migrations versionadas.
    pub async fn open(path: &Path) -> Result<Self> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        sqlx::migrate!("./migration").run(&pool).await?;

        Ok(Self { pool })
    }

    /// Pool de conexões SQLite.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
