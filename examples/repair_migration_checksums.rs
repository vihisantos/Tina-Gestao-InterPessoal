//! Ferramenta explícita de reparo: corrige somente diferenças LF/CRLF nos
//! checksums de migrations já aplicadas, com backup verificado.
//!
//! Uso: cargo run --example repair_migration_checksums -- <caminho-do-tina.db>

use std::env;
use std::path::PathBuf;
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

#[tokio::main]
async fn main() {
    let raw = env::args().nth(1).expect(
        "informe o caminho do banco: cargo run --example repair_migration_checksums -- <tina.db>",
    );
    let path = PathBuf::from(raw);

    let options = SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(false)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5))
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .expect("conectar ao banco");

    let migrator = sqlx::migrate!("./migration");

    match tina::infra::migration_repair::reconcile_checksums(&pool, &migrator, &path, true).await {
        Ok(report) => {
            if report.repaired_versions.is_empty() {
                println!("Nada a reparar: todos os checksums já correspondem ao SQL atual.");
            } else {
                println!("Versões reparadas: {:?}", report.repaired_versions);
                println!(
                    "Backup verificado: {}",
                    report
                        .backup_path
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "<nenhum>".into())
                );
            }
        }
        Err(e) => {
            eprintln!("FALHA NO REPARO: {e}");
            std::process::exit(2);
        }
    }

    match migrator.run(&pool).await {
        Ok(()) => println!("Inicialização validada: SQLx aceitou todas as migrations."),
        Err(e) => {
            eprintln!("FALHA NA VALIDAÇÃO FINAL: {e}");
            std::process::exit(3);
        }
    }
}
