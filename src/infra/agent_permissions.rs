//! Autorização interna de ferramentas; identidade deve vir da sessão confiável.
use sqlx::SqlitePool;

pub(crate) async fn require_tool_permission(
    pool: &SqlitePool,
    installation_id: &str,
    employee_id: &str,
    tool: &str,
    resource_scope: &str,
) -> Result<(), sqlx::Error> {
    if [installation_id, employee_id, tool, resource_scope]
        .iter()
        .any(|value| value.trim().is_empty())
    {
        return Err(sqlx::Error::Protocol("invalid permission context".into()));
    }
    let allowed: i64 = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM active_agent_permissions
         WHERE installation_id=? AND employee_id=? AND tool=? AND resource_scope=?)",
    )
    .bind(installation_id)
    .bind(employee_id)
    .bind(tool)
    .bind(resource_scope)
    .fetch_one(pool)
    .await?;
    if allowed != 1 {
        return Err(sqlx::Error::Protocol("agent permission denied".into()));
    }
    Ok(())
}
