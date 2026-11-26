//! Primeira ferramenta administrativa: proposta e aprovação persistidas no SQLite.

use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Sqlite, SqlitePool, Transaction};
use uuid::Uuid;

use crate::error::{Result, TinaError};
use super::audit::{begin_owner_write, commit_owner_write};
use super::chat::ChatMessage;

pub(crate) fn definition() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "create_department",
            "description": "Propõe criar um departamento explicitamente solicitado pelo proprietário. A organização deve existir; a escrita só ocorre após confirmação na interface. Não use para sugestões.",
            "parameters": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "organization_kind": {"type":"string","enum":["personal","business"]},
                    "company_name": {"type":"string","description":"Nome exato da organização empresarial; opcional para pessoal."},
                    "name": {"type":"string","minLength":1,"maxLength":120},
                    "code": {"type":"string","description":"Código curto, letras ASCII, números, hífen ou underscore."},
                    "description": {"type":"string","maxLength":2000}
                },
                "required": ["organization_kind","name","code"]
            }
        }
    })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    organization_kind: String,
    company_name: Option<String>,
    name: String,
    code: String,
    description: Option<String>,
}

fn parse(name: &str, input: &str) -> Result<Args> {
    if name != "create_department" || input.len() > 16_384 {
        return Err(TinaError::ToolError("ferramenta desconhecida ou argumentos extensos".into()));
    }
    let mut args: Args = serde_json::from_str(input)
        .map_err(|_| TinaError::Validation("argumentos de departamento inválidos".into()))?;
    args.name = args.name.trim().to_owned();
    args.code = args.code.trim().to_ascii_uppercase();
    args.company_name = args.company_name.map(|v| v.trim().to_owned());
    args.description = args.description.map(|v| v.trim().to_owned());
    if !matches!(args.organization_kind.as_str(), "personal" | "business")
        || args.name.is_empty() || args.name.chars().count() > 120
        || args.name.chars().any(char::is_control)
        || args.code.is_empty() || args.code.len() > 40
        || !args.code.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        || args.description.as_ref().is_some_and(|s| s.chars().count() > 2000)
        || args.company_name.as_ref().is_some_and(|s| s.is_empty() || s.chars().count() > 160 || s.chars().any(char::is_control))
        || (args.organization_kind == "business" && args.company_name.is_none())
    {
        return Err(TinaError::Validation("informe nome, código e organização válidos".into()));
    }
    Ok(args)
}

/// Proposta visível e verificável pelo proprietário antes de autorizar a escrita.
#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct DepartmentProposal {
    id: String,
    company_id: String,
    company_name: String,
    organization_kind: String,
    name: String,
    code: String,
    description: Option<String>,
}

async fn assistant(
    tx: &mut Transaction<'_, Sqlite>,
    installation: &str,
    session: &str,
    content: &str,
) -> Result<ChatMessage> {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO chat_messages(id, session_id, installation_id, role, content) VALUES (?, ?, ?, 'assistant', ?)")
        .bind(&id).bind(session).bind(installation).bind(content).execute(&mut **tx).await?;
    sqlx::query("UPDATE chat_sessions SET updated_at = datetime('now') WHERE id = ? AND installation_id = ?")
        .bind(session).bind(installation).execute(&mut **tx).await?;
    Ok(sqlx::query_as("SELECT id, role, content, created_at FROM chat_messages WHERE id = ?")
        .bind(id).fetch_one(&mut **tx).await?)
}

pub(crate) async fn propose(
    pool: &SqlitePool,
    installation: &str,
    session: &str,
    user: &str,
    name: &str,
    arguments: &str,
) -> Result<ChatMessage> {
    let args = parse(name, arguments)?;
    let mut tx = pool.begin().await?;
    let latest: Option<String> = sqlx::query_scalar(
        "SELECT id FROM chat_messages WHERE installation_id = ? AND session_id = ? AND role = 'user' AND id = (SELECT id FROM chat_messages WHERE installation_id = ? AND session_id = ? ORDER BY created_at DESC, rowid DESC LIMIT 1)"
    ).bind(installation).bind(session).bind(installation).bind(session).fetch_optional(&mut *tx).await?;
    if latest.as_deref() != Some(user) {
        return Err(TinaError::Unauthorized("mensagem de origem indisponível".into()));
    }
    let organizations: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, name FROM companies WHERE installation_id = ? AND kind = ? AND status = 'active'"
    ).bind(installation).bind(&args.organization_kind).fetch_all(&mut *tx).await?;
    let mut matching = organizations.iter().filter(|(_, name)| {
        args.company_name.as_ref().is_none_or(|requested| requested.to_lowercase() == name.to_lowercase())
    });
    let organization = matching.next();
    if organization.is_none() || matching.next().is_some() {
        let message = if args.organization_kind == "personal" {
            "Não encontrei uma organização pessoal ativa correspondente. Cadastre sua organização Pessoal na área Organizações antes de criar o departamento."
        } else {
            "Não encontrei uma organização empresarial ativa e inequívoca com esse nome. Confira o cadastro na área Organizações e informe o nome exato."
        };
        let response = assistant(&mut tx, installation, session, message).await?;
        tx.commit().await?;
        return Ok(response);
    }
    let (company_id, company_name) = organization.unwrap();
    let response = assistant(&mut tx, installation, session,
        "Preparei uma proposta de criação de departamento. Revise os dados abaixo e confirme ou recuse. Ainda não criei o departamento.").await?;
    sqlx::query("INSERT INTO chat_department_proposals(id,installation_id,session_id,user_message_id,proposal_message_id,company_id,company_name,organization_kind,name,code,description) VALUES (?,?,?,?,?,?,?,?,?,?,?)")
        .bind(Uuid::new_v4().to_string()).bind(installation).bind(session).bind(user)
        .bind(&response.id).bind(company_id).bind(company_name).bind(&args.organization_kind)
        .bind(&args.name).bind(&args.code).bind(&args.description).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(response)
}

const PENDING: &str = "SELECT p.id,p.company_id,p.company_name,p.organization_kind,p.name,p.code,p.description FROM chat_department_proposals p WHERE p.installation_id = ? AND p.session_id = ? AND p.status = 'pending' AND p.proposal_message_id = (SELECT m.id FROM chat_messages m WHERE m.installation_id = p.installation_id AND m.session_id = p.session_id ORDER BY m.created_at DESC,m.rowid DESC LIMIT 1)";

pub(crate) async fn pending(pool: &SqlitePool, installation: &str, session: &str) -> Result<Option<DepartmentProposal>> {
    Ok(sqlx::query_as(PENDING).bind(installation).bind(session).fetch_optional(pool).await?)
}

pub(crate) async fn decide(
    pool: &SqlitePool,
    installation: &str,
    owner: &str,
    session: &str,
    proposal: &str,
    approved: bool,
) -> Result<ChatMessage> {
    let mut tx = begin_owner_write(pool, installation, owner).await?;
    let previous: Option<(String, String)> = sqlx::query_as(
        "SELECT status,result_message_id FROM chat_department_proposals WHERE id = ? AND installation_id = ? AND session_id = ? AND result_message_id IS NOT NULL"
    ).bind(proposal).bind(installation).bind(session).fetch_optional(&mut *tx).await?;
    if let Some((status, message_id)) = previous {
        if (status == "approved") != approved {
            return Err(TinaError::Validation("proposta já decidida".into()));
        }
        let message = sqlx::query_as("SELECT id,role,content,created_at FROM chat_messages WHERE id = ? AND installation_id = ? AND session_id = ?")
            .bind(message_id).bind(installation).bind(session).fetch_one(&mut *tx).await?;
        commit_owner_write(tx, installation).await?;
        return Ok(message);
    }
    let p: DepartmentProposal = sqlx::query_as(PENDING).bind(installation).bind(session)
        .fetch_optional(&mut *tx).await?.filter(|p: &DepartmentProposal| p.id == proposal)
        .ok_or_else(|| TinaError::Unauthorized("proposta inexistente ou substituída".into()))?;
    let status;
    let content;
    let entity_id;
    if approved {
        entity_id = Uuid::new_v4().to_string();
        let result = sqlx::query(
            "INSERT INTO departments(id, company_id, code, name, description, status) SELECT ?,c.id,?,?,?,'active' FROM companies c WHERE c.id = ? AND c.installation_id = ? AND c.status = 'active' AND c.kind = ? AND c.name = ?"
        ).bind(&entity_id).bind(&p.code).bind(&p.name).bind(&p.description)
        .bind(&p.company_id).bind(installation).bind(&p.organization_kind).bind(&p.company_name)
        .execute(&mut *tx).await?;
        if result.rows_affected() != 1 {
            return Err(TinaError::Validation("organização alterada ou indisponível; refaça o pedido".into()));
        }
        status = "approved";
        content = format!("Departamento criado com sucesso: {} ({}), na organização {}. Consulte a área Organizações.", p.name, p.code, p.company_name);
    } else {
        entity_id = p.id.clone();
        status = "rejected";
        content = "Criação de departamento recusada. Nenhum departamento foi criado por esta proposta.".to_string();
    }
    sqlx::query("INSERT INTO organizational_audit_events(id,installation_id,company_id,entity_type,entity_id,action,outcome,actor_type,actor_id,metadata_json) VALUES (?,?,?,'tool',?,'create_department',?,'owner',?,?)")
        .bind(Uuid::new_v4().to_string()).bind(installation).bind(&p.company_id).bind(entity_id)
        .bind(if approved {"success"} else {"denied"}).bind(owner)
        .bind(json!({"proposal_id":p.id,"session_id":session}).to_string()).execute(&mut *tx).await?;
    let response = assistant(&mut tx, installation, session, &content).await?;
    sqlx::query("UPDATE chat_department_proposals SET status = ?,result_message_id = ? WHERE id = ? AND installation_id = ? AND status = 'pending'")
        .bind(status).bind(&response.id).bind(proposal).bind(installation).execute(&mut *tx).await?;
    commit_owner_write(tx, installation).await?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_tools_and_forged_authority() {
        let args = r#"{"organization_kind":"personal","name":"Finanças","code":"fin"}"#;
        assert_eq!(parse("create_department", args).unwrap().code, "FIN");
        assert!(parse("shell", args).is_err());
        assert!(parse("create_department", r#"{"organization_kind":"personal","name":"Finanças","code":"fin","owner_id":"fake"}"#).is_err());
        assert!(parse("create_department", r#"{"organization_kind":"business","name":"Finanças","code":"fin"}"#).is_err());
    }

    #[tokio::test]
    async fn approval_is_persistent_scoped_and_idempotent() {
        let path = std::env::temp_dir().join(format!("tina-tool-{}.db", Uuid::new_v4()));
        let app = crate::TinaApp::initialize_at(&path, "tool-install", "tool-owner").await.unwrap();
        app.unlock_owner_for_test().unwrap();
        let company = app.owner_create_company_with_kind("Pessoal", "personal").await.unwrap();
        let session = app.owner_create_chat("Ferramenta").await.unwrap();
        let user = app.owner_append_chat_message(&session.id, "Crie Finanças pessoais").await.unwrap();
        propose(app.database().pool(), "tool-install", &session.id, &user.id, "create_department",
            r#"{"organization_kind":"personal","name":"Finanças","code":"FIN"}"#).await.unwrap();
        let proposal = app.owner_pending_department(&session.id).await.unwrap().unwrap();
        assert!(app.database().departments().list_by_company(&company.id).await.unwrap().is_empty());
        let first = app.owner_decide_department(&session.id, &proposal.id, true).await.unwrap();
        let again = app.owner_decide_department(&session.id, &proposal.id, true).await.unwrap();
        assert_eq!(first.id, again.id);
        assert_eq!(app.database().departments().list_by_company(&company.id).await.unwrap().len(), 1);
        assert!(app.owner_pending_department(&session.id).await.unwrap().is_none());
        assert!(app.owner_decide_department("other-session", &proposal.id, true).await.is_err());
        app.database().pool().close().await;
        let _ = std::fs::remove_file(path);
    }
}
