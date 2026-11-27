//! Modelos imutáveis de plano e etapas.
//!
//! O fingerprint é calculado somente sobre a definição das etapas. Estado,
//! resultados e auditoria nunca entram no fingerprint: ele identifica
//! exatamente o que o proprietário autorizou.

use std::collections::{hash_map::DefaultHasher, BTreeSet, HashMap};
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;

use crate::error::{Result, TinaError};
use crate::orchestration::calls::ToolCall;

use super::dependencies::{symbolic_references, topological_order, validate_against_database};

/// Limites padrão aplicados a cada plano persistido.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanLimits {
    /// Máximo de etapas no plano.
    pub max_calls: usize,
    /// Máximo de rodadas de inferência de retomada.
    pub max_rounds: usize,
    /// Orçamento estimado de tokens por plano.
    pub max_tokens_estimate: usize,
}

impl Default for PlanLimits {
    fn default() -> Self {
        Self {
            max_calls: 8,
            max_rounds: 4,
            max_tokens_estimate: 16_000,
        }
    }
}

/// Etapa preparada antes da persistência; ainda não autorizada.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedStep {
    /// Ordem original solicitada pelo modelo, preservada para desempate.
    #[serde(default)]
    pub original_index: usize,
    /// Ordem de execução após validação topológica.
    pub step_index: usize,
    /// Identificador da chamada NVIDIA.
    pub tool_call_id: String,
    /// Ferramenta solicitada.
    pub tool_name: String,
    /// Argumentos originais, com referências simbólicas preservadas.
    pub arguments_json: String,
    /// Identificadores das etapas anteriores das quais depende.
    pub depends_on: Vec<String>,
    /// Efeitos legíveis exibidos no cartão de autorização.
    pub effects: Vec<String>,
    /// Chave de idempotência estável dentro do plano.
    pub idempotency_key: String,
}

/// Plano preparado, ainda não persistido.
#[derive(Debug, Clone)]
pub struct PreparedPlan {
    /// Etapas em ordem de execução.
    pub steps: Vec<PreparedStep>,
    /// Fingerprint da definição imutável do plano.
    pub fingerprint: String,
    /// Chamadas brutas recebidas do modelo, para auditoria.
    pub calls_json: String,
    /// Estimativa conservadora de tokens consumidos pelas definições.
    pub tokens_estimate: i64,
}

/// Plano persistido em `chat_tool_plans`.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ToolPlan {
    /// Identificador do plano.
    pub id: String,
    /// Instalação de origem.
    pub installation_id: String,
    /// Sessão de chat de origem.
    pub session_id: String,
    /// Mensagem do usuário que originou as chamadas.
    pub user_message_id: String,
    /// Estado atual do plano.
    pub state: String,
    /// Fingerprint imutável calculado no planejamento.
    pub fingerprint: String,
    /// Fingerprint registrado no momento da autorização.
    pub authorized_fingerprint: Option<String>,
    /// Chamadas brutas recebidas do modelo.
    pub calls_json: String,
    /// Quantidade de etapas.
    pub steps_count: i64,
    /// Limite de chamadas do plano.
    pub max_calls: i64,
    /// Limite de rodadas de retomada.
    pub max_rounds: i64,
    /// Orçamento estimado de tokens.
    pub max_tokens_estimate: i64,
    /// Tokens estimados já consumidos.
    pub tokens_estimate: i64,
    /// Rodadas de inferência já executadas.
    pub rounds: i64,
    /// Criação.
    pub created_at: String,
    /// Última atualização.
    pub updated_at: String,
    /// Decisão do proprietário.
    pub decided_at: Option<String>,
    /// Conclusão, falha ou cancelamento.
    pub finished_at: Option<String>,
    /// Motivo do cancelamento.
    pub cancel_reason: Option<String>,
    /// Último erro registrado.
    pub last_error: Option<String>,
    /// Plano que substituiu este, quando aplicável.
    pub superseded_by: Option<String>,
}

/// Etapa persistida em `chat_tool_plan_steps`.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct PlanStep {
    /// Identificador da etapa.
    pub id: String,
    /// Plano de origem.
    pub plan_id: String,
    /// Instalação de origem.
    pub installation_id: String,
    /// Sessão de origem.
    pub session_id: String,
    /// Ordem de execução.
    pub step_index: i64,
    /// Identificador da chamada NVIDIA.
    pub tool_call_id: String,
    /// Ferramenta solicitada.
    pub tool_name: String,
    /// Argumentos originais.
    pub arguments_json: String,
    /// Dependências serializadas.
    pub depends_on_json: String,
    /// Efeitos serializados.
    pub effects_json: String,
    /// Chave de idempotência.
    pub idempotency_key: String,
    /// Estado da etapa.
    pub state: String,
    /// Tentativas de execução.
    pub attempts: i64,
    /// Resultado serializado.
    pub result_json: Option<String>,
    /// Mensagem `role: tool` persistida.
    pub result_message_id: Option<String>,
    /// Mensagem `assistant` que criou a proposta desta etapa.
    pub proposal_message_id: Option<String>,
    /// Erro da etapa.
    pub error: Option<String>,
    /// Início da execução.
    pub started_at: Option<String>,
    /// Fim da execução.
    pub finished_at: Option<String>,
}

/// Cartão serializado para a interface: todas as ações antes da autorização.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanCard {
    /// Identificador do plano.
    pub id: String,
    /// Estado atual.
    pub state: String,
    /// Fingerprint que deve ser reenviado na decisão.
    pub fingerprint: String,
    /// Etapas em ordem de execução.
    pub steps: Vec<PlanCardStep>,
    /// Limites do plano.
    pub limits: PlanLimits,
    /// Rodadas já executadas.
    pub rounds: i64,
}

/// Uma ação exibida no cartão de plano.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanCardStep {
    /// Ordem de execução.
    pub step_index: i64,
    /// Identificador da chamada.
    pub tool_call_id: String,
    /// Ferramenta solicitada.
    pub tool_name: String,
    /// Justificativa e efeitos legíveis.
    pub effects: Vec<String>,
    /// Dependências declaradas e derivadas.
    pub depends_on: Vec<String>,
    /// Estado da etapa.
    pub state: String,
}

impl PlanLimits {
    /// Serialização pública dos limites para o cartão.
    pub fn as_json(&self) -> Value {
        json!({
            "maxCalls": self.max_calls,
            "maxRounds": self.max_rounds,
            "maxTokensEstimate": self.max_tokens_estimate
        })
    }
}

/// Prepara um plano a partir das chamadas recebidas, sem gravar nada.
pub async fn prepare(
    pool: &sqlx::SqlitePool,
    installation: &str,
    calls: &[ToolCall],
    limits: PlanLimits,
) -> Result<PreparedPlan> {
    if calls.is_empty() || calls.len() > limits.max_calls {
        return Err(TinaError::ToolError(
            "quantidade de chamadas inválida".into(),
        ));
    }
    let mut ids = BTreeSet::new();
    for call in calls {
        call.validate()?;
        if !ids.insert(call.id.clone()) {
            return Err(TinaError::ToolError(
                "identificador de chamada duplicado".into(),
            ));
        }
    }

    let mut steps = Vec::with_capacity(calls.len());
    for (index, call) in calls.iter().enumerate() {
        let args: Value = serde_json::from_str(&call.arguments)
            .map_err(|_| TinaError::ToolError("argumentos inválidos".into()))?;
        let mut depends_on: BTreeSet<String> =
            super::dependencies::backend_dependencies(call, index, calls);
        for reference in symbolic_references(&args) {
            if !ids.contains(&reference.tool_call_id) {
                return Err(TinaError::ToolError(format!(
                    "referência simbólica desconhecida: {}",
                    reference.tool_call_id
                )));
            }
            let target_index = calls
                .iter()
                .position(|candidate| candidate.id == reference.tool_call_id)
                .ok_or_else(|| TinaError::ToolError("referência simbólica inválida".into()))?;
            if target_index >= index {
                return Err(TinaError::ToolError(
                    "referência simbólica deve apontar para etapa anterior".into(),
                ));
            }
            depends_on.insert(reference.tool_call_id.clone());
        }
        let effects = describe_effects(&call.name, &args);
        steps.push(PreparedStep {
            original_index: index,
            step_index: index,
            tool_call_id: call.id.clone(),
            tool_name: call.name.clone(),
            arguments_json: call.arguments.clone(),
            depends_on: depends_on.into_iter().collect(),
            effects,
            idempotency_key: String::new(),
        });
    }

    let mut steps = topological_order(steps)?;
    let fingerprint = fingerprint_of(&steps);
    for (index, step) in steps.iter_mut().enumerate() {
        step.step_index = index;
        step.idempotency_key = format!("{}:{}", &fingerprint[..16], index);
    }
    validate_against_database(pool, installation, &steps).await?;
    let tokens_estimate = steps
        .iter()
        .map(|step| {
            (step.arguments_json.len() + step.tool_name.len() + step.tool_call_id.len()) as i64 / 4
                + 32
        })
        .sum();
    Ok(PreparedPlan {
        steps,
        fingerprint,
        calls_json: serde_json::to_string(calls)
            .map_err(|_| TinaError::ToolError("serialização falhou".into()))?,
        tokens_estimate,
    })
}

/// Calcula o fingerprint determinístico da definição do plano.
pub fn fingerprint_of(steps: &[PreparedStep]) -> String {
    let canonical = steps
        .iter()
        .map(|step| {
            json!({
                "toolCallId": step.tool_call_id,
                "toolName": step.tool_name,
                "arguments": step.arguments_json,
                "dependsOn": step.depends_on,
                "effects": step.effects
            })
        })
        .collect::<Vec<_>>();
    let encoded = serde_json::to_string(&canonical).unwrap_or_default();
    let mut hasher = DefaultHasher::new();
    encoded.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Descrição legível dos efeitos de uma etapa, exibida antes da autorização.
fn describe_effects(tool_name: &str, args: &Value) -> Vec<String> {
    let reason = args
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or("sem justificativa informada")
        .trim()
        .to_owned();
    let field = |name: &str| {
        args.get(name)
            .and_then(Value::as_str)
            .unwrap_or("(a resolver)")
            .trim()
            .to_owned()
    };
    let mut effects = vec![format!("Justificativa: {reason}")];
    match tool_name {
        "create_organization" => effects.push(format!(
            "Criar organização {} ({})",
            field("name"),
            field("organization_kind")
        )),
        "create_department" => effects.push(format!(
            "Criar departamento {} em {}",
            field("name"),
            field("company_name")
        )),
        "create_position" => effects.push(format!(
            "Criar cargo {} no departamento {}",
            field("name"),
            field("department_name")
        )),
        "generate_contract" => effects.push(format!(
            "Preparar minuta para {} no cargo {} (sem contratação)",
            field("name"),
            field("position_name")
        )),
        "assign_tool" | "assign_app" => effects.push(format!(
            "Conceder {} a {}",
            field("key"),
            field("employee_name")
        )),
        "revoke_tool" | "revoke_app" => effects.push(format!(
            "Revogar {} de {}",
            field("key"),
            field("employee_name")
        )),
        _ => effects.push(format!("Executar {tool_name}")),
    }
    effects
}

/// Converte etapas persistidas em entradas do cartão.
pub fn plan_card(plan: &ToolPlan, steps: &[PlanStep]) -> PlanCard {
    PlanCard {
        id: plan.id.clone(),
        state: plan.state.clone(),
        fingerprint: plan.fingerprint.clone(),
        limits: PlanLimits {
            max_calls: plan.max_calls as usize,
            max_rounds: plan.max_rounds as usize,
            max_tokens_estimate: plan.max_tokens_estimate as usize,
        },
        rounds: plan.rounds,
        steps: steps
            .iter()
            .map(|step| PlanCardStep {
                step_index: step.step_index,
                tool_call_id: step.tool_call_id.clone(),
                tool_name: step.tool_name.clone(),
                effects: serde_json::from_str(&step.effects_json).unwrap_or_default(),
                depends_on: serde_json::from_str(&step.depends_on_json).unwrap_or_default(),
                state: step.state.clone(),
            })
            .collect(),
    }
}

/// Resultado de uma etapa já executada, usado para referências simbólicas.
pub fn result_map(steps: &[PlanStep]) -> HashMap<String, Value> {
    steps
        .iter()
        .filter_map(|step| {
            step.result_json
                .as_ref()
                .and_then(|json| serde_json::from_str::<Value>(json).ok())
                .map(|value| (step.tool_call_id.clone(), value))
        })
        .collect()
}
