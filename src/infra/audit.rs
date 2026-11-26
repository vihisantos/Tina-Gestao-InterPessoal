//! Consultas à trilha organizacional; acesso público somente via TinaApp autorizada.
use sqlx::SqlitePool;

use crate::error::Result;

/// Evento de auditoria sem conteúdo secreto ou credenciais.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuditEvent {
    /// Identificador imutável.
    pub id: String,
    /// Instalação de origem.
    pub installation_id: String,
    /// Empresa associada, se houver.
    pub company_id: Option<String>,
    /// Tipo da entidade.
    pub entity_type: String,
    /// ID da entidade.
    pub entity_id: String,
    /// Ação persistida.
    pub action: String,
    /// Tipo de autor, se verificado.
    pub actor_type: Option<String>,
    /// ID de autor, se verificado.
    pub actor_id: Option<String>,
    /// Resultado.
    pub outcome: String,
    /// Data da ocorrência.
    pub occurred_at: String,
}

/// Consulta interna com filtros vinculados, nunca SQL concatenado de IPC.
pub(crate) async fn list_events(
    pool: &SqlitePool,
    installation_id: &str,
    company_id: Option<&str>,
    limit: i64,
) -> Result<Vec<AuditEvent>> {
    let rows = sqlx::query_as::<_, AuditEvent>(
        "SELECT id, installation_id, company_id, entity_type, entity_id,
                action, actor_type, actor_id, outcome, occurred_at
         FROM organizational_audit_events
         WHERE installation_id = ?
           AND (? IS NULL OR company_id = ?)
         ORDER BY rowid DESC
         LIMIT ?",
    )
    .bind(installation_id)
    .bind(company_id)
    .bind(company_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Paginação estável de eventos por ordem de inserção, usando cursor opaco.
pub(crate) async fn list_events_before(
    pool: &SqlitePool,
    installation_id: &str,
    company_id: Option<&str>,
    before_id: &str,
    limit: i64,
) -> Result<Vec<AuditEvent>> {
    let cursor: Option<i64> = sqlx::query_scalar(
        "SELECT rowid FROM organizational_audit_events
         WHERE id = ? AND installation_id = ?
           AND (? IS NULL OR company_id = ?)",
    )
    .bind(before_id)
    .bind(installation_id)
    .bind(company_id)
    .bind(company_id)
    .fetch_optional(pool)
    .await?;
    let cursor = cursor.ok_or_else(|| {
        crate::error::TinaError::Validation("cursor de auditoria inválido".into())
    })?;
    let rows = sqlx::query_as::<_, AuditEvent>(
        "SELECT id, installation_id, company_id, entity_type, entity_id,
                action, actor_type, actor_id, outcome, occurred_at
         FROM organizational_audit_events
         WHERE installation_id = ?
           AND (? IS NULL OR company_id = ?)
           AND rowid < ?
         ORDER BY rowid DESC LIMIT ?",
    )
    .bind(installation_id)
    .bind(company_id)
    .bind(company_id)
    .bind(cursor)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Registra uma tentativa negada separadamente da transação de negócio.
pub(crate) async fn record_denied_owner_audit_read(
    pool: &SqlitePool,
    installation_id: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO organizational_audit_events
         (id, installation_id, entity_type, entity_id, action, outcome)
         VALUES (?, ?, 'audit', ?, 'read', 'denied')",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(installation_id)
    .bind(installation_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Registra uma tentativa de escrita negada separadamente da transação de negócio.
/// Usado quando `record_denied_owner_audit_read` não é adequado porque a operação é de escrita.
pub(crate) async fn record_denied_owner_audit_write(
    pool: &SqlitePool,
    installation_id: &str,
    entity_type: &str,
    entity_id: &str,
    action: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO organizational_audit_events
         (id, installation_id, entity_type, entity_id, action, outcome)
         VALUES (?, ?, ?, ?, ?, 'denied')",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(installation_id)
    .bind(entity_type)
    .bind(entity_id)
    .bind(action)
    .execute(pool)
    .await?;
    Ok(())
}

/// Registra tentativa de escrita negada para o owner_change_employee_position.
/// Usado quando a sessão está bloqueada.
pub(crate) async fn record_denied_position_change(
    pool: &SqlitePool,
    installation_id: &str,
    employee_id: &str,
) -> Result<()> {
    record_denied_owner_audit_write(pool, installation_id, "employee", employee_id, "position_change").await
}

/// Inicia transação de escrita com autoria obtida do contexto autenticado.
pub(crate) async fn begin_owner_write<'a>(
    pool: &'a SqlitePool,
    installation_id: &str,
    owner_id: &str,
) -> Result<sqlx::Transaction<'a, sqlx::Sqlite>> {
    let mut tx = pool.begin().await?;
    let inserted = sqlx::query(
        "INSERT INTO organizational_audit_actor_context (installation_id, actor_id)
         SELECT id, owner_uuid FROM installations WHERE id = ? AND owner_uuid = ?",
    )
    .bind(installation_id)
    .bind(owner_id)
    .execute(&mut *tx)
    .await?;
    if inserted.rows_affected() != 1 {
        return Err(crate::error::TinaError::Unauthorized(
            "identidade de auditoria inválida".into(),
        ));
    }
    Ok(tx)
}

/// Remove o contexto antes de confirmar a mesma transação; rollback em falha.
pub(crate) async fn commit_owner_write(
    mut tx: sqlx::Transaction<'_, sqlx::Sqlite>,
    installation_id: &str,
) -> Result<()> {
    let removed =
        sqlx::query("DELETE FROM organizational_audit_actor_context WHERE installation_id = ?")
            .bind(installation_id)
            .execute(&mut *tx)
            .await?;
    if removed.rows_affected() != 1 {
        return Err(crate::error::TinaError::Other(
            "contexto de auditoria ausente".into(),
        ));
    }
    tx.commit().await?;
    Ok(())
}
