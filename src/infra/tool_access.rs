//! Aplicativos agrupam ferramentas reais. Concessão não substitui a autorização da ação.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Sqlite, SqlitePool, Transaction};
use uuid::Uuid;

use super::audit::{begin_owner_write, commit_owner_write};
use super::chat::ChatMessage;
use super::chat_tools::assistant;
use crate::core::authorization::{Actor, ActorContext};
use crate::error::{Result, TinaError};

/// Entrada do catálogo de ferramentas reais, com possibilidade de delegação.
#[derive(Debug, Serialize)]
pub struct ToolEntry {
    key: &'static str,
    app: &'static str,
    label: &'static str,
    assignable: bool,
}

/// Catálogo derivado dos handlers internos, sem comandos arbitrários.
pub fn catalog() -> Vec<ToolEntry> {
    [
        ("create_position", "rh", "Propor criação de cargo", true),
        ("create_organization", "organization", "Propor organização", false),
        ("create_department", "organization", "Propor departamento", false),
        ("assign_tool", "administration", "Atribuir ferramenta", false),
        ("revoke_tool", "administration", "Revogar ferramenta", false),
        ("assign_app", "administration", "Atribuir aplicativo", false),
        ("revoke_app", "administration", "Revogar aplicativo", false),
    ]
    .into_iter()
    .map(|(key, app, label, assignable)| ToolEntry { key, app, label, assignable })
    .collect()
}

pub(super) fn is_management(action: &str) -> bool {
    matches!(action, "assign_tool" | "revoke_tool" | "assign_app" | "revoke_app")
}

fn assigning(action: &str) -> bool {
    matches!(action, "assign_tool" | "assign_app")
}

fn expand(action: &str, key: &str) -> Result<Vec<String>> {
    if !is_management(action) {
        return Err(TinaError::Validation("operação de acesso inválida".into()));
    }
    let app = matches!(action, "assign_app" | "revoke_app");
    let keys: Vec<String> = catalog().into_iter()
        .filter(|t| t.assignable && if app { t.app == key } else { t.key == key })
        .map(|t| t.key.to_owned()).collect();
    if keys.is_empty() {
        return Err(TinaError::PermissionDenied("ferramenta ou aplicativo não delegável".into()));
    }
    Ok(keys)
}

pub(super) fn definition(action: &str) -> Value {
    json!({
        "type":"function",
        "function":{
            "name":action,
            "description":"Propõe atribuir ou revogar ferramenta individual ou aplicativo de um funcionário. Somente ferramentas delegáveis reais. Requer aprovação do proprietário.",
            "parameters":{
                "type":"object","additionalProperties":false,
                "properties":{
                    "company_name":{"type":"string"},
                    "department_name":{"type":"string"},
                    "employee_name":{"type":"string"},
                    "key":{"type":"string","description":"Para app: rh. Para ferramenta: create_position."},
                    "reason":{"type":"string","maxLength":500}
                },
                "required":["company_name","department_name","employee_name","key","reason"]
            }
        }
    })
}

/// Concessão registrada e vinculada ao escopo original.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ToolGrant {
    employee_id: String,
    company_id: String,
    department_id: String,
    tool_key: String,
}

pub(crate) async fn list(pool: &SqlitePool, installation: &str) -> Result<Vec<ToolGrant>> {
    Ok(sqlx::query_as("SELECT g.employee_id,g.company_id,g.department_id,g.tool_key FROM employee_tool_grants g JOIN employees e ON e.id=g.employee_id JOIN positions p ON p.id=e.position_id JOIN departments d ON d.id=p.department_id JOIN companies c ON c.id=d.company_id WHERE g.installation_id=? AND c.installation_id=g.installation_id AND c.id=g.company_id AND d.id=g.department_id AND e.status='active' AND p.status='active' AND d.status='active' AND c.status='active' ORDER BY g.employee_id,g.tool_key")
        .bind(installation).fetch_all(pool).await?)
}

#[derive(Debug, sqlx::FromRow)]
struct Target {
    employee_id: String,
    employee_name: String,
    company_id: String,
    company_name: String,
    organization_kind: String,
    department_id: String,
    department_name: String,
    active: bool,
}

const TARGETS: &str = "SELECT e.id AS employee_id,e.name AS employee_name,c.id AS company_id,c.name AS company_name,c.kind AS organization_kind,d.id AS department_id,d.name AS department_name,(e.status='active' AND p.status='active' AND d.status='active' AND c.status='active') AS active FROM employees e JOIN positions p ON p.id=e.position_id JOIN departments d ON d.id=p.department_id JOIN companies c ON c.id=d.company_id WHERE c.installation_id=?";

async fn target(tx: &mut Transaction<'_, Sqlite>, installation: &str, id: &str) -> Result<Target> {
    let query = format!("{TARGETS} AND e.id=?");
    sqlx::query_as(&query).bind(installation).bind(id).fetch_optional(&mut **tx).await?
        .ok_or_else(|| TinaError::Unauthorized("funcionário fora da instalação".into()))
}

async fn apply(
    tx: &mut Transaction<'_, Sqlite>,
    installation: &str,
    owner: &str,
    t: &Target,
    action: &str,
    keys: &[String],
    approved: bool,
) -> Result<()> {
    if assigning(action) && approved && !t.active {
        return Err(TinaError::Validation("funcionário ou estrutura inativa".into()));
    }
    if keys.is_empty() || !is_management(action) {
        return Err(TinaError::Validation("concessão inválida".into()));
    }
    for key in keys {
        if !catalog().iter().any(|entry| entry.assignable && entry.key == key.as_str()) {
            return Err(TinaError::PermissionDenied("ferramenta não delegável".into()));
        }
        if approved {
            if assigning(action) {
                sqlx::query("INSERT INTO employee_tool_grants(installation_id,employee_id,company_id,department_id,tool_key) VALUES (?,?,?,?,?) ON CONFLICT(installation_id,employee_id,tool_key) DO UPDATE SET company_id=excluded.company_id,department_id=excluded.department_id,granted_at=datetime('now')")
                    .bind(installation).bind(&t.employee_id).bind(&t.company_id).bind(&t.department_id).bind(key)
                    .execute(&mut **tx).await?;
            } else {
                sqlx::query("DELETE FROM employee_tool_grants WHERE installation_id=? AND employee_id=? AND tool_key=?")
                    .bind(installation).bind(&t.employee_id).bind(key).execute(&mut **tx).await?;
            }
        }
        sqlx::query("INSERT INTO organizational_audit_events(id,installation_id,company_id,entity_type,entity_id,action,outcome,actor_type,actor_id,metadata_json) VALUES (?,?,?,'tool',?,?,?,'owner',?,?)")
            .bind(Uuid::new_v4().to_string()).bind(installation).bind(&t.company_id)
            .bind(&t.employee_id).bind(action).bind(if approved {"success"} else {"denied"}).bind(owner)
            .bind(json!({"tool":key,"department_id":t.department_id}).to_string()).execute(&mut **tx).await?;
    }
    Ok(())
}

pub(crate) async fn change(pool: &SqlitePool, installation: &str, owner: &str, employee: &str, action: &str, key: &str) -> Result<()> {
    let keys = expand(action, key)?;
    let mut tx = begin_owner_write(pool, installation, owner).await?;
    let t = target(&mut tx, installation, employee).await?;
    apply(&mut tx, installation, owner, &t, action, &keys, true).await?;
    commit_owner_write(tx, installation).await
}

/// Verificador de concessão para o futuro dispatcher de funcionários.
/// O contexto deve vir da autenticação interna, não de argumentos do modelo.
/// Não concede autorização de escrita; o executor deve aplicar sua própria política.
#[allow(dead_code)]
pub(crate) async fn check_employee_tool(
    pool: &SqlitePool,
    context: &ActorContext,
    company: &str,
    department: &str,
    key: &str,
) -> Result<()> {
    let Actor::Employee { employee_id, company_id } = context.actor() else {
        return Err(TinaError::PermissionDenied("contexto de funcionário necessário".into()));
    };
    if company_id != company || !catalog().iter().any(|t| t.assignable && t.key == key) {
        return Err(TinaError::PermissionDenied("escopo ou ferramenta não concedida".into()));
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM employee_tool_grants g JOIN employees e ON e.id=g.employee_id JOIN positions p ON p.id=e.position_id JOIN departments d ON d.id=p.department_id JOIN companies c ON c.id=d.company_id WHERE g.installation_id=? AND g.employee_id=? AND g.company_id=? AND g.department_id=? AND g.tool_key=? AND c.installation_id=g.installation_id AND c.id=g.company_id AND d.id=g.department_id AND e.status='active' AND p.status='active' AND d.status='active' AND c.status='active'")
        .bind(context.installation_id()).bind(employee_id).bind(company).bind(department).bind(key)
        .fetch_one(pool).await?;
    if count != 1 {
        return Err(TinaError::PermissionDenied("ferramenta não concedida neste escopo".into()));
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    company_name: String,
    department_name: String,
    employee_name: String,
    key: String,
    reason: String,
}

pub(super) async fn propose(
    tx: &mut Transaction<'_, Sqlite>, installation: &str, session: &str,
    user: &str, action: &str, input: &str,
) -> Result<ChatMessage> {
    if input.len() > 16_384 {
        return Err(TinaError::Validation("argumentos extensos".into()));
    }
    let a: Args = serde_json::from_str(input).map_err(|_| TinaError::Validation("argumentos de acesso inválidos".into()))?;
    if [&a.company_name,&a.department_name,&a.employee_name,&a.key,&a.reason].iter().any(|s| s.trim().is_empty() || s.chars().count()>500 || s.chars().any(char::is_control)) {
        return Err(TinaError::Validation("informe funcionário, escopo e justificativa".into()));
    }
    let keys = expand(action, &a.key)?;
    let rows: Vec<Target> = sqlx::query_as(TARGETS).bind(installation).fetch_all(&mut **tx).await?;
    let mut matches = rows.iter().filter(|t| t.company_name.to_lowercase()==a.company_name.trim().to_lowercase() && t.department_name.to_lowercase()==a.department_name.trim().to_lowercase() && t.employee_name.to_lowercase()==a.employee_name.trim().to_lowercase());
    let found = matches.next();
    if found.is_none() || matches.next().is_some() {
        return assistant(tx,installation,session,"Não encontrei um funcionário inequívoco nesse escopo. Precisamos identificar o funcionário antes de alterar ferramentas.").await;
    }
    let t = found.unwrap();
    if assigning(action) && !t.active {
        return Err(TinaError::Validation("funcionário ou estrutura inativa".into()));
    }
    let response = assistant(tx,installation,session,"Preparei uma alteração de ferramentas do funcionário. Confira o escopo, as ferramentas e o motivo antes de autorizar.").await?;
    sqlx::query("INSERT INTO chat_tool_access_proposals(id,installation_id,session_id,user_message_id,proposal_message_id,employee_id,employee_name,company_id,company_name,organization_kind,department_id,department_name,action,tool_keys,reason) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(Uuid::new_v4().to_string()).bind(installation).bind(session).bind(user).bind(&response.id)
        .bind(&t.employee_id).bind(&t.employee_name).bind(&t.company_id).bind(&t.company_name).bind(&t.organization_kind)
        .bind(&t.department_id).bind(&t.department_name).bind(action).bind(json!(keys).to_string()).bind(a.reason.trim())
        .execute(&mut **tx).await?;
    Ok(response)
}

#[derive(sqlx::FromRow)]
struct Proposal {
    employee_id: String,
    employee_name: String,
    company_id: String,
    company_name: String,
    department_id: String,
    department_name: String,
    action: String,
    tool_keys: String,
}

pub(super) async fn decide(
    mut tx: Transaction<'_, Sqlite>, installation: &str, owner: &str,
    session: &str, proposal: &str, approved: bool,
) -> Result<ChatMessage> {
    let p: Proposal = sqlx::query_as("SELECT employee_id,employee_name,company_id,company_name,department_id,department_name,action,tool_keys FROM chat_tool_access_proposals WHERE id=? AND installation_id=? AND session_id=? AND status='pending'")
        .bind(proposal).bind(installation).bind(session).fetch_one(&mut *tx).await?;
    let t = target(&mut tx,installation,&p.employee_id).await?;
    if t.company_id != p.company_id || t.department_id != p.department_id || t.employee_name != p.employee_name || t.company_name != p.company_name || t.department_name != p.department_name {
        return Err(TinaError::Validation("escopo do funcionário mudou; refaça a proposta".into()));
    }
    let keys: Vec<String> = serde_json::from_str(&p.tool_keys).map_err(|_| TinaError::Validation("proposta inválida".into()))?;
    apply(&mut tx,installation,owner,&t,&p.action,&keys,approved).await?;
    let content = if approved {
        format!("Ferramentas {} para {}: {}. A configuração está registrada; o executor de agentes ainda não está integrado.", if assigning(&p.action) {"atribuídas"} else {"revogadas"}, t.employee_name, keys.join(", "))
    } else {
        "Alteração recusada. Nenhum acesso foi modificado.".to_owned()
    };
    let response = assistant(&mut tx,installation,session,&content).await?;
    sqlx::query("UPDATE chat_tool_access_proposals SET status=?,result_message_id=? WHERE id=? AND installation_id=? AND status='pending'")
        .bind(if approved {"approved"} else {"rejected"}).bind(&response.id).bind(proposal).bind(installation).execute(&mut *tx).await?;
    commit_owner_write(tx,installation).await?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packages_do_not_grant_administration() {
        assert_eq!(expand("assign_app", "rh").unwrap(), vec!["create_position"]);
        assert!(expand("assign_app", "administration").is_err());
        assert!(expand("assign_tool", "assign_tool").is_err());
        assert!(expand("assign_tool", "hire_employee").is_err());
    }

    #[tokio::test]
    async fn grants_are_scoped_and_revocable() {
        let path = std::env::temp_dir().join(format!("tina-grants-{}.db", Uuid::new_v4()));
        let app = crate::TinaApp::initialize_at(&path,"grant-install","grant-owner").await.unwrap();
        app.unlock_owner_for_test().unwrap();
        let company = app.owner_create_company("Empresa").await.unwrap();
        let dept = app.owner_create_department(&company.id,"RH","RH",None).await.unwrap();
        let position = app.owner_create_position(&dept.id,"Analista",1,0,None).await.unwrap();
        let employee = app.owner_create_employee(&position.id,"Agente RH",None,None).await.unwrap();
        let ctx = ActorContext::verified("grant-install".into(),Actor::Employee { employee_id:employee.id.clone(),company_id:company.id.clone() });
        let pool = app.database().pool();
        assert!(check_employee_tool(pool,&ctx,&company.id,&dept.id,"create_position").await.is_err());
        app.owner_change_tool_access(&employee.id,"assign_app","rh").await.unwrap();
        assert!(check_employee_tool(pool,&ctx,&company.id,&dept.id,"create_position").await.is_ok());
        assert!(check_employee_tool(pool,&ctx,&company.id,"other","create_position").await.is_err());
        assert!(check_employee_tool(pool,&ctx,&company.id,&dept.id,"assign_tool").await.is_err());
        app.owner_change_tool_access(&employee.id,"revoke_tool","create_position").await.unwrap();
        assert!(check_employee_tool(pool,&ctx,&company.id,&dept.id,"create_position").await.is_err());
        pool.close().await;
        let _ = std::fs::remove_file(path);
    }
}
