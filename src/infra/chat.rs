//! Armazenamento do chat principal: acesso somente por TinaApp autenticada.
use crate::error::{Result, TinaError};
use sqlx::SqlitePool;
use uuid::Uuid;

/// Conversa persistida, pertencente a uma instalação.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct ChatSession {
    /// Identificador da conversa.
    pub id: String,
    /// Título da conversa.
    pub title: String,
    /// Data de criação.
    pub created_at: String,
}

/// Mensagem persistida, sem tokens de autenticação.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
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
        return Err(TinaError::Unauthorized(
            "conversa fora da instalação ou inexistente".into(),
        ));
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

/// Seleciona um sufixo cronológico do histórico dentro de um orçamento em bytes.
/// A estimativa é conservadora e não substitui o tokenizer do provedor.
/// Nunca corta uma mensagem nem altera o histórico persistido.
pub fn context_window(messages: &[ChatMessage], byte_budget: usize) -> &[ChatMessage] {
    let mut remaining = byte_budget;
    let mut start = messages.len();
    for (index, message) in messages.iter().enumerate().rev() {
        let cost = message.content.len().saturating_add(64);
        if cost > remaining {
            break;
        }
        remaining -= cost;
        start = index;
    }
    &messages[start..]
}

/// Aceita exclusivamente o endpoint oficial configurado para NVIDIA NIM.
/// O transporte HTTP também deve desabilitar redirecionamentos.
pub fn nvidia_endpoint(base_url: &str) -> Result<&'static str> {
    if base_url != "https://integrate.api.nvidia.com/v1" {
        return Err(TinaError::Other("endpoint NVIDIA não autorizado".into()));
    }
    Ok("https://integrate.api.nvidia.com/v1/chat/completions")
}

#[cfg(test)]
mod context_tests {
    use super::*;

    fn message(content: &str) -> ChatMessage {
        ChatMessage {
            id: String::new(),
            role: "user".into(),
            content: content.into(),
            created_at: String::new(),
        }
    }

    #[test]
    fn context_preserves_recent_complete_messages() {
        let messages = vec![message("old"), message("ação"), message("new")];
        let budget = messages[1].content.len() + messages[2].content.len() + 128;
        let selected = context_window(&messages, budget);
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].content, "ação");
        assert_eq!(selected[1].content, "new");
        assert!(context_window(&messages, 1).is_empty());
        assert_eq!(messages.len(), 3);
    }

    #[test]
    fn endpoint_rejects_untrusted_destinations() {
        assert!(nvidia_endpoint("https://integrate.api.nvidia.com/v1").is_ok());
        for url in [
            "http://integrate.api.nvidia.com/v1",
            "https://integrate.api.nvidia.com.evil.example/v1",
            "https://integrate.api.nvidia.com@evil.example/v1",
            "https://integrate.api.nvidia.com/v1?redirect=evil",
        ] {
            assert!(nvidia_endpoint(url).is_err());
        }
    }
}

/// Decodifica eventos textuais NVIDIA NIM sem expor conteúdo de erro do servidor.
/// Mantém bytes incompletos entre chunks, inclusive caracteres UTF-8.
#[derive(Default)]
pub struct NvidiaStreamDecoder {
    pending: Vec<u8>,
    done: bool,
}

impl NvidiaStreamDecoder {
    /// Recebe bytes de transporte e devolve fragmentos de texto completos.
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<String>> {
        if self.done {
            return Ok(Vec::new());
        }
        if self.pending.len().saturating_add(bytes.len()) > 1_048_576 {
            return Err(TinaError::Other("evento NVIDIA excede limite".into()));
        }
        self.pending.extend_from_slice(bytes);
        let mut output = Vec::new();
        while let Some(end) = self.pending.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=end).collect();
            let line = std::str::from_utf8(&line)
                .map_err(|_| TinaError::Other("stream NVIDIA inválido".into()))?;
            if let Some(data) = line.trim_end_matches(['\r', '\n']).strip_prefix("data:") {
                let data = data.trim_start();
                if data == "[DONE]" {
                    self.done = true;
                    self.pending.clear();
                    break;
                }
                if !data.is_empty() {
                    // O JSON será interpretado pelo transporte; não registrar payloads.
                    output.push(data.to_owned());
                }
            }
        }
        Ok(output)
    }

    /// Exige marcador explícito de conclusão, evitando aceitar stream interrompido.
    pub fn finish(&self) -> Result<()> {
        if self.done {
            Ok(())
        } else {
            Err(TinaError::Other("stream NVIDIA interrompido".into()))
        }
    }
}

#[cfg(test)]
mod nvidia_stream_tests {
    use super::*;

    #[test]
    fn fragmented_utf8_and_done() {
        let mut decoder = NvidiaStreamDecoder::default();
        let data = "data: {\"content\":\"ação\"}\r\n\r\ndata: [DONE]\n\n";
        let mut events = Vec::new();
        for byte in data.as_bytes() {
            events.extend(decoder.push(&[*byte]).unwrap());
        }
        assert_eq!(events, vec!["{\"content\":\"ação\"}"]);
        assert!(decoder.finish().is_ok());
    }

    #[test]
    fn rejects_truncated_and_oversized_streams() {
        let mut decoder = NvidiaStreamDecoder::default();
        assert!(decoder.push(b"data: incomplete").unwrap().is_empty());
        assert!(decoder.finish().is_err());
        assert!(decoder.push(&vec![0; 1_048_577]).is_err());
    }
}

/// Transporte interno NVIDIA. O chamador autenticado resolve modelo e orçamento
/// a partir do perfil fixo, e obtém a chave do cofre nativo.
#[allow(dead_code)]
pub(crate) async fn stream_nvidia(
    key: &str,
    model: &str,
    messages: &[ChatMessage],
    max_tokens: u32,
    mut cancelled: tokio::sync::watch::Receiver<bool>,
    mut on_delta: impl FnMut(&str) -> Result<()>,
) -> Result<String> {
    if key.is_empty() || model.trim().is_empty() || max_tokens == 0 || messages.is_empty() {
        return Err(TinaError::Other("configuração NVIDIA inválida".into()));
    }
    if *cancelled.borrow() {
        return Err(TinaError::Other("geração cancelada".into()));
    }
    let client = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|_| TinaError::Other("cliente NVIDIA indisponível".into()))?;
    let payload = serde_json::json!({
        "model": model,
        "messages": messages.iter().map(|m| serde_json::json!({
            "role": m.role, "content": m.content
        })).collect::<Vec<_>>(),
        "max_tokens": max_tokens,
        "stream": true
    });
    let request = client
        .post(nvidia_endpoint("https://integrate.api.nvidia.com/v1")?)
        .bearer_auth(key)
        .json(&payload);
    let work = async {
        let mut response = request
            .send()
            .await
            .map_err(|_| TinaError::Other("falha de conexão NVIDIA".into()))?;
        if !response.status().is_success() {
            return Err(TinaError::Other(format!(
                "NVIDIA HTTP {}",
                response.status().as_u16()
            )));
        }
        let mut decoder = NvidiaStreamDecoder::default();
        let mut answer = String::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| TinaError::Other("falha no stream NVIDIA".into()))?
        {
            for event in decoder.push(&chunk)? {
                let value: serde_json::Value = serde_json::from_str(&event)
                    .map_err(|_| TinaError::Other("evento NVIDIA inválido".into()))?;
                if value.get("error").is_some() {
                    return Err(TinaError::Other("NVIDIA retornou erro de geração".into()));
                }
                if let Some(delta) = value
                    .pointer("/choices/0/delta/content")
                    .and_then(serde_json::Value::as_str)
                {
                    if answer.len().saturating_add(delta.len()) > 100_000 {
                        return Err(TinaError::Other("resposta excede limite local".into()));
                    }
                    on_delta(delta)?;
                    answer.push_str(delta);
                }
            }
            if decoder.done {
                break;
            }
        }
        decoder.finish()?;
        if answer.trim().is_empty() {
            return Err(TinaError::Other("resposta NVIDIA vazia".into()));
        }
        Ok(answer)
    };
    tokio::select! {
        biased;
        _ = async {
            loop {
                if *cancelled.borrow() || cancelled.changed().await.is_err() {
                    break;
                }
            }
        } => Err(TinaError::Other("geração cancelada".into())),
        result = work => result,
    }
}
