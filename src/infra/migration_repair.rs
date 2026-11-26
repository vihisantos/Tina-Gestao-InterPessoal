//! Reparo auditável de checksums de migrations após mudança de quebra de linha.
//!
//! O SQLx valida o SHA-384 do SQL exato de cada migration já aplicada. Em
//! Windows, um arquivo versionado pode ser checkoutado com LF e depois com
//! CRLF (ou vice-versa) por causa de `core.autocrlf`. O conteúdo SQL é o
//! mesmo, mas o checksum muda e a inicialização falha com
//! `migration N was previously applied but has been modified`.
//!
//! Este módulo **nunca** reescreve SQL, não reaplica migrations e não apaga
//! dados. Ele só atualiza `_sqlx_migrations.checksum` quando o checksum
//! registrado é, de forma criptograficamente verificável, o SHA-384 do mesmo
//! SQL com apenas CRLF ↔ LF trocados. Antes de qualquer escrita, cria e
//! valida um backup completo (incluindo o WAL integrado) via `VACUUM INTO`.
//!
//! O reparo é sempre explícito: `Database::open` nunca repara. Apenas a
//! ferramenta `examples/repair_migration_checksums.rs` (e os testes de
//! regressão) chamam `Database::open_with_checksum_repair` com autorização.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::migrate::{MigrateError, Migration, MigrationType, Migrator};
use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};

use crate::error::{Result, TinaError};

/// Linha do journal: versão, checksum SHA-384 e sucesso da migration.
type AppliedMigration = (i64, Vec<u8>, bool);

/// (versão, novo checksum, checksum antigo em hex, novo checksum em hex).
type RepairPlan = Vec<(i64, Vec<u8>, String, String)>;

/// Resultado de uma reconciliação de checksums.
#[derive(Debug, Default)]
pub struct ChecksumRepairReport {
    /// Versões cujo checksum foi ajustado para o SQL embutido atual.
    pub repaired_versions: Vec<i64>,
    /// Backup verificado criado antes da alteração (se houve alteração).
    pub backup_path: Option<PathBuf>,
}

fn checksum_for_sql(
    version: i64,
    description: Cow<'static, str>,
    migration_type: MigrationType,
    no_tx: bool,
    sql: &str,
) -> Vec<u8> {
    Migration::new(
        version,
        description,
        migration_type,
        Cow::Owned(sql.to_owned()),
        no_tx,
    )
    .checksum
    .into_owned()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

async fn table_counts(conn: &mut SqliteConnection) -> Result<BTreeMap<String, i64>> {
    let names: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )
    .fetch_all(&mut *conn)
    .await?;

    let mut counts = BTreeMap::new();
    for name in names {
        let quoted = name.replace('"', "\"\"");
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM \"{quoted}\""))
            .fetch_one(&mut *conn)
            .await?;
        counts.insert(name, count);
    }
    Ok(counts)
}

async fn journal_fingerprint(conn: &mut SqliteConnection) -> Result<Vec<AppliedMigration>> {
    let rows =
        sqlx::query("SELECT version, checksum, success FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&mut *conn)
            .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            (
                row.get::<i64, _>("version"),
                row.get::<Vec<u8>, _>("checksum"),
                row.get::<bool, _>("success"),
            )
        })
        .collect())
}

/// `PRAGMA data_version` muda quando **outra** conexão faz commit. É usado
/// para detectar escrita concorrente entre o backup e a transação de reparo.
async fn data_version(conn: &mut SqliteConnection) -> Result<i64> {
    Ok(sqlx::query_scalar("PRAGMA data_version")
        .fetch_one(&mut *conn)
        .await?)
}

/// Monta o plano somente quando toda divergência é explicada por LF/CRLF.
///
/// Qualquer outro checksum — inclusive SQL realmente alterado — aborta sem
/// produzir plano algum.
fn plan_repairs(applied: &[AppliedMigration], migrator: &Migrator) -> Result<RepairPlan> {
    if let Some(failed) = applied.iter().find(|(_, _, success)| !*success) {
        return Err(TinaError::MigrationChecksumConflict(format!(
            "migration {} está marcada como falha no journal; recuperação manual necessária",
            failed.0
        )));
    }

    let mut repairs: RepairPlan = Vec::new();
    for (version, stored, _) in applied {
        let Some(embedded) = migrator
            .iter()
            .find(|m| m.version == *version && !m.migration_type.is_down_migration())
        else {
            // Versão a mais no banco do que no binário: o SQLx falha
            // separadamente. Não alteramos nada aqui.
            continue;
        };
        if stored.as_slice() == embedded.checksum.as_ref() {
            continue;
        }

        let lf_sql = embedded.sql.replace("\r\n", "\n");
        let crlf_sql = lf_sql.replace('\n', "\r\n");
        let candidates = [embedded.sql.as_ref(), lf_sql.as_str(), crlf_sql.as_str()];
        let explained = candidates.iter().any(|sql| {
            checksum_for_sql(
                embedded.version,
                embedded.description.clone(),
                embedded.migration_type,
                embedded.no_tx,
                sql,
            ) == *stored
        });

        if !explained {
            return Err(TinaError::MigrationChecksumConflict(format!(
                "migration {version}: checksum registrado {} não corresponde ao SQL embutido {} \
                 nem a qualquer variante LF/CRLF; conteúdo divergente — nenhuma alteração foi feita",
                hex(stored),
                hex(embedded.checksum.as_ref())
            )));
        }

        repairs.push((
            *version,
            embedded.checksum.to_vec(),
            hex(stored),
            hex(embedded.checksum.as_ref()),
        ));
    }
    Ok(repairs)
}

async fn create_verified_backup(conn: &mut SqliteConnection, db_path: &Path) -> Result<PathBuf> {
    let parent = db_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let backup_dir = parent.join("backups");
    std::fs::create_dir_all(&backup_dir)?;

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| TinaError::Other(format!("relógio do sistema inválido: {e}")))?
        .as_nanos();
    let mut backup_path = backup_dir.join(format!("migration-checksum-{nanos}.db"));
    let mut suffix = 1u32;
    while backup_path.exists() {
        backup_path = backup_dir.join(format!("migration-checksum-{nanos}-{suffix}.db"));
        suffix += 1;
    }

    let original_counts = table_counts(&mut *conn).await?;
    let original_journal = journal_fingerprint(&mut *conn).await?;

    // `VACUUM INTO` produz um snapshot consistente, com o WAL já integrado.
    // Não toca no banco original.
    let escaped = backup_path.to_string_lossy().replace('\'', "''");
    sqlx::query(&format!("VACUUM INTO '{escaped}'"))
        .execute(&mut *conn)
        .await?;

    if !backup_path.is_file() {
        return Err(TinaError::Other(format!(
            "backup de segurança não foi criado: {}",
            backup_path.display()
        )));
    }

    let options = SqliteConnectOptions::new()
        .filename(&backup_path)
        .read_only(true);
    let backup_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    let mut backup_conn = backup_pool.acquire().await?;
    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&mut *backup_conn)
        .await?;
    let backup_counts = table_counts(&mut *backup_conn).await?;
    let backup_journal = journal_fingerprint(&mut *backup_conn).await?;
    drop(backup_conn);
    backup_pool.close().await;

    if integrity != "ok" {
        return Err(TinaError::Other(format!(
            "backup criado em {} falhou em PRAGMA integrity_check: {integrity}",
            backup_path.display()
        )));
    }
    if original_counts != backup_counts {
        return Err(TinaError::Other(format!(
            "backup criado em {} não preservou os contadores: {:?} != {:?}",
            backup_path.display(),
            original_counts,
            backup_counts
        )));
    }
    if original_journal != backup_journal {
        return Err(TinaError::Other(format!(
            "backup criado em {} não preservou o journal de migrations",
            backup_path.display()
        )));
    }

    Ok(backup_path)
}

/// Compara os checksums aplicados com os SQLs embutidos e, mediante
/// `allow_repair`, corrige somente diferenças de quebra de linha comprovadas
/// por SHA-384.
///
/// Qualquer outra divergência aborta sem alterar o banco. A atualização dos
/// checksums acontece em uma única transação `BEGIN IMMEDIATE`, depois de
/// confirmado que não houve escrita concorrente desde o backup.
pub async fn reconcile_checksums(
    pool: &SqlitePool,
    migrator: &Migrator,
    db_path: &Path,
    allow_repair: bool,
) -> Result<ChecksumRepairReport> {
    // Uma única conexão é usada para observar o `data_version`, criar o backup
    // e manter a transação de escrita. Isso evita falsos positivos causados
    // pelas próprias conexões do pool.
    let mut conn = pool.acquire().await?;

    let journal_exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
    )
    .fetch_one(&mut *conn)
    .await?;
    if journal_exists == 0 {
        return Ok(ChecksumRepairReport::default());
    }

    let data_version_before = data_version(&mut *conn).await?;
    let counts_before = table_counts(&mut *conn).await?;
    let journal_before = journal_fingerprint(&mut *conn).await?;
    let repairs = plan_repairs(&journal_before, migrator)?;

    if repairs.is_empty() {
        return Ok(ChecksumRepairReport::default());
    }

    if !allow_repair {
        let versions: Vec<String> = repairs.iter().map(|(v, _, _, _)| v.to_string()).collect();
        return Err(TinaError::MigrationChecksumConflict(format!(
            "checksum das migrations {} difere apenas por quebra de linha. \
             A inicialização não repara automaticamente. Execute a ferramenta explícita: \
             cargo run --example repair_migration_checksums -- \"<caminho-do-tina.db>\". \
             Nenhuma alteração foi feita.",
            versions.join(", ")
        )));
    }

    let backup_path = create_verified_backup(&mut *conn, db_path).await?;

    // `BEGIN IMMEDIATE` toma o lock de escrita antes de qualquer UPDATE e
    // bloqueia commits concorrentes até o COMMIT/ROLLBACK.
    sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;

    let outcome: Result<()> = async {
        let data_version_locked = data_version(&mut *conn).await?;
        if data_version_locked != data_version_before {
            return Err(TinaError::MigrationChecksumConflict(format!(
                "banco alterado por outro processo durante o reparo (data_version {} -> {}); \
                 nenhuma alteração foi feita; backup em {}",
                data_version_before,
                data_version_locked,
                backup_path.display()
            )));
        }

        let journal_locked = journal_fingerprint(&mut *conn).await?;
        if journal_locked != journal_before {
            return Err(TinaError::MigrationChecksumConflict(format!(
                "journal de migrations alterado durante o reparo; nenhuma alteração foi feita; \
                 backup em {}",
                backup_path.display()
            )));
        }

        let counts_locked = table_counts(&mut *conn).await?;
        if counts_locked != counts_before {
            return Err(TinaError::MigrationChecksumConflict(format!(
                "contadores de tabelas alterados durante o reparo ({:?} -> {:?}); \
                 nenhuma alteração foi feita; backup em {}",
                counts_before,
                counts_locked,
                backup_path.display()
            )));
        }

        let repairs_locked = plan_repairs(&journal_locked, migrator)?;
        if repairs_locked != repairs {
            return Err(TinaError::MigrationChecksumConflict(format!(
                "plano de reparo mudou durante a validação; nenhuma alteração foi feita; \
                 backup em {}",
                backup_path.display()
            )));
        }

        for (version, checksum, _, _) in &repairs {
            sqlx::query("UPDATE _sqlx_migrations SET checksum=? WHERE version=? AND success=1")
                .bind(checksum)
                .bind(version)
                .execute(&mut *conn)
                .await?;
        }
        Ok(())
    }
    .await;

    match outcome {
        Ok(()) => {
            sqlx::query("COMMIT").execute(&mut *conn).await?;
        }
        Err(err) => {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            return Err(err);
        }
    }

    // Validação final: todos os checksums aplicados e conhecidos devem
    // corresponder ao SQL embutido antes de devolver o controle ao chamador.
    let applied_after = journal_fingerprint(&mut *conn).await?;
    for (version, checksum, success) in applied_after {
        if !success {
            continue;
        }
        if let Some(embedded) = migrator.iter().find(|m| m.version == version) {
            if checksum != embedded.checksum.as_ref() {
                return Err(TinaError::MigrationChecksumConflict(format!(
                    "validação pós-reparo falhou na migration {version}: registrado {}, esperado {}; \
                     backup em {}",
                    hex(&checksum),
                    hex(embedded.checksum.as_ref()),
                    backup_path.display()
                )));
            }
        }
    }

    Ok(ChecksumRepairReport {
        repaired_versions: repairs.into_iter().map(|(v, _, _, _)| v).collect(),
        backup_path: Some(backup_path),
    })
}

/// Alias explícito para o erro do SQLx, mantido para chamadas que queiram
/// distinguir "migration modificada" de outros erros de migração.
pub fn is_version_mismatch(error: &MigrateError) -> bool {
    matches!(error, MigrateError::VersionMismatch(_))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksums_de_lf_e_crlf_sao_diferentes() {
        let migrator = sqlx::migrate!("./migration");
        let m = migrator
            .iter()
            .find(|m| m.version == 2)
            .expect("migration 2 existe");
        let lf = m.sql.replace("\r\n", "\n");
        let crlf = lf.replace('\n', "\r\n");
        let lf_sum = checksum_for_sql(
            m.version,
            m.description.clone(),
            m.migration_type,
            m.no_tx,
            &lf,
        );
        let crlf_sum = checksum_for_sql(
            m.version,
            m.description.clone(),
            m.migration_type,
            m.no_tx,
            &crlf,
        );
        assert_ne!(lf_sum, crlf_sum);
    }

    #[test]
    fn plano_aceita_apenas_variante_lf_crlf() {
        let migrator = sqlx::migrate!("./migration");
        let m = migrator.iter().find(|m| m.version == 2).unwrap();
        let lf = m.sql.replace("\r\n", "\n");
        let crlf = lf.replace('\n', "\r\n");
        let lf_sum = checksum_for_sql(
            m.version,
            m.description.clone(),
            m.migration_type,
            m.no_tx,
            &lf,
        );
        let crlf_sum = checksum_for_sql(
            m.version,
            m.description.clone(),
            m.migration_type,
            m.no_tx,
            &crlf,
        );
        assert_ne!(lf_sum, crlf_sum);
        // The fixture must differ from the embedded SQL on either checkout.
        let alternate = if m.checksum.as_ref() == crlf_sum.as_slice() {
            lf_sum
        } else {
            crlf_sum
        };
        assert_ne!(alternate.as_slice(), m.checksum.as_ref());
        assert!(
            plan_repairs(&[(m.version, m.checksum.to_vec(), true)], &migrator)
                .unwrap()
                .is_empty()
        );

        let plan = plan_repairs(&[(m.version, alternate, true)], &migrator).unwrap();
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].0, m.version);
        assert_eq!(plan[0].1, m.checksum.as_ref());
    }

    #[test]
    fn plano_rejeita_sql_realmente_alterado() {
        let migrator = sqlx::migrate!("./migration");
        let m = migrator.iter().find(|m| m.version == 2).unwrap();
        let altered = format!("{}\n-- alteração real no SQL\n", m.sql);
        let altered_sum = checksum_for_sql(
            m.version,
            m.description.clone(),
            m.migration_type,
            m.no_tx,
            &altered,
        );

        let err = plan_repairs(&[(m.version, altered_sum, true)], &migrator).unwrap_err();
        assert!(matches!(err, TinaError::MigrationChecksumConflict(_)));
    }
}
