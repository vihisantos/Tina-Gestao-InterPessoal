//! Catálogo e avaliação sintética NVIDIA. Não autoriza contratação.
use crate::core::authorization::Operation;
use crate::error::{Result, TinaError};
use serde::Serialize;
use serde_json::{json, Value};
use std::time::{Duration, Instant};

const BASE: &str = "https://integrate.api.nvidia.com/v1";
const SUITE: &str = "tina-synthetic-v1";
const LIMIT: usize = 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct ModelEvaluation {
    pub model: String,
    pub suite: String,
    pub observed_at: String,
    pub arithmetic_passed: bool,
    pub tool_call_passed: bool,
    pub arithmetic_ms: u64,
    pub tool_call_ms: u64,
    pub total_tokens: Option<u64>,
    pub cost: Option<f64>,
}

fn client(base: &str, timeout: u64) -> Result<reqwest::Client> {
    if base != BASE {
        return Err(TinaError::Config("endpoint NVIDIA não autorizado".into()));
    }
    reqwest::Client::builder()
        .https_only(true)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(timeout.clamp(1, 60)))
        .build()
        .map_err(|_| TinaError::Inference("cliente NVIDIA indisponível".into()))
}

async fn response_json(response: reqwest::Result<reqwest::Response>) -> Result<Value> {
    let mut response =
        response.map_err(|_| TinaError::Inference("falha de conexão NVIDIA".into()))?;
    if !response.status().is_success() {
        return Err(TinaError::Inference(format!(
            "NVIDIA HTTP {}",
            response.status().as_u16()
        )));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| TinaError::Inference("resposta NVIDIA interrompida".into()))?
    {
        if bytes.len().saturating_add(chunk.len()) > LIMIT {
            return Err(TinaError::Inference("resposta NVIDIA excede limite".into()));
        }
        bytes.extend_from_slice(&chunk);
    }
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|_| TinaError::Inference("JSON NVIDIA inválido".into()))?;
    if value.get("error").is_some() {
        return Err(TinaError::Inference("NVIDIA retornou erro".into()));
    }
    Ok(value)
}

fn model_ids(value: &Value) -> Result<Vec<String>> {
    let data = value["data"]
        .as_array()
        .ok_or_else(|| TinaError::Inference("catálogo NVIDIA inválido".into()))?;
    let mut ids: Vec<String> = data
        .iter()
        .filter_map(|entry| entry["id"].as_str())
        .filter(|id| !id.is_empty() && id.len() <= 256 && !id.chars().any(char::is_whitespace))
        .map(str::to_owned)
        .collect();
    ids.sort();
    ids.dedup();
    if ids.is_empty() {
        return Err(TinaError::Inference("catálogo NVIDIA vazio".into()));
    }
    Ok(ids)
}

async fn list_models(client: &reqwest::Client, key: &str) -> Result<Vec<String>> {
    let value = response_json(
        client
            .get(format!("{BASE}/models"))
            .bearer_auth(key)
            .send()
            .await,
    )
    .await?;
    model_ids(&value)
}

fn arithmetic_passed(value: &Value) -> bool {
    value.pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .is_some_and(|text| text.trim() == "42")
        && value.pointer("/choices/0/finish_reason").and_then(Value::as_str) == Some("stop")
}

fn tool_passed(value: &Value) -> bool {
    if value.pointer("/choices/0/finish_reason").and_then(Value::as_str) != Some("tool_calls") {
        return false;
    }
    let Some(calls) = value.pointer("/choices/0/message/tool_calls").and_then(Value::as_array)
    else {
        return false;
    };
    if calls.len() != 1 || calls[0].pointer("/function/name").and_then(Value::as_str) != Some("lookup_demo") {
        return false;
    }
    calls[0]
        .pointer("/function/arguments")
        .and_then(Value::as_str)
        .and_then(|args| serde_json::from_str::<Value>(args).ok())
        .is_some_and(|args| args == json!({"id": "demo-17"}))
}

async fn complete(
    client: &reqwest::Client,
    key: &str,
    payload: Value,
) -> Result<(Value, u64)> {
    let started = Instant::now();
    let value = response_json(
        client
            .post(format!("{BASE}/chat/completions"))
            .bearer_auth(key)
            .json(&payload)
            .send()
            .await,
    )
    .await?;
    Ok((value, started.elapsed().as_millis() as u64))
}

impl crate::TinaApp {
    #[cfg(windows)]
    fn evaluation_key(&self) -> Result<String> {
        let entry = keyring::Entry::new("Tina.NVIDIA", &self.config.installation_id)
            .map_err(|_| TinaError::Inference("cofre NVIDIA indisponível".into()))?;
        let key = entry
            .get_password()
            .map_err(|_| TinaError::Inference("configure a chave NVIDIA".into()))?;
        if key.is_empty() {
            return Err(TinaError::Inference("configure a chave NVIDIA".into()));
        }
        Ok(key)
    }

    /// Consulta o catálogo real sem expor credenciais.
    pub async fn owner_nvidia_models(&self) -> Result<Vec<String>> {
        self.require_owner(Operation::Read)?;
        #[cfg(not(windows))]
        return Err(TinaError::Inference("cofre NVIDIA requer Windows".into()));
        #[cfg(windows)]
        {
            let marker = self.owner_access.session_marker()?;
            let key = self.evaluation_key()?;
            let client = client(&self.config.nvidia.base_url, self.config.nvidia.timeout_seconds)?;
            let models = list_models(&client, &key).await?;
            self.require_owner(Operation::Read)?;
            if marker != self.owner_access.session_marker()? {
                return Err(TinaError::Unauthorized("sessão alterada".into()));
            }
            Ok(models)
        }
    }

    /// Executa dois testes sintéticos, sem dados pessoais e sem chamar a tool simulada.
    /// Não representa avaliação completa de competência para contratação.
    pub async fn owner_evaluate_nvidia_model(&self, model: &str) -> Result<ModelEvaluation> {
        self.require_owner(Operation::Write)?;
        #[cfg(not(windows))]
        {
            let _ = model;
            return Err(TinaError::Inference("cofre NVIDIA requer Windows".into()));
        }
        #[cfg(windows)]
        {
            let _guard = self.chat_generation.try_lock()
                .map_err(|_| TinaError::Other("já existe uma geração ativa".into()))?;
            let marker = self.owner_access.session_marker()?;
            let check_session = || {
                self.require_owner(Operation::Write)?;
                if marker != self.owner_access.session_marker()? {
                    return Err(TinaError::Unauthorized("sessão alterada".into()));
                }
                Ok(())
            };
            let key = self.evaluation_key()?;
            let client = client(&self.config.nvidia.base_url, self.config.nvidia.timeout_seconds)?;
            let models = list_models(&client, &key).await?;
            check_session()?;
            if !models.iter().any(|id| id == model) {
                return Err(TinaError::Validation("modelo ausente do catálogo NVIDIA".into()));
            }
            let (arithmetic, arithmetic_ms) = complete(&client, &key, json!({
                "model": model,
                "messages": [{"role":"user","content":"What is 6 times 7? Reply with only the integer."}],
                "max_tokens": 256, "temperature": 0, "stream": false
            })).await?;
            check_session()?;
            let (tool, tool_call_ms) = complete(&client, &key, json!({
                "model": model,
                "messages": [{"role":"user","content":"Use lookup_demo to look up id demo-17. Do not answer in text."}],
                "max_tokens": 256, "temperature": 0, "stream": false,
                "tools": [{"type":"function","function":{"name":"lookup_demo","description":"Look up a synthetic demo ID","parameters":{"type":"object","properties":{"id":{"type":"string"}},"required":["id"],"additionalProperties":false}}}],
                "tool_choice":"auto"
            })).await?;
            check_session()?;
            let total_tokens = arithmetic.pointer("/usage/total_tokens").and_then(Value::as_u64)
                .zip(tool.pointer("/usage/total_tokens").and_then(Value::as_u64))
                .and_then(|(a,b)| a.checked_add(b));
            Ok(ModelEvaluation {
                model: model.to_owned(),
                suite: SUITE.to_owned(),
                observed_at: chrono::Utc::now().to_rfc3339(),
                arithmetic_passed: arithmetic_passed(&arithmetic),
                tool_call_passed: tool_passed(&tool),
                arithmetic_ms,
                tool_call_ms,
                total_tokens,
                cost: None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_rejects_missing_data_and_deduplicates() {
        assert!(model_ids(&json!({})).is_err());
        assert!(model_ids(&json!({"data":[]})).is_err());
        assert_eq!(
            model_ids(&json!({"data":[{"id":"b"},{"id":"a"},{"id":"b"},{"id":" " }]})).unwrap(),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn scores_observed_responses_not_claims() {
        assert!(!arithmetic_passed(&json!({"choices":[{"message":{"content":"42"},"finish_reason":"length"}]})));
        assert!(arithmetic_passed(&json!({"choices":[{"message":{"content":"42"},"finish_reason":"stop"}]})));
        assert!(!tool_passed(&json!({"choices":[{"message":{"content":"I called it"},"finish_reason":"stop"}]})));
        assert!(tool_passed(&json!({"choices":[{"finish_reason":"tool_calls","message":{"tool_calls":[{"function":{"name":"lookup_demo","arguments":"{\"id\":\"demo-17\"}"}}]}}]})));
        assert!(!tool_passed(&json!({"choices":[{"finish_reason":"tool_calls","message":{"tool_calls":[{"function":{"name":"lookup_demo","arguments":"{\"id\":\"wrong\"}"}}]}}]})));
    }
}
