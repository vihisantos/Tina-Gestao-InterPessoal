//! Histórico compatível com o protocolo NVIDIA para chamadas de ferramentas.
//!
//! O SQLite continua aceitando apenas `user` e `assistant`. Mensagens de
//! resultado usam `role='assistant'` com `tool_call_id`; `build_nvidia_messages`
//! converte isso para `{role:'tool', tool_call_id, content}`. Pedidos de
//! ferramentas usam `tool_calls_json` e viram `{role:'assistant', content:null,
//! tool_calls:[...]}`. Nenhuma credencial é persistida.

use serde_json::{json, Value};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{Result, TinaError};
use crate::infra::chat::ChatMessage;

use super::calls::ToolCall;

/// Persiste o pedido de ferramentas como mensagem `assistant.tool_calls`.
pub async fn append_tool_call_message(
    pool: &SqlitePool,
    installation: &str,
    session: &str,
    calls: &[ToolCall],
) -> Result<ChatMessage> {
    let payload: Vec<Value> = calls
        .iter()
        .map(|call| {
            json!({
                "id": call.id,
                "type": "function",
                "function": {"name": call.name, "arguments": call.arguments}
            })
        })
        .collect();
    let tool_calls_json = serde_json::to_string(&payload)
        .map_err(|_| TinaError::ToolError("falha ao serializar chamadas".into()))?;
    let summary = format!(
        "Solicitei {} ação(ões) com ferramentas. Aguardando autorização do proprietário.",
        calls.len()
    );
    insert_message(
        pool,
        installation,
        session,
        &summary,
        None,
        Some(&tool_calls_json),
    )
    .await
}

/// Persiste o resultado de uma etapa como mensagem `role: tool` no payload NIM.
pub async fn append_tool_result_message(
    pool: &SqlitePool,
    installation: &str,
    session: &str,
    tool_call_id: &str,
    result: &Value,
) -> Result<ChatMessage> {
    let mut content = result.to_string();
    if content.len() > 100_000 {
        content.truncate(100_000);
    }
    insert_message(
        pool,
        installation,
        session,
        &content,
        Some(tool_call_id),
        None,
    )
    .await
}

async fn insert_message(
    pool: &SqlitePool,
    installation: &str,
    session: &str,
    content: &str,
    tool_call_id: Option<&str>,
    tool_calls_json: Option<&str>,
) -> Result<ChatMessage> {
    if content.is_empty() || content.len() > 100_000 {
        return Err(TinaError::Other("mensagem de ferramenta inválida".into()));
    }
    let id = Uuid::new_v4().to_string();
    let mut tx = pool.begin().await?;
    let inserted = sqlx::query(
        "INSERT INTO chat_messages(id,session_id,installation_id,role,content,tool_call_id,tool_calls_json) \
         SELECT ?, id, installation_id, 'assistant', ?, ?, ? FROM chat_sessions \
         WHERE id=? AND installation_id=? ON CONFLICT(id) DO NOTHING",
    )
    .bind(&id)
    .bind(content)
    .bind(tool_call_id)
    .bind(tool_calls_json)
    .bind(session)
    .bind(installation)
    .execute(&mut *tx)
    .await?;
    if inserted.rows_affected() != 1 {
        return Err(TinaError::Unauthorized(
            "sessão de chat indisponível".into(),
        ));
    }
    sqlx::query(
        "UPDATE chat_sessions SET updated_at=datetime('now') WHERE id=? AND installation_id=?",
    )
    .bind(session)
    .bind(installation)
    .execute(&mut *tx)
    .await?;
    let message = sqlx::query_as::<_, ChatMessage>(
        "SELECT id,role,content,created_at,tool_call_id,tool_calls_json FROM chat_messages WHERE id=? AND installation_id=? AND session_id=?",
    )
    .bind(&id)
    .bind(installation)
    .bind(session)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(message)
}
