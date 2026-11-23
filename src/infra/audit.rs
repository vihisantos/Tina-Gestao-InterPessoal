//! Consulta local e somente leitura da trilha de auditoria organizacional.
//! A criação de eventos é responsabilidade dos triggers SQLite na mesma transação.
use sqlx::SqlitePool;

use crate::error::Result;

/// Evento persistido na trilha organizacional.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OrganizationalAuditEvent {
    /// Identificador do evento.
    pub id: String,
    /// Instalação de origem.
    pub installation_id: String,
    /// Empresa afetada, quando aplicável.
    pub company_id: Option<String>,
    /// Tipo da entidade afetada.
    pub entity_type: String,
    /// Identificador da entidade afetada.
    pub entity_id: String,
    /// Ação registrada.
    pub action: String,
    /// Tipo do autor, quando autenticado e conhecido.
    pub actor_type: Option<String>,
    /// Identificador do autor, quando autenticado e conhecido.
    pub actor_id: Option<String>,
    /// Resultado da operação.
    pub outcome: String,
    /// Data UTC gerada pelo SQLite.
    pub occurred_at: String,
}

/// Repositório de consultas de auditoria, acessível por escopo autorizado.
pub struct OrganizationalAuditRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> OrganizationalAuditRepository<'a> {
    /// Cria o repositório internamente, sem expor construção via IPC.
    pub(crate) fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Consulta os eventos de uma instalação, com paginação limitada.
    /// O chamador deve validar a autorização antes de obter este repositório.
    pub async fn list_by_installation(
        &self,
        installation_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<OrganizationalAuditEvent>> {
        let limit = limit.clamp(1, 200);
        let offset = offset.max(0);
        let events = sqlx::query_as::<_, OrganizationalAuditEvent>(
            "SELECT id, installation_id, company_id, entity_type, entity_id,
                    action, actor_type, actor_id, outcome, occurred_at
             FROM organizational_audit_events
             WHERE installation_id = ?
             ORDER BY occurred_at DESC, rowid DESC LIMIT ? OFFSET ?",
        )
        .bind(installation_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await?;
        Ok(events)
    }
}
