//! Banco de dados SQLite local (local-first, sem serviço externo).
//!
//! O arquivo original `src/infra/db.rs` foi preservado e corrigido: ele usava
//! uma URL `sqlite://` inadequada para caminhos absolutos do Windows e um
//! `builder()` inexistente em `SqlitePool`. Agora usamos
//! `SqliteConnectOptions::filename()`, que lida corretamente com
//! `%APPDATA%\tina\tina.db`.

use std::path::Path;
use std::time::Duration;

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
        // `busy_timeout` faz transações concorrentes esperarem o lock de
        // escrita em vez de falharem imediatamente. A hierarquia usa
        // `BEGIN IMMEDIATE`, então esse é o tempo máximo de espera por uma
        // alteração concorrente antes de o SQLite retornar `SQLITE_BUSY`.
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5))
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
