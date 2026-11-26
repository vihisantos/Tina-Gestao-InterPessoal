//! Execução experimental de um funcionário, iniciada diretamente pelo proprietário.
//! A única ferramenta executável nesta etapa é leitura de cargos do próprio departamento.

use crate::core::authorization::{
    Actor, ActorContext, AuthorizationPolicy, Operation, ResourceScope,
};
use crate::error::{Result, TinaError};
use serde_json::{json, Value};
use sqlx::SqlitePool;

const LIST: &str = "list_positions";

#[derive(Debug, PartialEq, sqlx::FromRow)]
struct Scope {
    company_id: String,
    department_id: String,
    name: String,
    position: String,
    model: String,
}

async fn scope(pool: &SqlitePool, installation: &str, employee: &str) -> Result<Scope> {
    sqlx::query_as("SELECT c.id AS company_id,d.id AS department_id,e.name,p.name AS position,e.model_config AS model FROM employees e JOIN positions p ON p.id=e.position_id JOIN departments d ON d.id=p.department_id JOIN companies c ON c.id=d.company_id WHERE c.installation_id=? AND e.id=? AND e.status='active' AND p.status='active' AND d.status='active' AND c.status='active'")
        .bind(installation).bind(employee).fetch_optional(pool).await?
        .ok_or_else(|| TinaError::Unauthorized("funcionário ativo não encontrado nesta instalação".into()))
}

fn context(installation: &str, employee: &str, scope: &Scope) -> ActorContext {
    ActorContext::verified(
        installation.to_owned(),
        Actor::Employee {
            employee_id: employee.to_owned(),
            company_id: scope.company_id.clone(),
        },
    )
}

async fn read_positions(
    pool: &SqlitePool,
    actor: &ActorContext,
    scope: &Scope,
    args: &Value,
) -> Result<Value> {
    if args != &json!({}) {
        return Err(TinaError::ToolError(
            "list_positions não recebe parâmetros".into(),
        ));
    }
    super::tool_access::check_employee_tool(
        pool,
        actor,
        &scope.company_id,
        &scope.department_id,
        LIST,
    )
    .await?;
    AuthorizationPolicy::authorize(
        actor,
        Operation::Read,
        ResourceScope {
            installation_id: actor.installation_id(),
            company_id: Some(&scope.company_id),
        },
    )?;
    // Escopo e concessão são conferidos novamente na consulta que lê os dados.
    let Actor::Employee { employee_id, .. } = actor.actor() else {
        return Err(TinaError::Unauthorized("funcionário necessário".into()));
    };
    let rows: Vec<String> = sqlx::query_scalar("SELECT p.name FROM positions p JOIN departments d ON d.id=p.department_id JOIN companies c ON c.id=d.company_id WHERE c.installation_id=? AND c.id=? AND d.id=? AND c.status='active' AND d.status='active' AND p.status='active' AND EXISTS(SELECT 1 FROM employee_tool_grants g JOIN employees e ON e.id=g.employee_id JOIN positions ep ON ep.id=e.position_id WHERE g.installation_id=c.installation_id AND g.company_id=c.id AND g.department_id=d.id AND g.employee_id=? AND g.tool_key='list_positions' AND e.status='active' AND ep.status='active' AND ep.department_id=d.id) ORDER BY p.name LIMIT 50")
        .bind(actor.installation_id()).bind(&scope.company_id).bind(&scope.department_id).bind(employee_id).fetch_all(pool).await?;
    super::tool_access::check_employee_tool(
        pool,
        actor,
        &scope.company_id,
        &scope.department_id,
        LIST,
    )
    .await?;
    Ok(json!({"positions":rows,"limit":50}))
}

/// Resposta real de uma execução experimental individual.
#[derive(Debug, serde::Serialize)]
pub struct EmployeeRun {
    /// Texto retornado pelo modelo.
    pub answer: String,
    /// Modelo cadastrado utilizado; não implica certificação de contratação.
    pub model: String,
    /// Ferramenta realmente executada, quando aplicável.
    pub tool: Option<String>,
    /// Tempo total da inferência, em milissegundos.
    pub inference_ms: u64,
}

impl crate::TinaApp {
    /// Executa uma tarefa isolada para funcionário ativo, mediante ação do proprietário.
    /// Não concede acesso, não contrata e não permite ferramentas administrativas.
    pub async fn owner_run_employee(&self, employee: &str, task: &str) -> Result<EmployeeRun> {
        self.require_owner(Operation::Write)?;
        if task.trim().is_empty() || task.len() > 4000 {
            return Err(TinaError::Validation("tarefa vazia ou extensa".into()));
        }
        #[cfg(not(windows))]
        {
            let _ = employee;
            return Err(TinaError::Inference("cofre NVIDIA requer Windows".into()));
        }
        #[cfg(windows)]
        {
            let _guard = self
                .chat_generation
                .try_lock()
                .map_err(|_| TinaError::Other("já existe uma geração ativa".into()))?;
            let marker = self.owner_access.session_marker()?;
            let check_session = || {
                self.require_owner(Operation::Write)?;
                if marker != self.owner_access.session_marker()? {
                    return Err(TinaError::Unauthorized("sessão alterada".into()));
                }
                Ok(())
            };
            let pool = self.database.pool();
            let installation = &self.config.installation_id;
            let initial = scope(pool, installation, employee).await?;
            let actor = context(installation, employee, &initial);
            let can_read = match super::tool_access::check_employee_tool(
                pool,
                &actor,
                &initial.company_id,
                &initial.department_id,
                LIST,
            )
            .await
            {
                Ok(()) => true,
                Err(TinaError::PermissionDenied(_)) => false,
                Err(error) => return Err(error),
            };
            let key = self.evaluation_key()?;
            let client = super::nvidia_evaluation::client(
                &self.config.nvidia.base_url,
                self.config.nvidia.timeout_seconds,
            )?;
            let mut messages = vec![
                json!({"role":"system","content":"Você é um funcionário IA da Tina executando uma tarefa pontual do proprietário. Responda no seu papel e não invente ferramentas ou resultados. Não possui autoridade administrativa. Não execute ações externas. Se não houver dados, informe."}),
                json!({"role":"user","content":format!("Nome cadastrado: {}. Cargo: {}. Tarefa do proprietário: {}",initial.name,initial.position,task)}),
            ];
            let mut payload =
                json!({"model":initial.model,"messages":messages,"max_tokens":1024,"stream":false});
            if can_read {
                payload["tools"] = json!([{"type":"function","function":{"name":LIST,"description":"Consulta até 50 cargos ativos do próprio departamento.","parameters":{"type":"object","properties":{},"additionalProperties":false}}}]);
                payload["tool_choice"] = json!("auto");
            }
            check_session()?;
            let (mut output, mut inference_ms) =
                super::nvidia_evaluation::complete(&client, &key, payload).await?;
            check_session()?;
            if scope(pool, installation, employee).await? != initial {
                return Err(TinaError::Unauthorized(
                    "escopo do funcionário alterado".into(),
                ));
            }
            let mut used = None;
            if output
                .pointer("/choices/0/finish_reason")
                .and_then(Value::as_str)
                == Some("tool_calls")
            {
                let calls = output
                    .pointer("/choices/0/message/tool_calls")
                    .and_then(Value::as_array)
                    .ok_or_else(|| TinaError::ToolError("chamada inválida".into()))?;
                if !can_read
                    || calls.len() != 1
                    || calls[0].pointer("/function/name").and_then(Value::as_str) != Some(LIST)
                {
                    return Err(TinaError::PermissionDenied(
                        "ferramenta não habilitada".into(),
                    ));
                }
                let id = calls[0]["id"]
                    .as_str()
                    .filter(|id| !id.is_empty() && id.len() <= 256)
                    .ok_or_else(|| TinaError::ToolError("identificador inválido".into()))?;
                let args = calls[0]
                    .pointer("/function/arguments")
                    .and_then(Value::as_str)
                    .filter(|s| s.len() <= 128)
                    .and_then(|s| serde_json::from_str::<Value>(s).ok())
                    .ok_or_else(|| TinaError::ToolError("argumentos inválidos".into()))?;
                let result = read_positions(pool, &actor, &initial, &args).await?;
                check_session()?;
                sqlx::query("INSERT INTO organizational_audit_events(id,installation_id,company_id,entity_type,entity_id,action,outcome,actor_type,actor_id,metadata_json) VALUES (?,?,?,'tool',?,'list_positions','success','employee',?,?)")
                    .bind(uuid::Uuid::new_v4().to_string()).bind(installation).bind(&initial.company_id)
                    .bind(employee).bind(employee).bind(json!({"initiated_by":self.config.owner_uuid,"arguments":args,"result":result}).to_string())
                    .execute(pool).await?;
                messages.push(json!({"role":"assistant","content":null,"tool_calls":[{"id":id,"type":"function","function":{"name":LIST,"arguments":"{}"}}]}));
                messages
                    .push(json!({"role":"tool","tool_call_id":id,"content":result.to_string()}));
                super::tool_access::check_employee_tool(
                    pool,
                    &actor,
                    &initial.company_id,
                    &initial.department_id,
                    LIST,
                )
                .await?;
                check_session()?;
                let (final_output, elapsed) = super::nvidia_evaluation::complete(&client, &key,
                    json!({"model":initial.model,"messages":messages,"max_tokens":1024,"stream":false})).await?;
                output = final_output;
                inference_ms = inference_ms.saturating_add(elapsed);
                used = Some(LIST.to_owned());
            }
            check_session()?;
            if scope(pool, installation, employee).await? != initial {
                return Err(TinaError::Unauthorized(
                    "escopo do funcionário alterado".into(),
                ));
            }
            if used.is_some() {
                super::tool_access::check_employee_tool(
                    pool,
                    &actor,
                    &initial.company_id,
                    &initial.department_id,
                    LIST,
                )
                .await?;
            }
            if output
                .pointer("/choices/0/finish_reason")
                .and_then(Value::as_str)
                != Some("stop")
            {
                return Err(TinaError::Inference(
                    "resposta do funcionário incompleta".into(),
                ));
            }
            let answer = output
                .pointer("/choices/0/message/content")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty() && s.len() <= 32_768)
                .ok_or_else(|| TinaError::Inference("resposta inválida".into()))?
                .to_owned();
            sqlx::query("INSERT INTO organizational_audit_events(id,installation_id,company_id,entity_type,entity_id,action,outcome,actor_type,actor_id,metadata_json) VALUES (?,?,?,'employee',?,'experimental_run','success','owner',?,?)")
                .bind(uuid::Uuid::new_v4().to_string()).bind(installation).bind(&initial.company_id)
                .bind(employee).bind(&self.config.owner_uuid)
                .bind(json!({"model":initial.model,"tool":used,"inference_ms":inference_ms}).to_string())
                .execute(pool).await?;
            Ok(EmployeeRun {
                answer,
                model: initial.model,
                tool: used,
                inference_ms,
            })
        }
    }
}
