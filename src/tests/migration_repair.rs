//! Regressão: divergência de checksum causada apenas por LF/CRLF.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use sqlx::migrate::{Migration, MigrationType};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use crate::error::TinaError;
use crate::infra::{chat, InstallationRepository};
use crate::Database;

fn checksum_of(version: i64, description: Cow<'static, str>, sql: String) -> Vec<u8> {
    Migration::new(
        version,
        description,
        MigrationType::Simple,
        Cow::Owned(sql),
        false,
    )
    .checksum
    .into_owned()
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tina-{}-{}", name, uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

async fn read_checksum(path: &Path, version: i64) -> Vec<u8> {
    let options = SqliteConnectOptions::new().filename(path).read_only(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    let value: Vec<u8> =
        sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version=?")
            .bind(version)
            .fetch_one(&pool)
            .await
            .unwrap();
    pool.close().await;
    value
}

async fn tamper_checksum(path: &Path, version: i64, checksum: &[u8]) {
    let options = SqliteConnectOptions::new().filename(path);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    sqlx::query("UPDATE _sqlx_migrations SET checksum=? WHERE version=?")
        .bind(checksum)
        .bind(version)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

#[tokio::test]
async fn startup_nunca_repara_e_ferramenta_explicita_preserva_dados_e_backup() {
    let dir = temp_dir("migration-repair");
    let path = dir.join("tina.db");
    let (installation_id, session_id) = {
        let db = Database::open(&path).await.unwrap();
        let installation = InstallationRepository::new(db.pool())
            .create("repair-owner", "owner")
            .await
            .unwrap();
        let session = chat::create_session(db.pool(), &installation.id, "Histórico")
            .await
            .unwrap();
        chat::append_message(db.pool(), &installation.id, &session.id, "user", "Olá Tina")
            .await
            .unwrap();
        (installation.id, session.id)
    };

    let migrator = sqlx::migrate!("./migration");
    let m = migrator
        .iter()
        .find(|m| m.version == 2)
        .expect("migration 2 existe");
    let lf_sql = m.sql.replace("\r\n", "\n");
    let crlf_sql = lf_sql.replace('\n', "\r\n");
    let lf_checksum = checksum_of(m.version, m.description.clone(), lf_sql);
    let crlf_checksum = checksum_of(m.version, m.description.clone(), crlf_sql);
    assert_ne!(lf_checksum, crlf_checksum);
    // Create a real mismatch even when the embedded SQL uses CRLF.
    let alternate_checksum = if m.checksum.as_ref() == crlf_checksum.as_slice() {
        lf_checksum
    } else {
        crlf_checksum
    };
    assert_ne!(alternate_checksum.as_slice(), m.checksum.as_ref());
    tamper_checksum(&path, 2, &alternate_checksum).await;

    let err = Database::open(&path)
        .await
        .expect_err("a inicialização normal deve falhar sem reparar");
    assert!(matches!(err, TinaError::MigrationChecksumConflict(_)));
    assert_eq!(read_checksum(&path, 2).await, alternate_checksum);
    let backup_dir = dir.join("backups");
    assert!(
        !backup_dir.exists(),
        "startup não pode criar backup sem autorização explícita"
    );

    let db = Database::open_with_checksum_repair(&path, true)
        .await
        .expect("a ferramenta explícita deve reparar");
    let messages = chat::list_messages(db.pool(), &installation_id, &session_id)
        .await
        .expect("mensagens preservadas");
    assert!(messages.iter().any(|m| m.content == "Olá Tina"));
    drop(db);

    assert_eq!(read_checksum(&path, 2).await, m.checksum.as_ref());

    let backups: Vec<_> = std::fs::read_dir(&backup_dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("migration-checksum-")
        })
        .collect();
    assert_eq!(backups.len(), 1, "um único backup verificado deve existir");
    assert_eq!(
        read_checksum(&backups[0].path(), 2).await,
        alternate_checksum,
        "o backup deve conter o checksum original, anterior ao reparo"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn divergencia_real_de_sql_nao_e_reparada_sem_backup() {
    let dir = temp_dir("migration-repair-strict");
    let path = dir.join("tina.db");
    let _ = Database::open(&path).await.unwrap();

    let migrator = sqlx::migrate!("./migration");
    let m = migrator.iter().find(|m| m.version == 2).unwrap();
    let altered = format!("{}\n-- alteração real no SQL\n", m.sql);
    let altered_checksum = checksum_of(m.version, m.description.clone(), altered);
    tamper_checksum(&path, 2, &altered_checksum).await;

    let err = Database::open_with_checksum_repair(&path, true)
        .await
        .expect_err("SQL divergente não pode ser reparado");
    assert!(matches!(err, TinaError::MigrationChecksumConflict(_)));

    assert_eq!(read_checksum(&path, 2).await, altered_checksum);
    assert!(
        !dir.join("backups").exists(),
        "divergência real não pode gerar backup de reparo"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
