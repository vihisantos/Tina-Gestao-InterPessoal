//! Orquestração de ferramentas: plano, autorização, execução e retomada.
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::error::Result;
use crate::infra::chat;
use crate::orchestration::{calls::ToolCall, executor, history, plan, store};
use crate::TinaApp;

fn tool(index: u64, id: &str, name: &str, arguments: Value) -> ToolCall {
    ToolCall {
        index,
        id: id.to_owned(),
        name: name.to_owned(),
        arguments: arguments.to_string(),
    }
}

async fn setup(name: &str, prompt: &str) -> (TinaApp, PathBuf, String, String) {
    let path = std::env::temp_dir().join(format!(
        "tina-orchestration-{}-{}.db",
        name,
        uuid::Uuid::new_v4()
    ));
    let installation = format!("orch-{name}-installation");
    let owner = format!("orch-{name}-owner");
    let app = TinaApp::initialize_at(&path, &installation, &owner)
        .await
        .unwrap();
    app.unlock_owner_for_test().unwrap();
    let session = app.owner_create_chat("Plano de ferramentas").await.unwrap();
    let user = app
        .owner_append_chat_message(&session.id, prompt)
        .await
        .unwrap();
    (app, path, session.id, user.id)
}

fn ok_session() -> impl FnMut() -> Result<()> {
    || Ok(())
}

fn two_step_calls() -> Vec<ToolCall> {
    vec![
        tool(
            0,
            "call-org",
            "create_organization",
            json!({
                "organization_kind": "business",
                "name": "Empresa Teste",
                "reason": "Separar as operações da empresa"
            }),
        ),
        tool(
            1,
            "call-dept",
            "create_department",
            json!({
                "organization_kind": "business",
                "company_name": "@call-org.name",
                "name": "Finanças",
                "code": "FIN",
                "description": "Controlar finanças",
                "reason": "Organizar as finanças da empresa"
            }),
        ),
    ]
}

async fn create_authorized(
    app: &TinaApp,
    session: &str,
    user: &str,
    calls: &[ToolCall],
) -> (
    crate::orchestration::plan::ToolPlan,
    crate::orchestration::plan::PreparedPlan,
) {
    let installation = &app.config().installation_id;
    let prepared = plan::prepare(
        app.database().pool(),
        installation,
        calls,
        plan::PlanLimits::default(),
    )
    .await
    .unwrap();
    history::append_tool_call_message(app.database().pool(), installation, session, calls)
        .await
        .unwrap();
    let created = store::create_plan(
        app.database().pool(),
        installation,
        session,
        user,
        &prepared,
        plan::PlanLimits::default(),
    )
    .await
    .unwrap();
    let authorized = store::authorize_plan(
        app.database().pool(),
        installation,
        &app.config().owner_uuid,
        session,
        &created.id,
        &prepared.fingerprint,
        true,
    )
    .await
    .unwrap();
    (authorized, prepared)
}

#[tokio::test]
async fn plan_is_immutable_and_only_one_stays_active_per_session() {
    let (app, path, session, user) =
        setup("immutable", "Crie a empresa e o departamento financeiro.").await;
    let installation = &app.config().installation_id;
    let calls = two_step_calls();
    let prepared = plan::prepare(
        app.database().pool(),
        installation,
        &calls,
        plan::PlanLimits::default(),
    )
    .await
    .unwrap();
    let created = store::create_plan(
        app.database().pool(),
        installation,
        &session,
        &user,
        &prepared,
        plan::PlanLimits::default(),
    )
    .await
    .unwrap();
    assert_eq!(created.state, "awaiting_authorization");
    assert!(store::create_plan(
        app.database().pool(),
        installation,
        &session,
        &user,
        &prepared,
        plan::PlanLimits::default(),
    )
    .await
    .is_err());

    assert!(store::authorize_plan(
        app.database().pool(),
        installation,
        &app.config().owner_uuid,
        &session,
        &created.id,
        "fingerprint-forjado",
        true,
    )
    .await
    .is_err());

    let authorized = store::authorize_plan(
        app.database().pool(),
        installation,
        &app.config().owner_uuid,
        &session,
        &created.id,
        &prepared.fingerprint,
        true,
    )
    .await
    .unwrap();
    assert_eq!(authorized.state, "authorized");
    assert_eq!(
        authorized.authorized_fingerprint.as_deref(),
        Some(prepared.fingerprint.as_str())
    );

    let card = store::load_card(app.database().pool(), installation, &session)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(card.steps.len(), 2);
    assert_eq!(card.steps[1].depends_on, vec!["call-org".to_owned()]);
    assert!(store::load_plan(
        app.database().pool(),
        "outra-instalacao",
        &session,
        &created.id
    )
    .await
    .unwrap()
    .is_none());

    drop(app);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn authorized_plan_executes_in_order_and_resolves_symbolic_reference() {
    let (app, path, session, user) = setup(
        "sequence",
        "Crie a empresa e depois o departamento financeiro.",
    )
    .await;
    let installation = &app.config().installation_id;
    let (authorized, _prepared) = create_authorized(&app, &session, &user, &two_step_calls()).await;
    let (_tx, cancelled) = tokio::sync::watch::channel(false);
    let summary = executor::execute_authorized(
        app.database().pool(),
        installation,
        &app.config().owner_uuid,
        &session,
        &authorized.id,
        authorized.authorized_fingerprint.as_deref().unwrap(),
        cancelled,
        |_, _| async { Ok("Plano executado.".to_owned()) },
        ok_session(),
    )
    .await
    .unwrap();

    assert_eq!(summary.executed, 2);
    assert!(summary.failed.is_empty());
    assert_eq!(summary.plan.state, "completed");
    assert_eq!(summary.final_answer.as_deref(), Some("Plano executado."));

    let companies: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM companies WHERE installation_id=? AND name='Empresa Teste'",
    )
    .bind(installation)
    .fetch_all(app.database().pool())
    .await
    .unwrap();
    assert_eq!(companies.len(), 1);
    let departments: Vec<String> = sqlx::query_scalar(
        "SELECT d.name FROM departments d JOIN companies c ON c.id=d.company_id \
         WHERE c.installation_id=? AND d.name='Finanças'",
    )
    .bind(installation)
    .fetch_all(app.database().pool())
    .await
    .unwrap();
    assert_eq!(departments, vec!["Finanças".to_owned()]);

    let tool_calls: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM chat_messages WHERE installation_id=? AND session_id=? \
         AND tool_calls_json IS NOT NULL",
    )
    .bind(installation)
    .bind(&session)
    .fetch_one(app.database().pool())
    .await
    .unwrap();
    assert_eq!(tool_calls, 1);
    let tool_results: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM chat_messages WHERE installation_id=? AND session_id=? \
         AND tool_call_id IS NOT NULL",
    )
    .bind(installation)
    .bind(&session)
    .fetch_one(app.database().pool())
    .await
    .unwrap();
    assert_eq!(tool_results, 2);

    drop(app);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn failed_step_skips_dependents_and_preserves_completed_effects() {
    let (app, path, session, user) =
        setup("partial", "Crie a empresa e tente as etapas seguintes.").await;
    let installation = &app.config().installation_id;
    let calls = vec![
        two_step_calls()[0].clone(),
        tool(
            1,
            "call-dept",
            "create_department",
            json!({
                "organization_kind": "business",
                "company_name": "@call-org.name",
                "name": "Finanças",
                "code": "FIN",
                "bogus": true,
                "reason": "Teste de falha controlada"
            }),
        ),
        tool(
            2,
            "call-position",
            "create_position",
            json!({
                "organization_kind": "business",
                "company_name": "Empresa Teste",
                "department_name": "@call-dept.name",
                "name": "Analista",
                "level": 1,
                "description": "Analisar números",
                "reason": "Depende do departamento"
            }),
        ),
    ];
    let (authorized, _prepared) = create_authorized(&app, &session, &user, &calls).await;
    let (_tx, cancelled) = tokio::sync::watch::channel(false);
    let summary = executor::execute_authorized(
        app.database().pool(),
        installation,
        &app.config().owner_uuid,
        &session,
        &authorized.id,
        authorized.authorized_fingerprint.as_deref().unwrap(),
        cancelled,
        |_, _| async { Ok(String::new()) },
        ok_session(),
    )
    .await
    .unwrap();

    assert_eq!(summary.executed, 1);
    assert_eq!(summary.failed, vec!["call-dept".to_owned()]);
    assert!(summary.skipped.contains(&"call-position".to_owned()));
    assert_eq!(summary.plan.state, "failed");
    let companies: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM companies WHERE installation_id=? AND name='Empresa Teste'",
    )
    .bind(installation)
    .fetch_one(app.database().pool())
    .await
    .unwrap();
    assert_eq!(companies, 1, "efeito concluído não pode ser desfeito");
    let positions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM positions WHERE name='Analista'")
        .fetch_one(app.database().pool())
        .await
        .unwrap();
    assert_eq!(positions, 0);

    drop(app);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn explicit_cancellation_preserves_completed_steps_and_permits_new_plan() {
    let (app, path, session, user) = setup("cancel", "Crie a empresa e o departamento.").await;
    let installation = &app.config().installation_id;
    let calls = two_step_calls();
    let prepared = plan::prepare(
        app.database().pool(),
        installation,
        &calls,
        plan::PlanLimits::default(),
    )
    .await
    .unwrap();
    let created = store::create_plan(
        app.database().pool(),
        installation,
        &session,
        &user,
        &prepared,
        plan::PlanLimits::default(),
    )
    .await
    .unwrap();
    let authorized = store::authorize_plan(
        app.database().pool(),
        installation,
        &app.config().owner_uuid,
        &session,
        &created.id,
        &prepared.fingerprint,
        true,
    )
    .await
    .unwrap();
    store::start_plan(
        app.database().pool(),
        installation,
        &session,
        &authorized.id,
        &prepared.fingerprint,
    )
    .await
    .unwrap();

    let steps = store::load_steps(
        app.database().pool(),
        installation,
        &session,
        &authorized.id,
    )
    .await
    .unwrap();
    let result = json!({"id":"company-concluida","name":"Empresa Teste","status":"created"});
    let message = history::append_tool_result_message(
        app.database().pool(),
        installation,
        &session,
        &steps[0].tool_call_id,
        &result,
    )
    .await
    .unwrap();
    store::complete_step(
        app.database().pool(),
        installation,
        &authorized.id,
        &steps[0].id,
        &result,
        &message.id,
    )
    .await
    .unwrap();

    let cancelled = store::cancel_plan(
        app.database().pool(),
        installation,
        &app.config().owner_uuid,
        &session,
        &authorized.id,
        "Cancelamento explícito",
    )
    .await
    .unwrap();
    assert_eq!(cancelled.state, "cancelled");
    let steps = store::load_steps(
        app.database().pool(),
        installation,
        &session,
        &authorized.id,
    )
    .await
    .unwrap();
    assert_eq!(steps[0].state, "executed");
    assert_eq!(steps[1].state, "cancelled");

    let replacement = store::create_plan(
        app.database().pool(),
        installation,
        &session,
        &user,
        &prepared,
        plan::PlanLimits::default(),
    )
    .await
    .unwrap();
    assert_ne!(replacement.id, authorized.id);
    assert_eq!(replacement.state, "awaiting_authorization");

    drop(app);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn cancellation_signal_stops_execution_before_any_step() {
    let (app, path, session, user) = setup("signal", "Crie a empresa e o departamento.").await;
    let installation = &app.config().installation_id;
    let (authorized, _prepared) = create_authorized(&app, &session, &user, &two_step_calls()).await;
    let (_tx, cancelled) = tokio::sync::watch::channel(true);
    let summary = executor::execute_authorized(
        app.database().pool(),
        installation,
        &app.config().owner_uuid,
        &session,
        &authorized.id,
        authorized.authorized_fingerprint.as_deref().unwrap(),
        cancelled,
        |_, _| async { Ok(String::new()) },
        ok_session(),
    )
    .await
    .unwrap();
    assert_eq!(summary.executed, 0);
    assert_eq!(summary.plan.state, "cancelled");
    let steps = store::load_steps(
        app.database().pool(),
        installation,
        &session,
        &authorized.id,
    )
    .await
    .unwrap();
    assert!(steps.iter().all(|step| step.state == "cancelled"));

    drop(app);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn interrupted_plan_resumes_without_repeating_completed_step() {
    let (app, path, session, user) = setup("resume", "Crie a empresa e o departamento.").await;
    let installation = &app.config().installation_id;
    let (authorized, prepared) = create_authorized(&app, &session, &user, &two_step_calls()).await;
    store::start_plan(
        app.database().pool(),
        installation,
        &session,
        &authorized.id,
        &prepared.fingerprint,
    )
    .await
    .unwrap();

    let steps = store::load_steps(
        app.database().pool(),
        installation,
        &session,
        &authorized.id,
    )
    .await
    .unwrap();
    let result = json!({"id":"company-retomada","name":"Empresa Teste","status":"created"});
    let message = history::append_tool_result_message(
        app.database().pool(),
        installation,
        &session,
        &steps[0].tool_call_id,
        &result,
    )
    .await
    .unwrap();
    store::complete_step(
        app.database().pool(),
        installation,
        &authorized.id,
        &steps[0].id,
        &result,
        &message.id,
    )
    .await
    .unwrap();

    let (_tx, cancelled) = tokio::sync::watch::channel(false);
    let summary = executor::execute_authorized(
        app.database().pool(),
        installation,
        &app.config().owner_uuid,
        &session,
        &authorized.id,
        &prepared.fingerprint,
        cancelled,
        |_, _| async { Ok("Retomado.".to_owned()) },
        ok_session(),
    )
    .await
    .unwrap();
    assert_eq!(summary.executed, 1);
    assert_eq!(summary.plan.state, "completed");

    let resumed_steps = store::load_steps(
        app.database().pool(),
        installation,
        &session,
        &authorized.id,
    )
    .await
    .unwrap();
    assert_eq!(resumed_steps[0].attempts, 0);
    assert_eq!(resumed_steps[1].attempts, 1);
    assert_eq!(resumed_steps[1].state, "executed");

    drop(app);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn tool_history_is_compatible_with_nvidia_protocol() {
    let (app, path, session, _user) = setup("history", "Solicite uma ação.").await;
    let installation = &app.config().installation_id;
    let calls = vec![two_step_calls()[0].clone()];
    let request =
        history::append_tool_call_message(app.database().pool(), installation, &session, &calls)
            .await
            .unwrap();
    assert!(request.tool_calls_json.is_some());
    let result = history::append_tool_result_message(
        app.database().pool(),
        installation,
        &session,
        &calls[0].id,
        &json!({"id":"company-history","name":"Empresa Teste"}),
    )
    .await
    .unwrap();
    assert_eq!(result.tool_call_id.as_deref(), Some("call-org"));

    let messages = chat::list_messages(app.database().pool(), installation, &session)
        .await
        .unwrap();
    let nvidia = chat::build_nvidia_messages(&messages);
    assert!(nvidia.iter().any(|message| {
        message.get("role").and_then(Value::as_str) == Some("tool")
            && message.get("tool_call_id").and_then(Value::as_str) == Some("call-org")
    }));
    assert!(nvidia.iter().any(|message| {
        message.get("role").and_then(Value::as_str) == Some("assistant")
            && message
                .get("tool_calls")
                .and_then(Value::as_array)
                .is_some_and(|items| !items.is_empty())
    }));

    drop(app);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn pending_owner_proposal_pauses_plan_without_auto_approval() {
    let (app, path, session, user) = setup("pending", "Crie uma empresa.").await;
    let installation = &app.config().installation_id;
    let calls = vec![two_step_calls()[0].clone()];
    let (authorized, _prepared) = create_authorized(&app, &session, &user, &calls).await;
    let (_tx, cancelled) = tokio::sync::watch::channel(false);
    let summary = executor::execute_authorized(
        app.database().pool(),
        installation,
        &app.config().owner_uuid,
        &session,
        &authorized.id,
        authorized.authorized_fingerprint.as_deref().unwrap(),
        cancelled,
        |_, _| async { Ok("não deve chamar".to_owned()) },
        ok_session(),
    )
    .await
    .unwrap();

    assert_eq!(summary.plan.state, "authorized");
    assert_eq!(summary.executed, 0);
    assert!(summary.final_answer.is_none());
    let step = store::load_steps(
        app.database().pool(),
        installation,
        &session,
        &authorized.id,
    )
    .await
    .unwrap()
    .remove(0);
    assert_eq!(step.state, "authorized");
    let result: Value = serde_json::from_str(step.result_json.as_deref().unwrap()).unwrap();
    assert_eq!(result["status"], "pending");
    assert_eq!(result["requires_owner_approval"], true);

    drop(app);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn planning_does_not_supersede_unrelated_pending_proposals() {
    let (app, path, session, user) = setup(
        "preserve-pending",
        "Prepare uma ação e mantenha decisões separadas.",
    )
    .await;
    let installation = &app.config().installation_id;

    crate::infra::chat_tools::propose_for_plan(
        app.database().pool(),
        installation,
        &session,
        &user,
        "create_organization",
        &json!({
            "organization_kind": "business",
            "name": "Proposta independente",
            "reason": "Aguardar decisão explícita"
        }),
    )
    .await
    .unwrap();

    let pending_before: String = sqlx::query_scalar(
        "SELECT lifecycle_status FROM chat_organization_proposals
         WHERE installation_id=? AND session_id=? AND name=?",
    )
    .bind(installation)
    .bind(&session)
    .bind("Proposta independente")
    .fetch_one(app.database().pool())
    .await
    .unwrap();
    assert_eq!(pending_before, "pending");

    let calls = two_step_calls();
    let prepared = plan::prepare(
        app.database().pool(),
        installation,
        &calls,
        plan::PlanLimits::default(),
    )
    .await
    .unwrap();
    let created = store::create_plan(
        app.database().pool(),
        installation,
        &session,
        &user,
        &prepared,
        plan::PlanLimits::default(),
    )
    .await
    .unwrap();
    assert_eq!(created.state, "awaiting_authorization");

    let pending_after: String = sqlx::query_scalar(
        "SELECT lifecycle_status FROM chat_organization_proposals
         WHERE installation_id=? AND session_id=? AND name=?",
    )
    .bind(installation)
    .bind(&session)
    .bind("Proposta independente")
    .fetch_one(app.database().pool())
    .await
    .unwrap();
    assert_eq!(pending_after, "pending");

    drop(app);
    let _ = std::fs::remove_file(path);
}
