//! Execução sequencial, retomada e recuperação segura de planos autorizados.
//!
//! Nenhuma etapa concluída é repetida: o estado persistido é a fonte da verdade.
//! Em falha parcial, as etapas concluídas são preservadas e as dependentes são
//! marcadas como `skipped`. Não existe rollback de efeitos já efetivados.

use std::collections::HashMap;
use std::future::Future;

use serde_json::{json, Value};
use sqlx::SqlitePool;
use tokio::sync::watch;

use crate::error::{Result, TinaError};
use crate::infra::chat::{self, ChatMessage};
use crate::infra::chat_tools;

use super::dependencies::resolve_arguments;
use super::history;
use super::plan::{result_map, ToolPlan};
use super::store;

/// Resultado agregado da execução de um plano.
#[derive(Debug, Clone)]
pub struct ExecutionSummary {
    /// Plano executado.
    pub plan: ToolPlan,
    /// Etapas concluídas nesta chamada.
    pub executed: usize,
    /// Etapas que falharam nesta chamada.
    pub failed: Vec<String>,
    /// Etapas bloqueadas por dependência falha.
    pub skipped: Vec<String>,
    /// Texto final devolvido pelo modelo após os resultados.
    pub final_answer: Option<String>,
}

/// Executa um plano autorizado.
///
/// `resume` recebe o histórico atualizado (incluindo mensagens `role: tool`) e
/// o orçamento restante de tokens, e deve devolver a continuação textual da
/// Tina. A inferência de retomada não recebe ferramentas: qualquer ampliação do
/// plano exige nova autorização.
pub async fn execute_authorized<F, Fut, C>(
    pool: &SqlitePool,
    installation: &str,
    owner_id: &str,
    session: &str,
    plan_id: &str,
    fingerprint: &str,
    cancelled: watch::Receiver<bool>,
    mut resume: F,
    mut check_session: C,
) -> Result<ExecutionSummary>
where
    F: FnMut(Vec<ChatMessage>, i64) -> Fut,
    Fut: Future<Output = Result<String>>,
    C: FnMut() -> Result<()>,
{
    let plan = store::start_plan(pool, installation, session, plan_id, fingerprint).await?;
    if plan.max_calls < 1 || plan.steps_count > plan.max_calls {
        return Err(TinaError::ToolError(
            "plano excede limite de chamadas".into(),
        ));
    }
    let steps = store::load_steps(pool, installation, session, plan_id).await?;
    let mut results: HashMap<String, Value> = result_map(&steps);
    let mut executed = 0usize;
    let mut failed = Vec::new();
    let mut skipped = Vec::new();

    for step in &steps {
        check_session()?;
        if *cancelled.borrow() {
            let reason = "execução cancelada pelo proprietário";
            let cancelled_plan =
                store::cancel_plan(pool, installation, owner_id, session, plan_id, reason).await?;
            store::record_event(
                pool,
                plan_id,
                installation,
                "execution_cancelled",
                &json!({"reason": reason, "executed": executed}),
            )
            .await?;
            return Ok(ExecutionSummary {
                plan: cancelled_plan,
                executed,
                failed,
                skipped,
                final_answer: None,
            });
        }
        if matches!(step.state.as_str(), "executed" | "skipped" | "cancelled") {
            continue;
        }
        if !store::claim_step(pool, installation, plan_id, &step.id).await? {
            continue;
        }
        let outcome =
            execute_step(pool, installation, owner_id, session, &plan, step, &results).await;
        match outcome {
            Ok(result) => {
                let message = history::append_tool_result_message(
                    pool,
                    installation,
                    session,
                    &step.tool_call_id,
                    &result,
                )
                .await?;
                if result.get("status").and_then(Value::as_str) == Some("pending") {
                    store::pause_step_for_approval(
                        pool,
                        installation,
                        plan_id,
                        &step.id,
                        &result,
                        &message.id,
                    )
                    .await?;
                    store::pause_plan_for_approval(pool, installation, session, plan_id).await?;
                    let paused_plan = store::load_plan(pool, installation, session, plan_id)
                        .await?
                        .ok_or_else(|| TinaError::Other("plano não encontrado ao pausar".into()))?;
                    return Ok(ExecutionSummary {
                        plan: paused_plan,
                        executed,
                        failed,
                        skipped,
                        final_answer: None,
                    });
                }
                store::complete_step(pool, installation, plan_id, &step.id, &result, &message.id)
                    .await?;
                results.insert(step.tool_call_id.clone(), result);
                executed += 1;
                store::record_event(
                    pool,
                    plan_id,
                    installation,
                    "step_executed",
                    &json!({"toolCallId": step.tool_call_id, "tool": step.tool_name}),
                )
                .await?;
            }
            Err(error) => {
                let message = error.to_string();
                let result = json!({
                    "status": "failed",
                    "tool_call_id": step.tool_call_id,
                    "error": message
                });
                let recorded = history::append_tool_result_message(
                    pool,
                    installation,
                    session,
                    &step.tool_call_id,
                    &result,
                )
                .await?;
                store::fail_step(pool, installation, plan_id, &step.id, &message).await?;
                store::record_failed_result(
                    pool,
                    installation,
                    plan_id,
                    &step.id,
                    &result,
                    &recorded.id,
                )
                .await?;
                let blocked = store::skip_dependents(
                    pool,
                    installation,
                    plan_id,
                    &step.tool_call_id,
                    &format!("dependência de etapa falha: {}", step.tool_call_id),
                )
                .await?;
                skipped.extend(blocked.into_iter().filter(|id| id != &step.tool_call_id));
                failed.push(step.tool_call_id.clone());
                results.insert(
                    step.tool_call_id.clone(),
                    json!({"status": "failed", "error": message}),
                );
                store::record_event(
                    pool,
                    plan_id,
                    installation,
                    "step_failed",
                    &json!({"toolCallId": step.tool_call_id, "error": message}),
                )
                .await?;
            }
        }
    }

    let mut final_answer = None;
    let history = chat::list_messages(pool, installation, session).await?;
    let remaining_tokens = (plan.max_tokens_estimate - plan.tokens_estimate).max(0);
    check_session()?;
    if !*cancelled.borrow() && plan.rounds < plan.max_rounds && remaining_tokens > 0 {
        match resume(history, remaining_tokens).await {
            Ok(answer) if !*cancelled.borrow() && !answer.trim().is_empty() => {
                chat::append_message(pool, installation, session, "assistant", &answer).await?;
                final_answer = Some(answer);
            }
            Ok(_) => {}
            Err(error) => {
                store::record_event(
                    pool,
                    plan_id,
                    installation,
                    "resume_failed",
                    &json!({"error": error.to_string()}),
                )
                .await?;
            }
        }
    }
    let estimated_tokens = remaining_tokens.min(plan.max_tokens_estimate / 4).max(1);
    store::record_round(pool, installation, plan_id, estimated_tokens).await?;
    let plan = store::finish_plan(
        pool,
        installation,
        session,
        plan_id,
        failed.first().map(String::as_str),
    )
    .await?;
    Ok(ExecutionSummary {
        plan,
        executed,
        failed,
        skipped,
        final_answer,
    })
}

async fn execute_step(
    pool: &SqlitePool,
    installation: &str,
    owner_id: &str,
    session: &str,
    plan: &ToolPlan,
    step: &super::plan::PlanStep,
    results: &HashMap<String, Value>,
) -> Result<Value> {
    let arguments = resolve_arguments(&step.arguments_json, results)?;
    let response = if let Some(message_id) = &step.proposal_message_id {
        ChatMessage {
            id: message_id.clone(),
            role: "assistant".into(),
            content: String::new(),
            created_at: String::new(),
            tool_call_id: None,
            tool_calls_json: None,
        }
    } else {
        let created = chat_tools::propose_for_plan(
            pool,
            installation,
            session,
            &plan.user_message_id,
            &step.tool_name,
            &arguments,
        )
        .await?;
        store::set_step_proposal_message(pool, installation, &plan.id, &step.id, &created.id)
            .await?;
        created
    };
    let Some((proposal_id, mut result)) =
        store::proposal_result(pool, installation, session, &response.id).await?
    else {
        return Ok(json!({
            "status": "no_proposal",
            "tool_call_id": step.tool_call_id,
            "message_id": response.id,
            "content": response.content
        }));
    };
    if result.get("status").and_then(Value::as_str) == Some("pending") {
        // A plan step may prepare a proposal, but it must never self-approve
        // an owner-controlled side effect. Leave the proposal pending so the
        // authenticated owner can decide through the explicit approval path.
        check_session()?;
        result["requires_owner_approval"] = Value::Bool(true);
        return Ok(result);
    }
    result["tool_call_id"] = Value::String(step.tool_call_id.clone());
    Ok(result)
}
