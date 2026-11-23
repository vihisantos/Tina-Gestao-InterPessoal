//! Armazenamento do chat principal: acesso somente por TinaApp autenticada.
use crate::error::{Result, TinaError};
use sqlx::SqlitePool;
use uuid::Uuid;

/// Conversa persistida, pertencente a uma instalação.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChatSession {
    /// Identificador da conversa.
    pub id: String,
    /// Título da conversa.
    pub title: String,
    /// Data de criação.
    pub created_at: String,
}

/// Mensagem persistida, sem tokens de autenticação.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChatMessage {
    /// Identificador da mensagem.
    pub id: String,
    /// Papel (user ou assistant).
    pub role: String,
    /// Conteúdo textual.
    pub content: String,
    /// Data de criação.
    pub created_at: String,
}

pub(crate) async fn create_session(
    pool: &SqlitePool,
    installation: &str,
    title: &str,
) -> Result<ChatSession> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 120 {
        return Err(TinaError::Other("título inválido".into()));
    }
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO chat_sessions(id, installation_id, title) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(installation)
        .bind(title)
        .execute(pool)
        .await?;
    Ok(sqlx::query_as(
        "SELECT id, title, created_at FROM chat_sessions WHERE id = ? AND installation_id = ?",
    )
    .bind(&id)
    .bind(installation)
    .fetch_one(pool)
    .await?)
}

/// Renomeia uma conversa somente dentro da instalação autenticada.
pub(crate) async fn rename_session(
    pool: &SqlitePool,
    installation: &str,
    session: &str,
    title: &str,
) -> Result<ChatSession> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 120 {
        return Err(TinaError::Other("título inválido".into()));
    }
    let result = sqlx::query(
        "UPDATE chat_sessions SET title = ?, updated_at = datetime('now') WHERE id = ? AND installation_id = ?",
    )
    .bind(title)
    .bind(session)
    .bind(installation)
    .execute(pool)
    .await?;
    if result.rows_affected() != 1 {
        return Err(TinaError::Unauthorized("conversa fora da instalação ou inexistente".into()));
    }
    Ok(sqlx::query_as(
        "SELECT id, title, created_at FROM chat_sessions WHERE id = ? AND installation_id = ?",
    )
    .bind(session)
    .bind(installation)
    .fetch_one(pool)
    .await?)
}

pub(crate) async fn list_sessions(
    pool: &SqlitePool,
    installation: &str,
) -> Result<Vec<ChatSession>> {
    Ok(sqlx::query_as("SELECT id, title, created_at FROM chat_sessions WHERE installation_id = ? ORDER BY updated_at DESC, id DESC LIMIT 100")
        .bind(installation).fetch_all(pool).await?)
}

pub(crate) async fn append_message(
    pool: &SqlitePool,
    installation: &str,
    session: &str,
    role: &str,
    content: &str,
) -> Result<ChatMessage> {
    if !matches!(role, "user" | "assistant") || content.trim().is_empty() || content.len() > 100_000
    {
        return Err(TinaError::Other("mensagem inválida".into()));
    }
    let id = Uuid::new_v4().to_string();
    let mut tx = pool.begin().await?;
    let result = sqlx::query(
        "INSERT INTO chat_messages(id, session_id, installation_id, role, content)
         SELECT ?, id, installation_id, ?, ? FROM chat_sessions
         WHERE id = ? AND installation_id = ?",
    )
    .bind(&id)
    .bind(role)
    .bind(content)
    .bind(session)
    .bind(installation)
    .execute(&mut *tx)
    .await?;
    if result.rows_affected() != 1 {
        return Err(TinaError::Unauthorized(
            "conversa fora da instalação ou inexistente".into(),
        ));
    }
    sqlx::query("UPDATE chat_sessions SET updated_at = datetime('now') WHERE id = ? AND installation_id = ?")
        .bind(session).bind(installation).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(sqlx::query_as("SELECT id, role, content, created_at FROM chat_messages WHERE id = ? AND installation_id = ?")
        .bind(&id).bind(installation).fetch_one(pool).await?)
}

pub(crate) async fn list_messages(
    pool: &SqlitePool,
    installation: &str,
    session: &str,
) -> Result<Vec<ChatMessage>> {
    Ok(sqlx::query_as(
        "SELECT id, role, content, created_at FROM (
             SELECT m.id, m.role, m.content, m.created_at, m.rowid AS message_order
             FROM chat_messages m
             JOIN chat_sessions s ON s.id = m.session_id AND s.installation_id = m.installation_id
             WHERE s.id = ? AND s.installation_id = ?
             ORDER BY m.created_at DESC, m.rowid DESC LIMIT 500
         ) ORDER BY created_at ASC, message_order ASC",
    )
    .bind(session)
    .bind(installation)
    .fetch_all(pool)
    .await?)
}
