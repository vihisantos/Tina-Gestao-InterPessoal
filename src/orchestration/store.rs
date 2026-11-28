//! Persistência de planos, etapas e eventos de auditoria do orquestrador.
//!
//! Toda transição de estado acontece em transação e com `installation_id` como
//! filtro obrigatório. Planos ativos nunca são substituídos por uma nova
//! mensagem; a substituição é uma operação explícita e auditada.

use serde_json::{json, Value};
use sqlx::{Sqlite, SqlitePool, Transaction};
use uuid::Uuid;

use crate::error::{Result, TinaError};
use crate::infra::{begin_owner_write, commit_owner_write};

use super::plan::{PlanCard, PlanLimits, PlanStep, PreparedPlan, PreparedStep, ToolPlan};

const PLAN_SELECT: &str = "SELECT id,installation_id,session_id,user_message_id,state,\
     fingerprint,authorized_fingerprint,calls_json,steps_count,max_calls,max_rounds,\
     max_tokens_estimate,tokens_estimate,rounds,created_at,updated_at,decided_at,\
     finished_at,cancel_reason,last_error,superseded_by FROM chat_tool_plans";

const STEP_SELECT: &str = "SELECT id,plan_id,installation_id,session_id,step_index,\
     tool_call_id,tool_name,arguments_json,depends_on_json,effects_json,idempotency_key,\
     state,attempts,result_json,result_message_id,proposal_message_id,error,started_at,finished_at \
     FROM chat_tool_plan_steps";

/// Cria um plano aguardando autorização. Falha explicitamente se já houver plano ativo.
pub async fn create_plan(
    pool: &SqlitePool,
    installation: &str,
    session: &str,
    user_message_id: &str,
    prepared: &PreparedPlan,
    limits: PlanLimits,
) -> Result<ToolPlan> {
    let plan_id = Uuid::new_v4().to_string();
    let mut tx = pool.begin().await?;
    let active: Option<String> = sqlx::query_scalar(
        "SELECT id FROM chat_tool_plans WHERE installation_id=? AND session_id=? \
         AND state IN ('draft','awaiting_authorization','authorized','running') LIMIT 1",
    )
    .bind(installation)
    .bind(session)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(active) = active {
        return Err(TinaError::ToolError(format!(
            "já existe um plano ativo ({active}); aprove, recuse ou cancele antes de continuar"
        )));
    }

    // Propostas avulsas pendentes são substituídas de forma explícita e
    // auditável pelo plano, que passa a ser a única porta de autorização.
    for table in [
        "chat_department_proposals",
        "chat_organization_proposals",
        "chat_position_proposals",
        "chat_tool_access_proposals",
        "chat_contract_drafts",
    ] {
        let query = format!(
            "UPDATE {table} SET status='rejected', lifecycle_status='superseded', \
             superseded_by=?, decided_at=datetime('now'), updated_at=datetime('now') \
             WHERE installation_id=? AND session_id=? AND lifecycle_status='pending'"
        );
        sqlx::query(&query)
            .bind(&plan_id)
            .bind(installation)
            .bind(session)
            .execute(&mut *tx)
            .await?;
    }

    sqlx::query(
        "INSERT INTO chat_tool_plans(id,installation_id,session_id,user_message_id,state,\
         fingerprint,calls_json,steps_count,max_calls,max_rounds,max_tokens_estimate,tokens_estimate) \
         VALUES (?,?,?,?,'awaiting_authorization',?,?,?,?,?,?,?)",
    )
    .bind(&plan_id)
    .bind(installation)
    .bind(session)
    .bind(user_message_id)
    .bind(&prepared.fingerprint)
    .bind(&prepared.calls_json)
    .bind(prepared.steps.len() as i64)
    .bind(limits.max_calls as i64)
    .bind(limits.max_rounds as i64)
    .bind(limits.max_tokens_estimate as i64)
    .bind(prepared.tokens_estimate)
    .execute(&mut *tx)
    .await?;

    insert_steps(&mut tx, installation, session, &plan_id, &prepared.steps).await?;
    record_event_in(
        &mut tx,
        &plan_id,
        installation,
        "planned",
        &json!({"fingerprint": prepared.fingerprint, "steps": prepared.steps.len()}),
    )
    .await?;
    tx.commit().await?;
    load_plan(pool, installation, session, &plan_id)
        .await?
        .ok_or_else(|| TinaError::Other("plano não persistido".into()))
}

async fn insert_steps(
    tx: &mut Transaction<'_, Sqlite>,
    installation: &str,
    session: &str,
    plan_id: &str,
    steps: &[PreparedStep],
) -> Result<()> {
    for step in steps {
        sqlx::query(
            "INSERT INTO chat_tool_plan_steps(id,plan_id,installation_id,session_id,step_index,\
             tool_call_id,tool_name,arguments_json,depends_on_json,effects_json,idempotency_key,state) \
             VALUES (?,?,?,?,?,?,?,?,?,?,?,'planned')",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(plan_id)
        .bind(installation)
        .bind(session)
        .bind(step.step_index as i64)
        .bind(&step.tool_call_id)
        .bind(&step.tool_name)
        .bind(&step.arguments_json)
        .bind(serde_json::to_string(&step.depends_on).unwrap_or_else(|_| "[]".into()))
        .bind(serde_json::to_string(&step.effects).unwrap_or_else(|_| "[]".into()))
        .bind(&step.idempotency_key)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// Carrega um plano específico dentro da instalação e sessão informadas.
pub async fn load_plan(
    pool: &SqlitePool,
    installation: &str,
    session: &str,
    plan_id: &str,
) -> Result<Option<ToolPlan>> {
    Ok(sqlx::query_as::<_, ToolPlan>(&format!(
        "{PLAN_SELECT} WHERE id=? AND installation_id=? AND session_id=?"
    ))
    .bind(plan_id)
    .bind(installation)
    .bind(session)
    .fetch_optional(pool)
    .await?)
}

/// Carrega o plano ativo da sessão, se houver.
pub async fn load_active_plan(
    pool: &SqlitePool,
    installation: &str,
    session: &str,
) -> Result<Option<ToolPlan>> {
    Ok(sqlx::query_as::<_, ToolPlan>(&format!(
        "{PLAN_SELECT} WHERE installation_id=? AND session_id=? \
         AND state IN ('draft','awaiting_authorization','authorized','running') \
         ORDER BY created_at DESC, rowid DESC LIMIT 1"
    ))
    .bind(installation)
    .bind(session)
    .fetch_optional(pool)
    .await?)
}

/// Carrega as etapas de um plano em ordem de execução.
pub async fn load_steps(
    pool: &SqlitePool,
    installation: &str,
    session: &str,
    plan_id: &str,
) -> Result<Vec<PlanStep>> {
    Ok(sqlx::query_as::<_, PlanStep>(&format!(
        "{STEP_SELECT} WHERE plan_id=? AND installation_id=? AND session_id=? ORDER BY step_index"
    ))
    .bind(plan_id)
    .bind(installation)
    .bind(session)
    .fetch_all(pool)
    .await?)
}

/// Carrega etapas sem exigir a sessão, usado pela recuperação interna.
pub async fn load_steps_by_plan(
    pool: &SqlitePool,
    installation: &str,
    plan_id: &str,
) -> Result<Vec<PlanStep>> {
    Ok(sqlx::query_as::<_, PlanStep>(&format!(
        "{STEP_SELECT} WHERE plan_id=? AND installation_id=? ORDER BY step_index"
    ))
    .bind(plan_id)
    .bind(installation)
    .fetch_all(pool)
    .await?)
}

/// Monta o cartão completo exibido ao proprietário.
pub async fn load_card(
    pool: &SqlitePool,
    installation: &str,
    session: &str,
) -> Result<Option<PlanCard>> {
    let Some(plan) = load_active_plan(pool, installation, session).await? else {
        return Ok(None);
    };
    let steps = load_steps(pool, installation, session, &plan.id).await?;
    Ok(Some(super::plan::plan_card(&plan, &steps)))
}

/// Registra a decisão do proprietário, vinculada ao fingerprint autorizado.
pub async fn authorize_plan(
    pool: &SqlitePool,
    installation: &str,
    owner_id: &str,
    session: &str,
    plan_id: &str,
    fingerprint: &str,
    approved: bool,
) -> Result<ToolPlan> {
    let mut tx = begin_owner_write(pool, installation, owner_id).await?;
    let plan: ToolPlan = sqlx::query_as::<_, ToolPlan>(&format!(
        "{PLAN_SELECT} WHERE id=? AND installation_id=? AND session_id=?"
    ))
    .bind(plan_id)
    .bind(installation)
    .bind(session)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| TinaError::Unauthorized("plano inexistente".into()))?;
    if plan.state != "awaiting_authorization" {
        return Err(TinaError::ToolError(format!(
            "plano não está aguardando autorização: {}",
            plan.state
        )));
    }
    if plan.fingerprint != fingerprint {
        return Err(TinaError::ToolError(
            "o plano foi alterado após a apresentação; nova autorização necessária".into(),
        ));
    }
    let (state, step_state) = if approved {
        ("authorized", "authorized")
    } else {
        ("cancelled", "cancelled")
    };
    sqlx::query(
        "UPDATE chat_tool_plans SET state=?, authorized_fingerprint=?, decided_at=datetime('now'), \
         finished_at=CASE WHEN ?='cancelled' THEN datetime('now') ELSE NULL END, \
         updated_at=datetime('now') WHERE id=? AND installation_id=? AND fingerprint=?",
    )
    .bind(state)
    .bind(if approved { Some(fingerprint) } else { None })
    .bind(state)
    .bind(plan_id)
    .bind(installation)
    .bind(fingerprint)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE chat_tool_plan_steps SET state=? WHERE plan_id=? AND installation_id=? AND state='planned'",
    )
    .bind(step_state)
    .bind(plan_id)
    .bind(installation)
    .execute(&mut *tx)
    .await?;
    record_event_in(
        &mut tx,
        plan_id,
        installation,
        if approved { "authorized" } else { "rejected" },
        &json!({"fingerprint": fingerprint}),
    )
    .await?;
    sqlx::query(
        "INSERT INTO organizational_audit_events(id,installation_id,entity_type,entity_id,action,outcome,actor_type,actor_id,metadata_json) \
         VALUES (?,?,'tool_plan',?,?,'success','owner',?,?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(installation)
    .bind(plan_id)
    .bind(if approved { "authorize_plan" } else { "reject_plan" })
    .bind(owner_id)
    .bind(json!({"fingerprint": fingerprint}).to_string())
    .execute(&mut *tx)
    .await?;
    commit_owner_write(tx, installation).await?;
    load_plan(pool, installation, session, plan_id)
        .await?
        .ok_or_else(|| TinaError::Other("plano não encontrado após decisão".into()))
}

async fn record_event_in(
    tx: &mut Transaction<'_, Sqlite>,
    plan_id: &str,
    installation: &str,
    action: &str,
    detail: &Value,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO chat_tool_plan_events(id,plan_id,installation_id,action,detail_json) \
         VALUES (?,?,?,?,?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(plan_id)
    .bind(installation)
    .bind(action)
    .bind(detail.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Registra um evento de execução do plano.
pub async fn record_event(
    pool: &SqlitePool,
    plan_id: &str,
    installation: &str,
    action: &str,
    detail: &Value,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    record_event_in(&mut tx, plan_id, installation, action, detail).await?;
    tx.commit().await?;
    Ok(())
}

/// Marca o plano como em execução, com verificação de fingerprint.
///
/// Planos já `running` também são aceitos para retomada após interrupção: as
/// etapas concluídas permanecem gravadas e não são repetidas.
pub async fn start_plan(
    pool: &SqlitePool,
    installation: &str,
    session: &str,
    plan_id: &str,
    fingerprint: &str,
) -> Result<ToolPlan> {
    let result = sqlx::query(
        "UPDATE chat_tool_plans SET state='running', updated_at=datetime('now') \
         WHERE id=? AND installation_id=? AND session_id=? \
           AND state IN ('authorized','running') AND authorized_fingerprint=?",
    )
    .bind(plan_id)
    .bind(installation)
    .bind(session)
    .bind(fingerprint)
    .execute(pool)
    .await?;
    if result.rows_affected() != 1 {
        return Err(TinaError::ToolError(
            "plano não autorizado ou alterado após decisão".into(),
        ));
    }
    load_plan(pool, installation, session, plan_id)
        .await?
        .ok_or_else(|| TinaError::Other("plano não encontrado".into()))
}

/// Inicia uma etapa de forma idempotente; retorna `false` se já foi executada.
pub async fn claim_step(
    pool: &SqlitePool,
    installation: &str,
    plan_id: &str,
    step_id: &str,
) -> Result<bool> {
    let current: Option<String> = sqlx::query_scalar(
        "SELECT state FROM chat_tool_plan_steps WHERE id=? AND plan_id=? AND installation_id=?",
    )
    .bind(step_id)
    .bind(plan_id)
    .bind(installation)
    .fetch_optional(pool)
    .await?;
    match current.as_deref() {
        Some("executed" | "failed" | "skipped" | "cancelled") => Ok(false),
        Some("authorized") | Some("running") => {
            sqlx::query(
                "UPDATE chat_tool_plan_steps SET state='running', attempts=attempts+1, \
                 started_at=COALESCE(started_at,datetime('now')), error=NULL \
                 WHERE id=? AND plan_id=? AND installation_id=? AND state IN ('authorized','running')",
            )
            .bind(step_id)
            .bind(plan_id)
            .bind(installation)
            .execute(pool)
            .await?;
            Ok(true)
        }
        Some(state) => Err(TinaError::ToolError(format!(
            "etapa em estado não executável: {state}"
        ))),
        None => Err(TinaError::Unauthorized("etapa fora da instalação".into())),
    }
}

/// Conclui uma etapa com resultado e mensagem `role: tool` persistida.
pub async fn complete_step(
    pool: &SqlitePool,
    installation: &str,
    plan_id: &str,
    step_id: &str,
    result: &Value,
    result_message_id: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE chat_tool_plan_steps SET state='executed', result_json=?, \
         result_message_id=?, finished_at=datetime('now'), error=NULL \
         WHERE id=? AND plan_id=? AND installation_id=?",
    )
    .bind(result.to_string())
    .bind(result_message_id)
    .bind(step_id)
    .bind(plan_id)
    .bind(installation)
    .execute(pool)
    .await?;
    Ok(())
}

/// Persiste uma proposta pendente sem consumar a etapa.
pub async fn pause_step_for_approval(
    pool: &SqlitePool,
    installation: &str,
    plan_id: &str,
    step_id: &str,
    result: &Value,
    result_message_id: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE chat_tool_plan_steps SET state='authorized', result_json=?,          result_message_id=?, finished_at=NULL, error=NULL          WHERE id=? AND plan_id=? AND installation_id=?",
    )
    .bind(result.to_string())
    .bind(result_message_id)
    .bind(step_id)
    .bind(plan_id)
    .bind(installation)
    .execute(pool)
    .await?;
    Ok(())
}

/// Devolve o plano ao estado autorizado enquanto aguarda decisão do proprietário.
pub async fn pause_plan_for_approval(
    pool: &SqlitePool,
    installation: &str,
    session: &str,
    plan_id: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE chat_tool_plans SET state='authorized', updated_at=datetime('now')          WHERE id=? AND installation_id=? AND session_id=? AND state='running'",
    )
    .bind(plan_id)
    .bind(installation)
    .bind(session)
    .execute(pool)
    .await?;
    Ok(())
}

/// Persista resultado e mensagem de uma etapa falha sem alterar seu estado.
pub async fn record_failed_result(
    pool: &SqlitePool,
    installation: &str,
    plan_id: &str,
    step_id: &str,
    result: &Value,
    result_message_id: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE chat_tool_plan_steps SET result_json=?, result_message_id=?, \
         finished_at=datetime('now') WHERE id=? AND plan_id=? AND installation_id=?",
    )
    .bind(result.to_string())
    .bind(result_message_id)
    .bind(step_id)
    .bind(plan_id)
    .bind(installation)
    .execute(pool)
    .await?;
    Ok(())
}

/// Registra a mensagem de proposta criada para uma etapa, permitindo retomada.
pub async fn set_step_proposal_message(
    pool: &SqlitePool,
    installation: &str,
    plan_id: &str,
    step_id: &str,
    proposal_message_id: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE chat_tool_plan_steps SET proposal_message_id=? \
         WHERE id=? AND plan_id=? AND installation_id=?",
    )
    .bind(proposal_message_id)
    .bind(step_id)
    .bind(plan_id)
    .bind(installation)
    .execute(pool)
    .await?;
    Ok(())
}

/// Registra falha de uma etapa sem desfazer efeitos já concluídos.
pub async fn fail_step(
    pool: &SqlitePool,
    installation: &str,
    plan_id: &str,
    step_id: &str,
    error: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE chat_tool_plan_steps SET state='failed', error=?, finished_at=datetime('now') \
         WHERE id=? AND plan_id=? AND installation_id=?",
    )
    .bind(error)
    .bind(step_id)
    .bind(plan_id)
    .bind(installation)
    .execute(pool)
    .await?;
    Ok(())
}

/// Marca dependentes de uma etapa falha como `skipped`, sem repetir efeitos.
pub async fn skip_dependents(
    pool: &SqlitePool,
    installation: &str,
    plan_id: &str,
    failed_tool_call_id: &str,
    reason: &str,
) -> Result<Vec<String>> {
    let steps = load_steps_by_plan(pool, installation, plan_id).await?;
    let mut blocked = std::collections::BTreeSet::new();
    blocked.insert(failed_tool_call_id.to_owned());
    loop {
        let mut changed = false;
        for step in &steps {
            if blocked.contains(&step.tool_call_id) {
                continue;
            }
            let deps: Vec<String> = serde_json::from_str(&step.depends_on_json).unwrap_or_default();
            if deps.iter().any(|dep| blocked.contains(dep)) {
                blocked.insert(step.tool_call_id.clone());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for step in &steps {
        if !blocked.contains(&step.tool_call_id) || step.tool_call_id == failed_tool_call_id {
            continue;
        }
        if matches!(
            step.state.as_str(),
            "executed" | "failed" | "skipped" | "cancelled"
        ) {
            continue;
        }
        sqlx::query(
            "UPDATE chat_tool_plan_steps SET state='skipped', error=?, finished_at=datetime('now') \
             WHERE id=? AND plan_id=? AND installation_id=?",
        )
        .bind(reason)
        .bind(&step.id)
        .bind(plan_id)
        .bind(installation)
        .execute(pool)
        .await?;
    }
    Ok(blocked.into_iter().collect())
}

/// Finaliza o plano com o estado agregado das etapas.
pub async fn finish_plan(
    pool: &SqlitePool,
    installation: &str,
    session: &str,
    plan_id: &str,
    error: Option<&str>,
) -> Result<ToolPlan> {
    let failed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM chat_tool_plan_steps WHERE plan_id=? AND installation_id=? \
         AND state IN ('failed','skipped')",
    )
    .bind(plan_id)
    .bind(installation)
    .fetch_one(pool)
    .await?;
    let state = if failed > 0 { "failed" } else { "completed" };
    sqlx::query(
        "UPDATE chat_tool_plans SET state=?, last_error=?, finished_at=datetime('now'), \
         updated_at=datetime('now') WHERE id=? AND installation_id=? AND session_id=? \
         AND state='running'",
    )
    .bind(state)
    .bind(error)
    .bind(plan_id)
    .bind(installation)
    .bind(session)
    .execute(pool)
    .await?;
    load_plan(pool, installation, session, plan_id)
        .await?
        .ok_or_else(|| TinaError::Other("plano não encontrado ao finalizar".into()))
}

/// Cancela explicitamente um plano ativo, com motivo e auditoria.
pub async fn cancel_plan(
    pool: &SqlitePool,
    installation: &str,
    owner_id: &str,
    session: &str,
    plan_id: &str,
    reason: &str,
) -> Result<ToolPlan> {
    let mut tx = begin_owner_write(pool, installation, owner_id).await?;
    let plan: ToolPlan = sqlx::query_as::<_, ToolPlan>(&format!(
        "{PLAN_SELECT} WHERE id=? AND installation_id=? AND session_id=?"
    ))
    .bind(plan_id)
    .bind(installation)
    .bind(session)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| TinaError::Unauthorized("plano inexistente".into()))?;
    if !matches!(
        plan.state.as_str(),
        "awaiting_authorization" | "authorized" | "running"
    ) {
        return Err(TinaError::ToolError(format!(
            "plano não pode ser cancelado no estado {}",
            plan.state
        )));
    }
    sqlx::query(
        "UPDATE chat_tool_plans SET state='cancelled', cancel_reason=?, \
         decided_at=COALESCE(decided_at,datetime('now')), finished_at=datetime('now'), \
         updated_at=datetime('now') WHERE id=? AND installation_id=?",
    )
    .bind(reason)
    .bind(plan_id)
    .bind(installation)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE chat_tool_plan_steps SET state='cancelled', error=?, finished_at=datetime('now') \
         WHERE plan_id=? AND installation_id=? AND state NOT IN ('executed','failed')",
    )
    .bind(reason)
    .bind(plan_id)
    .bind(installation)
    .execute(&mut *tx)
    .await?;
    record_event_in(
        &mut tx,
        plan_id,
        installation,
        "cancelled",
        &json!({"reason": reason}),
    )
    .await?;
    sqlx::query(
        "INSERT INTO organizational_audit_events(id,installation_id,entity_type,entity_id,action,outcome,actor_type,actor_id,metadata_json) \
         VALUES (?,?,'tool_plan',?,'cancel_plan','success','owner',?,?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(installation)
    .bind(plan_id)
    .bind(owner_id)
    .bind(json!({"reason": reason}).to_string())
    .execute(&mut *tx)
    .await?;
    commit_owner_write(tx, installation).await?;
    load_plan(pool, installation, session, plan_id)
        .await?
        .ok_or_else(|| TinaError::Other("plano não encontrado após cancelamento".into()))
}

/// Substitui explicitamente um plano ativo e registra o motivo auditável.
pub async fn supersede_plan(
    pool: &SqlitePool,
    installation: &str,
    owner_id: &str,
    session: &str,
    plan_id: &str,
    reason: &str,
) -> Result<ToolPlan> {
    let replacement = Uuid::new_v4().to_string();
    let mut tx = begin_owner_write(pool, installation, owner_id).await?;
    let plan: ToolPlan = sqlx::query_as::<_, ToolPlan>(&format!(
        "{PLAN_SELECT} WHERE id=? AND installation_id=? AND session_id=?"
    ))
    .bind(plan_id)
    .bind(installation)
    .bind(session)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| TinaError::Unauthorized("plano inexistente".into()))?;
    if !matches!(
        plan.state.as_str(),
        "awaiting_authorization" | "authorized" | "running"
    ) {
        return Err(TinaError::ToolError(
            "somente plano ativo pode ser substituído".into(),
        ));
    }
    sqlx::query(
        "UPDATE chat_tool_plans SET state='superseded', superseded_by=?, \
         finished_at=datetime('now'), cancel_reason=?, updated_at=datetime('now') WHERE id=?",
    )
    .bind(&replacement)
    .bind(reason)
    .bind(plan_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE chat_tool_plan_steps SET state='cancelled', error=?, finished_at=datetime('now') \
         WHERE plan_id=? AND state NOT IN ('executed','failed')",
    )
    .bind(reason)
    .bind(plan_id)
    .execute(&mut *tx)
    .await?;
    record_event_in(
        &mut tx,
        plan_id,
        installation,
        "superseded",
        &json!({"reason": reason, "replacement": replacement}),
    )
    .await?;
    sqlx::query(
        "INSERT INTO organizational_audit_events(id,installation_id,entity_type,entity_id,action,outcome,actor_type,actor_id,metadata_json) \
         VALUES (?,?,'tool_plan',?,'supersede_plan','success','owner',?,?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(installation)
    .bind(plan_id)
    .bind(owner_id)
    .bind(json!({"reason": reason, "replacement": replacement}).to_string())
    .execute(&mut *tx)
    .await?;
    commit_owner_write(tx, installation).await?;
    load_plan(pool, installation, session, plan_id)
        .await?
        .ok_or_else(|| TinaError::Other("plano não encontrado após substituição".into()))
}

/// Incrementa contadores de rodadas e tokens estimados do plano.
pub async fn record_round(
    pool: &SqlitePool,
    installation: &str,
    plan_id: &str,
    tokens: i64,
) -> Result<()> {
    sqlx::query(
        "UPDATE chat_tool_plans SET rounds=rounds+1, tokens_estimate=tokens_estimate+?, \
         updated_at=datetime('now') WHERE id=? AND installation_id=?",
    )
    .bind(tokens)
    .bind(plan_id)
    .bind(installation)
    .execute(pool)
    .await?;
    Ok(())
}

/// Recupera o resultado persistido de uma proposta pela mensagem que a criou.
pub async fn proposal_result(
    pool: &SqlitePool,
    installation: &str,
    session: &str,
    proposal_message_id: &str,
) -> Result<Option<(String, Value)>> {
    let query = "SELECT id,'create_department' AS tool,company_id,name,code,lifecycle_status FROM chat_department_proposals WHERE installation_id=? AND session_id=? AND proposal_message_id=? \
        UNION ALL SELECT id,'create_organization','',name,'',lifecycle_status FROM chat_organization_proposals WHERE installation_id=? AND session_id=? AND proposal_message_id=? \
        UNION ALL SELECT id,'create_position',company_id,name,'',lifecycle_status FROM chat_position_proposals WHERE installation_id=? AND session_id=? AND proposal_message_id=? \
        UNION ALL SELECT id,action,company_id,employee_name,'',lifecycle_status FROM chat_tool_access_proposals WHERE installation_id=? AND session_id=? AND proposal_message_id=? \
        UNION ALL SELECT id,'generate_contract',company_id,name,'',lifecycle_status FROM chat_contract_drafts WHERE installation_id=? AND session_id=? AND proposal_message_id=? LIMIT 1";
    let row: Option<(String, String, String, String, String, String)> = sqlx::query_as(query)
        .bind(installation)
        .bind(session)
        .bind(proposal_message_id)
        .bind(installation)
        .bind(session)
        .bind(proposal_message_id)
        .bind(installation)
        .bind(session)
        .bind(proposal_message_id)
        .bind(installation)
        .bind(session)
        .bind(proposal_message_id)
        .bind(installation)
        .bind(session)
        .bind(proposal_message_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|(id, tool, company, name, code, status)| {
        (
            id.clone(),
            json!({
                "id": id,
                "tool": tool,
                "company_id": company,
                "name": name,
                "code": code,
                "status": status
            }),
        )
    }))
}
