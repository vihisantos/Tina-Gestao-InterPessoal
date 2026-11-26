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
    ///
    /// A inicialização normal é rigorosa: qualquer divergência de checksum
    /// aborta o startups e aponta a ferramenta explícita de recuperação.
    /// Nunca há reparo automático durante a abertura do aplicativo.
    pub async fn open(path: &Path) -> Result<Self> {
        Self::open_inner(path, false).await
    }

    /// Variante usada apenas pela ferramenta explícita de recuperação e pelos
    /// testes de regressão. Não é chamada pelo caminho normal de startup.
    pub async fn open_with_checksum_repair(path: &Path, allow_repair: bool) -> Result<Self> {
        Self::open_inner(path, allow_repair).await
    }

    async fn open_inner(path: &Path, allow_repair: bool) -> Result<Self> {
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

        let migrator = sqlx::migrate!("./migration");
        let report = crate::infra::migration_repair::reconcile_checksums(
            &pool,
            &migrator,
            path,
            allow_repair,
        )
        .await?;
        if !report.repaired_versions.is_empty() {
            eprintln!(
                "Tina: checksum das migrations {:?} reconciliado (apenas LF/CRLF). Backup verificado em {}",
                report.repaired_versions,
                report
                    .backup_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "<desconhecido>".to_string())
            );
        }

        migrator.run(&pool).await?;

        Ok(Self { pool })
    }

    /// Pool de conexões SQLite.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
