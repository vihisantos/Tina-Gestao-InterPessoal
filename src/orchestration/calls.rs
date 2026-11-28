//! Acumulação indexada de `tool_calls` recebidos em stream SSE.
//!
//! O provedor pode fragmentar uma chamada em vários eventos e pode emitir
//! varias chamadas no mesmo turno (`index` 0, 1, 2...). O acumulador preserva
//! a ordem dos índices, valida cada chamada e impõe limites antes de qualquer
//! persistência. Nenhum dado do modelo é executado aqui.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Result, TinaError};

/// Número máximo de chamadas de ferramenta em uma única rodada de inferência.
pub const MAX_TOOL_CALLS_PER_TURN: usize = 8;
/// Tamanho máximo do identificador de uma chamada, em bytes.
pub const MAX_TOOL_CALL_ID_BYTES: usize = 256;
/// Tamanho máximo do nome de uma ferramenta, em bytes.
pub const MAX_TOOL_NAME_BYTES: usize = 100;
/// Tamanho máximo dos argumentos de uma chamada, em bytes.
pub const MAX_TOOL_ARGUMENTS_BYTES: usize = 16_384;
/// Orçamento total de argumentos por rodada, em bytes.
pub const MAX_TOTAL_ARGUMENTS_BYTES: usize = 65_536;

/// Uma chamada de ferramenta completa, reconstruída a partir dos deltas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    /// Índice do delta na resposta do provedor; preserva a ordem solicitada.
    pub index: u64,
    /// Identificador da chamada, usado em `tool_call_id` no histórico NIM.
    pub id: String,
    /// Nome da ferramenta solicitada.
    pub name: String,
    /// Argumentos em JSON, ainda não resolvidos nem autorizados.
    pub arguments: String,
}

impl ToolCall {
    /// Valida uma chamada já acumulada, sem depender do catálogo de ferramentas.
    pub fn validate(&self) -> Result<()> {
        if self.id.is_empty() || self.id.len() > MAX_TOOL_CALL_ID_BYTES {
            return Err(TinaError::ToolError(
                "identificador de chamada inválido".into(),
            ));
        }
        if self.name.is_empty() || self.name.len() > MAX_TOOL_NAME_BYTES {
            return Err(TinaError::ToolError("nome de ferramenta inválido".into()));
        }
        if !self
            .name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            return Err(TinaError::ToolError(
                "nome de ferramenta não suportado".into(),
            ));
        }
        if self.arguments.is_empty() || self.arguments.len() > MAX_TOOL_ARGUMENTS_BYTES {
            return Err(TinaError::ToolError(
                "argumentos de chamada inválidos".into(),
            ));
        }
        match serde_json::from_str::<Value>(&self.arguments) {
            Ok(Value::Object(_)) => Ok(()),
            Ok(_) => Err(TinaError::ToolError(
                "argumentos de chamada devem ser um objeto".into(),
            )),
            Err(_) => Err(TinaError::ToolError(
                "argumentos de chamada inválidos".into(),
            )),
        }
    }
}

/// Acumula deltas `choices[0].delta.tool_calls` em chamadas completas.
#[derive(Debug, Default)]
pub struct ToolCallAccumulator {
    calls: BTreeMap<u64, ToolCall>,
    total_arguments_bytes: usize,
    next_implicit_index: u64,
}

impl ToolCallAccumulator {
    /// Cria um acumulador vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Quantidade de chamadas distintas vistas até agora.
    pub fn len(&self) -> usize {
        self.calls.len()
    }

    /// Verdadeiro quando nenhum delta de ferramenta foi recebido.
    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }

    /// Incorpora um evento SSE ao acumulador.
    ///
    /// Índice ausente é aceito e recebe o próximo índice implícito livre; isso
    /// evita rejeitar provedores que omitem o campo em deltas parciais.
    pub fn push(&mut self, value: &Value) -> Result<()> {
        let Some(entries) = value.pointer("/choices/0/delta/tool_calls") else {
            return Ok(());
        };
        let entries = entries
            .as_array()
            .ok_or_else(|| TinaError::ToolError("tool_calls inválido".into()))?;
        for item in entries {
            if item
                .get("type")
                .and_then(Value::as_str)
                .is_some_and(|kind| kind != "function")
            {
                return Err(TinaError::ToolError("tipo de chamada não suportado".into()));
            }
            let index = match item.get("index").and_then(Value::as_u64) {
                Some(index) => {
                    self.next_implicit_index =
                        self.next_implicit_index.max(index.saturating_add(1));
                    index
                }
                None => {
                    let index = self.next_implicit_index;
                    self.next_implicit_index = self.next_implicit_index.saturating_add(1);
                    index
                }
            };
            if !self.calls.contains_key(&index) && self.calls.len() >= MAX_TOOL_CALLS_PER_TURN {
                return Err(TinaError::ToolError("muitas chamadas de ferramenta".into()));
            }
            let call = self.calls.entry(index).or_insert_with(|| ToolCall {
                index,
                id: String::new(),
                name: String::new(),
                arguments: String::new(),
            });
            if let Some(id) = item.get("id").and_then(Value::as_str) {
                if !call.id.is_empty() && call.id != id {
                    return Err(TinaError::ToolError(
                        "identificador de chamada conflitante".into(),
                    ));
                }
                if id.len() > MAX_TOOL_CALL_ID_BYTES {
                    return Err(TinaError::ToolError(
                        "identificador de chamada extenso".into(),
                    ));
                }
                call.id = id.to_owned();
            }
            for (pointer, target, limit) in [
                ("/function/name", &mut call.name, MAX_TOOL_NAME_BYTES),
                (
                    "/function/arguments",
                    &mut call.arguments,
                    MAX_TOOL_ARGUMENTS_BYTES,
                ),
            ] {
                let Some(fragment) = item.pointer(pointer).and_then(Value::as_str) else {
                    continue;
                };
                if target.len().saturating_add(fragment.len()) > limit {
                    return Err(TinaError::ToolError("chamada excede limite".into()));
                }
                if pointer == "/function/arguments"
                    && self.total_arguments_bytes.saturating_add(fragment.len())
                        > MAX_TOTAL_ARGUMENTS_BYTES
                {
                    return Err(TinaError::ToolError(
                        "orçamento total de argumentos excedido".into(),
                    ));
                }
                if pointer == "/function/arguments" {
                    self.total_arguments_bytes =
                        self.total_arguments_bytes.saturating_add(fragment.len());
                }
                target.push_str(fragment);
            }
        }
        Ok(())
    }

    /// Finaliza a rodada e devolve as chamadas em ordem de índice.
    pub fn finish(self) -> Result<Vec<ToolCall>> {
        if self.calls.is_empty() {
            return Err(TinaError::ToolError("chamada sem conclusão".into()));
        }
        let mut ids = std::collections::BTreeSet::new();
        let mut calls = Vec::with_capacity(self.calls.len());
        for (_, call) in self.calls {
            call.validate()?;
            if !ids.insert(call.id.clone()) {
                return Err(TinaError::ToolError(
                    "identificador de chamada duplicado".into(),
                ));
            }
            calls.push(call);
        }
        Ok(calls)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn delta(index: u64, id: &str, name: &str, arguments: &str) -> Value {
        json!({"choices":[{"delta":{"tool_calls":[{
            "index": index,
            "id": id,
            "type": "function",
            "function": {"name": name, "arguments": arguments}
        }]}}]})
    }

    #[test]
    fn accumulates_fragmented_arguments_for_single_call() {
        let mut accumulator = ToolCallAccumulator::new();
        accumulator
            .push(&delta(0, "call-1", "create_department", "{\"name\":"))
            .unwrap();
        accumulator
            .push(&json!({"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"Finanças\"}"}}]}}]}))
            .unwrap();
        let calls = accumulator.finish().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "create_department");
        assert_eq!(
            serde_json::from_str::<Value>(&calls[0].arguments).unwrap()["name"],
            "Finanças"
        );
    }

    #[test]
    fn accepts_multiple_indexed_calls_in_order() {
        let mut accumulator = ToolCallAccumulator::new();
        accumulator
            .push(&delta(
                1,
                "call-2",
                "create_organization",
                "{\"name\":\"Pessoal\"}",
            ))
            .unwrap();
        accumulator
            .push(&delta(
                0,
                "call-1",
                "create_department",
                "{\"name\":\"Finanças\"}",
            ))
            .unwrap();
        accumulator
            .push(&delta(1, "call-2", "create_organization", ""))
            .unwrap();
        let calls = accumulator.finish().unwrap();
        assert_eq!(
            calls.iter().map(|c| c.index).collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(calls[0].id, "call-1");
        assert_eq!(calls[1].id, "call-2");
    }

    #[test]
    fn accepts_delta_without_index() {
        let mut accumulator = ToolCallAccumulator::new();
        accumulator
            .push(&json!({"choices":[{"delta":{"tool_calls":[{
                "id":"call-1","type":"function",
                "function":{"name":"create_position","arguments":"{}"}
            }]}}]}))
            .unwrap();
        let calls = accumulator.finish().unwrap();
        assert_eq!(calls[0].index, 0);
    }

    #[test]
    fn rejects_unsupported_type_duplicate_ids_and_limits() {
        let mut accumulator = ToolCallAccumulator::new();
        assert!(accumulator
            .push(&json!({"choices":[{"delta":{"tool_calls":[{"index":0,"type":"custom"}]}}]}))
            .is_err());

        let mut duplicate = ToolCallAccumulator::new();
        duplicate
            .push(&delta(0, "same", "create_department", "{}"))
            .unwrap();
        duplicate
            .push(&delta(1, "same", "create_organization", "{}"))
            .unwrap();
        assert!(duplicate.finish().is_err());

        let mut oversized = ToolCallAccumulator::new();
        let long = "a".repeat(MAX_TOOL_ARGUMENTS_BYTES + 1);
        assert!(oversized
            .push(&delta(0, "call-1", "create_department", &long))
            .is_err());
    }

    #[test]
    fn rejects_more_calls_than_allowed() {
        let mut accumulator = ToolCallAccumulator::new();
        for index in 0..MAX_TOOL_CALLS_PER_TURN {
            accumulator
                .push(&delta(
                    index as u64,
                    &format!("call-{index}"),
                    "create_department",
                    "{}",
                ))
                .unwrap();
        }
        assert!(accumulator
            .push(&delta(
                MAX_TOOL_CALLS_PER_TURN as u64,
                "overflow",
                "create_department",
                "{}",
            ))
            .is_err());
    }

    #[test]
    fn finish_rejects_incomplete_calls() {
        let mut accumulator = ToolCallAccumulator::new();
        accumulator
            .push(&delta(0, "call-1", "create_department", ""))
            .unwrap();
        assert!(accumulator.finish().is_err());
    }
}
