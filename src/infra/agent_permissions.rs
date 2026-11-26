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

/// Ponto de entrada interno para adaptadores de ferramentas.
/// O contexto deve ser obtido pelo backend, nunca dos argumentos do modelo.
/// A operação só é construída após autorização bem-sucedida.
pub(crate) async fn execute_authorized<T, F, Fut>(
    pool: &SqlitePool,
    installation_id: &str,
    employee_id: &str,
    tool: &str,
    resource_scope: &str,
    operation: F,
) -> Result<T, sqlx::Error>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, sqlx::Error>>,
{
    require_tool_permission(pool, installation_id, employee_id, tool, resource_scope).await?;
    operation().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[tokio::test]
    async fn exact_scope_and_installation_required() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query("CREATE TABLE active_agent_permissions (installation_id TEXT, employee_id TEXT, tool TEXT, resource_scope TEXT)")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO active_agent_permissions VALUES ('i1','e1','read','r1')")
            .execute(&pool).await.unwrap();
        assert!(require_tool_permission(&pool, "i1", "e1", "read", "r1").await.is_ok());
        for (i, e, t, r) in [
            ("i2", "e1", "read", "r1"),
            ("i1", "e2", "read", "r1"),
            ("i1", "e1", "write", "r1"),
            ("i1", "e1", "read", "r2"),
            ("i1", "e1", "read", "*"),
        ] {
            assert!(require_tool_permission(&pool, i, e, t, r).await.is_err());
        }
        let value = execute_authorized(&pool, "i1", "e1", "read", "r1", || async {
            Ok(42)
        }).await.unwrap();
        assert_eq!(value, 42);
        sqlx::query("DELETE FROM active_agent_permissions").execute(&pool).await.unwrap();
        let executed = AtomicBool::new(false);
        assert!(execute_authorized(&pool, "i1", "e1", "read", "r1", || async {
            executed.store(true, Ordering::SeqCst);
            Ok(())
        }).await.is_err());
        assert!(!executed.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn database_failure_never_executes_tool() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let executed = AtomicBool::new(false);
        let result = execute_authorized(
            &pool, "installation", "employee", "tool", "resource", || async {
                executed.store(true, Ordering::SeqCst);
                Ok(())
            },
        ).await;
        assert!(result.is_err());
        assert!(!executed.load(Ordering::SeqCst));
    }
}
