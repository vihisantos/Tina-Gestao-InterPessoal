//! Upgrades de bancos legados e política de checksum das migrations publicadas.
//!
//! Os bancos de teste são criados com as próprias migrations versionadas do
//! repositório, filtradas até uma versão anterior. Nenhum SQL externo é usado.

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sqlx::migrate::{MigrateError, Migration, Migrator};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::error::TinaError;
use crate::Database;

fn embedded() -> Migrator {
    sqlx::migrate!("./migration")
}

/// Migrator com as mesmas versões e checksums do binário, limitado a `through`.
fn migrator_through(through: i64) -> Migrator {
    let migrations: Vec<Migration> = embedded()
        .iter()
        .filter(|m| m.version <= through && !m.migration_type.is_down_migration())
        .cloned()
        .collect();
    Migrator {
        migrations: Cow::Owned(migrations),
        ..Migrator::DEFAULT
    }
}

fn expected_versions() -> Vec<i64> {
    let mut versions: Vec<i64> = embedded().iter().map(|m| m.version).collect();
    versions.sort_unstable();
    versions
}

fn tmp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tina-upgrade-{}-{}", name, uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

async fn raw_pool(path: &Path) -> SqlitePool {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal)
                .busy_timeout(Duration::from_secs(5))
                .foreign_keys(true),
        )
        .await
        .unwrap()
}

async fn seed_legacy_db(path: &Path, through: i64) {
    let pool = raw_pool(path).await;
    migrator_through(through).run(&pool).await.unwrap();
    pool.close().await;
}

async fn journal(path: &Path) -> Vec<(i64, Vec<u8>, bool)> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(SqliteConnectOptions::new().filename(path).read_only(true))
        .await
        .unwrap();
    let rows =
        sqlx::query_as("SELECT version,checksum,success FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&pool)
            .await
            .unwrap();
    pool.close().await;
    rows
}

async fn versions(path: &Path) -> Vec<i64> {
    journal(path)
        .await
        .into_iter()
        .map(|(version, _, _)| version)
        .collect()
}

async fn integrity_ok(path: &Path) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(SqliteConnectOptions::new().filename(path).read_only(true))
        .await
        .unwrap();
    let result: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(result, "ok");
    pool.close().await;
}

fn checksum_for(m: &Migration, sql: &str) -> Vec<u8> {
    Migration::new(
        m.version,
        m.description.clone(),
        m.migration_type,
        Cow::Owned(sql.to_owned()),
        m.no_tx,
    )
    .checksum
    .into_owned()
}

/// Checksum da mesma SQL com a quebra de linha oposta à incorporada.
fn alternate_eol_checksum(m: &Migration) -> Vec<u8> {
    let lf = m.sql.replace("\r\n", "\n");
    let crlf = lf.replace('\n', "\r\n");
    let lf_sum = checksum_for(m, &lf);
    let crlf_sum = checksum_for(m, &crlf);
    let chosen = if lf_sum.as_slice() == m.checksum.as_ref() {
        crlf_sum
    } else {
        lf_sum
    };
    assert_ne!(chosen.as_slice(), m.checksum.as_ref());
    chosen
}

/// Checksum de uma SQL realmente diferente — modelo da dívida histórica do 001.
fn real_divergence_checksum(m: &Migration) -> Vec<u8> {
    checksum_for(m, &format!("{}\n-- conteúdo publicado divergente\n", m.sql))
}

async fn set_checksum(path: &Path, version: i64, checksum: &[u8]) {
    let pool = raw_pool(path).await;
    sqlx::query("UPDATE _sqlx_migrations SET checksum=? WHERE version=?")
        .bind(checksum)
        .bind(version)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

fn migration(version: i64) -> Migration {
    embedded()
        .iter()
        .find(|m| m.version == version)
        .unwrap()
        .clone()
}

#[tokio::test]
async fn upgrade_from_v1_preserves_employee_status_and_history_tables() {
    let dir = tmp("v1");
    let path = dir.join("tina.db");
    seed_legacy_db(&path, 1).await;
    assert_eq!(versions(&path).await, vec![1]);

    {
        let pool = raw_pool(&path).await;
        sqlx::query("INSERT INTO installations(id,name,root_path,version,owner_uuid) VALUES ('inst-legacy','Legacy','/tmp/legacy','0.1.0','owner-legacy')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO companies(id,installation_id,name) VALUES ('comp-legacy','inst-legacy','Empresa Legacy')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO departments(id,company_id,name,code) VALUES ('dept-legacy','comp-legacy','Dept Legacy','DEPT')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO positions(id,department_id,name) VALUES ('pos-legacy','dept-legacy','Cargo Legacy')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO employees(id,position_id,name,model_config,is_active) VALUES ('legacy-inactive','pos-legacy','Inativo','{}',0)")
            .execute(&pool).await.unwrap();
        pool.close().await;
    }

    let db = Database::open(&path).await.unwrap();
    assert_eq!(versions(&path).await, expected_versions());
    let (status, active): (String, i64) =
        sqlx::query_as("SELECT status,is_active FROM employees WHERE id='legacy-inactive'")
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(status, "inactive");
    assert_eq!(active, 0);
    let in_view: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM active_employees WHERE id='legacy-inactive'")
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(in_view, 0);
    for table in [
        "employee_supervisor_history",
        "employee_assignment_history",
        "organizational_audit_events",
    ] {
        let sql = format!("SELECT COUNT(*) FROM {}", table);
        let count: i64 = sqlx::query_scalar(&sql).fetch_one(db.pool()).await.unwrap();
        assert_eq!(count, 0, "tabela {} deveria começar vazia", table);
    }
    let triggers: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='trigger' AND name LIKE '%reject_%' ORDER BY name",
    )
    .fetch_all(db.pool())
    .await
    .unwrap();
    for expected in [
        "employee_assignment_history_reject_delete",
        "employee_assignment_history_reject_update",
        "employee_supervisor_history_reject_delete",
        "employee_supervisor_history_reject_update",
    ] {
        assert!(
            triggers.iter().any(|name| name == expected),
            "falta {}",
            expected
        );
    }
    drop(db);
    integrity_ok(&path).await;
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn upgrade_from_v12_backfills_company_kind_and_enforces_one_personal() {
    let dir = tmp("v12");
    let path = dir.join("tina.db");
    seed_legacy_db(&path, 12).await;
    {
        let pool = raw_pool(&path).await;
        sqlx::query("INSERT INTO installations(id,name,root_path,version,owner_uuid) VALUES ('inst-legacy','Legacy','/tmp/legacy','0.1.0','owner-legacy')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO companies(id,installation_id,name) VALUES ('comp-legacy','inst-legacy','Empresa Legacy')")
            .execute(&pool).await.unwrap();
        pool.close().await;
    }

    let db = Database::open(&path).await.unwrap();
    let kind: String = sqlx::query_scalar("SELECT kind FROM companies WHERE id='comp-legacy'")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(kind, "business");
    let index: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_companies_one_personal'",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(index, 1);
    sqlx::query("INSERT INTO companies(id,installation_id,name,kind) VALUES ('personal-legacy','inst-legacy','Pessoal','personal')")
        .execute(db.pool()).await.unwrap();
    assert!(sqlx::query("INSERT INTO companies(id,installation_id,name,kind) VALUES ('personal-extra','inst-legacy','Outra Pessoal','personal')")
        .execute(db.pool()).await.is_err());
    drop(db);
    integrity_ok(&path).await;
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn upgrade_from_v15_keeps_only_the_latest_pending_proposal_active() {
    let dir = tmp("v15-proposals");
    let path = dir.join("tina.db");
    seed_legacy_db(&path, 15).await;
    {
        let pool = raw_pool(&path).await;
        sqlx::query("INSERT INTO installations(id,name,root_path,version,owner_uuid) VALUES ('inst-legacy','Legacy','/tmp/legacy','0.1.0','owner-legacy')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO companies(id,installation_id,name,kind) VALUES ('comp-legacy','inst-legacy','Empresa Legacy','business')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO chat_sessions(id,installation_id,title,created_at,updated_at) VALUES ('session-legacy','inst-legacy','Legado','2026-01-01 00:00:00','2026-01-01 00:00:00')")
            .execute(&pool).await.unwrap();
        for (id, role, content) in [
            ("msg-user-1", "user", "Crie Finanças"),
            ("msg-prop-1", "assistant", "Proposta de departamento"),
            ("msg-user-2", "user", "Crie uma organização pessoal"),
            ("msg-prop-2", "assistant", "Proposta de organização"),
            ("msg-user-3", "user", "Crie Operações"),
            ("msg-prop-3", "assistant", "Proposta aprovada"),
            ("msg-result-3", "assistant", "Departamento criado"),
            ("msg-user-4", "user", "Mensagem posterior à proposta"),
            ("msg-after-4", "assistant", "Resposta posterior"),
        ] {
            sqlx::query("INSERT INTO chat_messages(id,session_id,installation_id,role,content,created_at) VALUES (?, 'session-legacy', 'inst-legacy', ?, ?, '2026-01-01 00:00:00')")
                .bind(id).bind(role).bind(content).execute(&pool).await.unwrap();
        }
        sqlx::query("INSERT INTO chat_department_proposals(id,installation_id,session_id,user_message_id,proposal_message_id,company_id,company_name,organization_kind,name,code,status) VALUES ('dept-old','inst-legacy','session-legacy','msg-user-1','msg-prop-1','comp-legacy','Empresa Legacy','business','Finanças','FIN','pending')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO chat_organization_proposals(id,installation_id,session_id,user_message_id,proposal_message_id,name,organization_kind,reason,status) VALUES ('org-new','inst-legacy','session-legacy','msg-user-2','msg-prop-2','Pessoal','personal','Separar rotina','pending')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO chat_department_proposals(id,installation_id,session_id,user_message_id,proposal_message_id,company_id,company_name,organization_kind,name,code,status,result_message_id) VALUES ('dept-approved','inst-legacy','session-legacy','msg-user-3','msg-prop-3','comp-legacy','Empresa Legacy','business','Operações','OPS','approved','msg-result-3')")
            .execute(&pool).await.unwrap();
        pool.close().await;
    }

    let db = Database::open(&path).await.unwrap();
    assert_eq!(versions(&path).await, expected_versions());
    let (state, by): (String, Option<String>) = sqlx::query_as(
        "SELECT lifecycle_status,superseded_by FROM chat_department_proposals WHERE id='dept-old'",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(state, "superseded");
    assert_eq!(by.as_deref(), Some("org-new"));
    let legacy_status: String =
        sqlx::query_scalar("SELECT status FROM chat_department_proposals WHERE id='dept-old'")
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(legacy_status, "rejected");
    let approved: String = sqlx::query_scalar(
        "SELECT lifecycle_status FROM chat_department_proposals WHERE id='dept-approved'",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(approved, "approved");

    let active = crate::infra::chat_tools::pending(db.pool(), "inst-legacy", "session-legacy")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(active.id, "org-new");
    assert_eq!(active.tool_name, "create_organization");
    drop(db);
    integrity_ok(&path).await;
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn upgrade_from_v19_unifies_rh_proposal_lifecycle_after_019() {
    let dir = tmp("v19-rh-lifecycle");
    let path = dir.join("tina.db");
    seed_legacy_db(&path, 19).await;
    assert_eq!(versions(&path).await, (1..=19).collect::<Vec<_>>());
    {
        let pool = raw_pool(&path).await;
        sqlx::query("INSERT INTO installations(id,name,root_path,version,owner_uuid) VALUES ('inst-legacy','Legacy','/tmp/legacy','0.1.0','owner-legacy')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO companies(id,installation_id,name,kind,status) VALUES ('comp-legacy','inst-legacy','Empresa Legacy','business','active')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO departments(id,company_id,name,code,status) VALUES ('dept-legacy','comp-legacy','Finanças','FIN','active')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO positions(id,department_id,name,level,authority_level,description,status) VALUES ('pos-legacy','dept-legacy','Analista',1,0,'Acompanhar orçamento','active')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO employees(id,position_id,name,model_config,is_active) VALUES ('emp-legacy','pos-legacy','Ana','{}',1)")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO chat_sessions(id,installation_id,title,created_at,updated_at) VALUES ('session-legacy','inst-legacy','Legado','2026-01-01 00:00:00','2026-01-01 00:00:00')")
            .execute(&pool).await.unwrap();
        for (id, role, content) in [
            ("msg-user-1", "user", "Alterar ferramentas"),
            ("msg-prop-1", "assistant", "Proposta de acesso"),
            ("msg-user-2", "user", "Criar Analista II"),
            ("msg-prop-2", "assistant", "Proposta de cargo"),
        ] {
            sqlx::query("INSERT INTO chat_messages(id,session_id,installation_id,role,content,created_at) VALUES (?, 'session-legacy', 'inst-legacy', ?, ?, '2026-01-01 00:00:00')")
                .bind(id).bind(role).bind(content).execute(&pool).await.unwrap();
        }
        sqlx::query("INSERT INTO chat_tool_access_proposals(id,installation_id,session_id,user_message_id,proposal_message_id,employee_id,employee_name,company_id,company_name,organization_kind,department_id,department_name,action,tool_keys,reason,status) VALUES ('access-old','inst-legacy','session-legacy','msg-user-1','msg-prop-1','emp-legacy','Ana','comp-legacy','Empresa Legacy','business','dept-legacy','Finanças','assign_tool','[\"create_position\"]','Rotina','pending')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO chat_position_proposals(id,installation_id,session_id,user_message_id,proposal_message_id,company_id,company_name,organization_kind,department_id,department_name,name,level,description,reason,status) VALUES ('position-new','inst-legacy','session-legacy','msg-user-2','msg-prop-2','comp-legacy','Empresa Legacy','business','dept-legacy','Finanças','Analista II',1,'Apoiar rotina','Nova demanda','pending')")
            .execute(&pool).await.unwrap();
        pool.close().await;
    }

    let db = Database::open(&path).await.unwrap();
    assert_eq!(versions(&path).await, expected_versions());
    let (state, by, legacy): (String, Option<String>, String) = sqlx::query_as(
        "SELECT lifecycle_status,superseded_by,status FROM chat_tool_access_proposals WHERE id='access-old'",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(state, "pending");
    assert_eq!(by, None);
    assert_eq!(legacy, "pending");
    let active = crate::infra::chat_tools::pending(db.pool(), "inst-legacy", "session-legacy")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(active.id, "position-new");
    assert_eq!(active.tool_name, "create_position");
    let access_state: String = sqlx::query_scalar(
        "SELECT lifecycle_status FROM chat_tool_access_proposals WHERE id='access-old'",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(access_state, "pending");
    let columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('chat_contract_drafts') WHERE name='lifecycle_status'",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(columns, 1);
    drop(db);
    integrity_ok(&path).await;
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn upgrade_from_v20_creates_tool_orchestration_schema_preserving_chats() {
    let dir = tmp("v20-tool-orchestration");
    let path = dir.join("tina.db");
    seed_legacy_db(&path, 20).await;
    assert_eq!(versions(&path).await, (1..=20).collect::<Vec<_>>());
    {
        let pool = raw_pool(&path).await;
        sqlx::query("INSERT INTO installations(id,name,root_path,version,owner_uuid) VALUES ('inst-legacy','Legacy','/tmp/legacy','0.1.0','owner-legacy')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO chat_sessions(id,installation_id,title,created_at,updated_at) VALUES ('session-legacy','inst-legacy','Legado','2026-01-01 00:00:00','2026-01-01 00:00:00')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO chat_messages(id,session_id,installation_id,role,content,created_at) VALUES ('msg-legacy','session-legacy','inst-legacy','user','Conversa antiga','2026-01-01 00:00:00')")
            .execute(&pool).await.unwrap();
        pool.close().await;
    }

    let db = Database::open(&path).await.unwrap();
    assert_eq!(versions(&path).await, expected_versions());
    let plans: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='chat_tool_plans'",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(plans, 1);
    let message_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('chat_messages') \
         WHERE name IN ('tool_call_id','tool_calls_json')",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(message_columns, 2);
    let content: String =
        sqlx::query_scalar("SELECT content FROM chat_messages WHERE id='msg-legacy'")
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(content, "Conversa antiga");
    drop(db);
    integrity_ok(&path).await;
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn startup_refuses_checksum_drift_and_explicit_repair_creates_backup() {
    let dir = tmp("eol");
    let path = dir.join("tina.db");
    seed_legacy_db(&path, 13).await;
    {
        let pool = raw_pool(&path).await;
        sqlx::query("INSERT INTO installations(id,name,root_path,version,owner_uuid) VALUES ('inst-legacy','Legacy','/tmp/legacy','0.1.0','owner-legacy')")
            .execute(&pool).await.unwrap();
        pool.close().await;
    }
    std::fs::write(dir.join("tina.toml"), "sentinela = true\n").unwrap();

    let m = migration(13);
    set_checksum(&path, 13, &alternate_eol_checksum(&m)).await;
    let before = journal(&path).await;

    let err = Database::open(&path)
        .await
        .expect_err("startup não pode reparar");
    assert!(matches!(err, TinaError::MigrationChecksumConflict(_)));
    assert!(err.to_string().contains("repair_migration_checksums"));
    assert_eq!(journal(&path).await, before);
    assert!(!dir.join("backups").exists());
    assert!(dir.join("tina.toml").exists());

    let db = Database::open_with_checksum_repair(&path, true)
        .await
        .expect("ferramenta explícita deve reparar apenas LF/CRLF");
    assert_eq!(versions(&path).await, expected_versions());
    let after = journal(&path).await;
    assert_eq!(
        after.iter().find(|(v, _, _)| *v == 13).unwrap().1,
        m.checksum.as_ref()
    );
    drop(db);

    let backups: Vec<PathBuf> = std::fs::read_dir(dir.join("backups"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("migration-checksum-"))
        })
        .collect();
    assert_eq!(backups.len(), 1);
    integrity_ok(&backups[0]).await;
    integrity_ok(&path).await;
    assert!(dir.join("tina.toml").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn real_checksum_divergence_is_never_repaired() {
    let dir = tmp("real-divergence");
    let path = dir.join("tina.db");
    let db = Database::open(&path).await.unwrap();
    sqlx::query("INSERT INTO installations(id,name,root_path,version,owner_uuid) VALUES ('inst-legacy','Legacy','/tmp/legacy','0.1.0','owner-legacy')")
        .execute(db.pool()).await.unwrap();
    drop(db);
    std::fs::write(dir.join("tina.toml"), "sentinela = true\n").unwrap();

    let m = migration(1);
    set_checksum(&path, 1, &real_divergence_checksum(&m)).await;
    let before = journal(&path).await;

    for allow in [false, true] {
        let err = Database::open_with_checksum_repair(&path, allow)
            .await
            .expect_err("divergência real de SQL nunca é reparada");
        assert!(matches!(err, TinaError::MigrationChecksumConflict(_)));
        assert_eq!(journal(&path).await, before);
        assert!(!dir.join("backups").exists());
    }
    assert!(dir.join("tina.toml").exists());
    {
        let pool = raw_pool(&path).await;
        let name: String =
            sqlx::query_scalar("SELECT name FROM installations WHERE id='inst-legacy'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(name, "Legacy");
        pool.close().await;
    }
    integrity_ok(&path).await;
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn unknown_and_failed_migration_versions_fail_closed() {
    let dir = tmp("journal");
    let path = dir.join("tina.db");
    let db = Database::open(&path).await.unwrap();
    drop(db);

    {
        let pool = raw_pool(&path).await;
        sqlx::query("INSERT INTO _sqlx_migrations(version,description,success,checksum,execution_time) VALUES (99,'future',1,x'00',0)")
            .execute(&pool).await.unwrap();
        pool.close().await;
    }
    let err = Database::open(&path)
        .await
        .expect_err("versão futura deve falhar");
    assert!(matches!(
        err,
        TinaError::Migration(MigrateError::VersionMissing(99))
    ));
    assert!(!dir.join("backups").exists());

    {
        let pool = raw_pool(&path).await;
        sqlx::query("DELETE FROM _sqlx_migrations WHERE version=99")
            .execute(&pool)
            .await
            .unwrap();
        let latest = *expected_versions().last().unwrap();
        sqlx::query("UPDATE _sqlx_migrations SET success=0 WHERE version=?")
            .bind(latest)
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
    }
    let err = Database::open(&path)
        .await
        .expect_err("migration marcada como falha deve impedir a inicialização");
    assert!(matches!(err, TinaError::MigrationChecksumConflict(_)));
    assert!(!dir.join("backups").exists());
    integrity_ok(&path).await;
    let _ = std::fs::remove_dir_all(&dir);
}
