//! Ferramentas administrativas do chat: proposta persistida e aprovação local.
//!
//! Nenhuma escrita organizacional acontece na inferência. A Tina registra uma
//! proposta com justificativa e aguarda confirmação do proprietário na interface.

use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Sqlite, SqlitePool, Transaction};
use uuid::Uuid;

use super::audit::{begin_owner_write, commit_owner_write};
use super::chat::ChatMessage;
use crate::error::{Result, TinaError};

pub(crate) fn definitions() -> Value {
    json!([
        department_definition(),
        organization_definition(),
        super::hr_tools::position_definition(),
        super::hr_contracts::definition(),
        super::tool_access::definition("assign_tool"),
        super::tool_access::definition("revoke_tool"),
        super::tool_access::definition("assign_app"),
        super::tool_access::definition("revoke_app")
    ])
}

fn department_definition() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "create_department",
            "description": "Propõe criar um departamento quando o proprietário pedir ou quando a conversa revelar uma necessidade concreta ainda não atendida. Explique por quê. A escrita só ocorre após confirmação na interface.",
            "parameters": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "organization_kind": {"type":"string","enum":["personal","business"]},
                    "company_name": {"type":"string","description":"Nome exato da organização empresarial; opcional para pessoal."},
                    "name": {"type":"string","minLength":1,"maxLength":120},
                    "code": {"type":"string","description":"Código curto, letras ASCII, números, hífen ou underscore."},
                    "description": {"type":"string","maxLength":2000},
                    "reason": {"type":"string","description":"Benefício concreto identificado na conversa, sem inventar fatos.","maxLength":500}
                },
                "required": ["organization_kind","name","code","reason"]
            }
        }
    })
}

fn organization_definition() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "create_organization",
            "description": "Propõe uma organização nova por pedido ou iniciativa da Tina, quando houver necessidade concreta. A organização pessoal reúne a vida do proprietário; empresas reais usam business. A escrita exige confirmação na interface.",
            "parameters": {
                "type":"object",
                "additionalProperties":false,
                "properties":{
                    "organization_kind":{"type":"string","enum":["personal","business"]},
                    "name":{"type":"string","minLength":1,"maxLength":160},
                    "reason":{"type":"string","minLength":1,"maxLength":500}
                },
                "required":["organization_kind","name","reason"]
            }
        }
    })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OrganizationArgs {
    organization_kind: String,
    name: String,
    reason: String,
}

fn parse_organization(input: &str) -> Result<OrganizationArgs> {
    if input.len() > 16_384 {
        return Err(TinaError::Validation("argumentos extensos".into()));
    }
    let mut args: OrganizationArgs = serde_json::from_str(input)
        .map_err(|_| TinaError::Validation("argumentos de organização inválidos".into()))?;
    args.name = args.name.trim().to_owned();
    args.reason = args.reason.trim().to_owned();
    if !matches!(args.organization_kind.as_str(), "personal" | "business")
        || args.name.is_empty()
        || args.name.chars().count() > 160
        || args.name.chars().any(char::is_control)
        || args.reason.is_empty()
        || args.reason.chars().count() > 500
        || args.reason.chars().any(char::is_control)
    {
        return Err(TinaError::Validation(
            "nome, tipo ou justificativa inválidos".into(),
        ));
    }
    Ok(args)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    organization_kind: String,
    company_name: Option<String>,
    name: String,
    code: String,
    description: Option<String>,
    reason: Option<String>,
}

fn parse(name: &str, input: &str) -> Result<Args> {
    if name != "create_department" || input.len() > 16_384 {
        return Err(TinaError::ToolError(
            "ferramenta desconhecida ou argumentos extensos".into(),
        ));
    }
    let mut args: Args = serde_json::from_str(input)
        .map_err(|_| TinaError::Validation("argumentos de departamento inválidos".into()))?;
    args.name = args.name.trim().to_owned();
    args.code = args.code.trim().to_ascii_uppercase();
    args.company_name = args.company_name.map(|v| v.trim().to_owned());
    args.description = args.description.map(|v| v.trim().to_owned());
    args.reason = args.reason.map(|v| v.trim().to_owned());
    if !matches!(args.organization_kind.as_str(), "personal" | "business")
        || args.name.is_empty()
        || args.name.chars().count() > 120
        || args.name.chars().any(char::is_control)
        || args.code.is_empty()
        || args.code.len() > 40
        || !args
            .code
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        || args
            .description
            .as_ref()
            .is_some_and(|s| s.chars().count() > 2000)
        || args.company_name.as_ref().is_some_and(|s| {
            s.is_empty() || s.chars().count() > 160 || s.chars().any(char::is_control)
        })
        || args.reason.as_ref().is_some_and(|s| {
            s.is_empty() || s.chars().count() > 500 || s.chars().any(char::is_control)
        })
        || (args.organization_kind == "business" && args.company_name.is_none())
    {
        return Err(TinaError::Validation(
            "informe nome, código e organização válidos".into(),
        ));
    }
    Ok(args)
}

/// Proposta visível e verificável pelo proprietário antes de autorizar a escrita.
#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct DepartmentProposal {
    pub(crate) id: String,
    pub(crate) tool_name: String,
    pub(crate) lifecycle_status: String,
    pub(crate) reason: Option<String>,
    pub(crate) company_id: String,
    pub(crate) company_name: String,
    pub(crate) organization_kind: String,
    pub(crate) name: String,
    pub(crate) code: String,
    pub(crate) description: Option<String>,
}

pub(super) async fn assistant(
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
    Ok(
        sqlx::query_as("SELECT id, role, content, created_at FROM chat_messages WHERE id = ?")
            .bind(id)
            .fetch_one(&mut **tx)
            .await?,
    )
}

/// A proposta ativa sobrevive a mensagens posteriores: só deixa de ser ativa
/// quando o proprietário decide ou quando uma nova proposta a substitui.
const PENDING: &str = "SELECT id,tool_name,lifecycle_status,reason,company_id,company_name,organization_kind,name,code,description FROM ( \
    SELECT p.id AS id,'create_department' AS tool_name,p.lifecycle_status,p.reason,p.company_id,p.company_name,p.organization_kind,p.name,p.code,p.description,m.rowid AS ord \
      FROM chat_department_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id \
     WHERE p.installation_id = ? AND p.session_id = ? AND p.lifecycle_status = 'pending' AND p.result_message_id IS NULL \
    UNION ALL \
    SELECT p.id,'create_organization',p.lifecycle_status,p.reason,'',p.name,p.organization_kind,p.name,'',NULL,m.rowid \
      FROM chat_organization_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id \
     WHERE p.installation_id = ? AND p.session_id = ? AND p.lifecycle_status = 'pending' AND p.result_message_id IS NULL \
    UNION ALL \
    SELECT p.id,'create_position',p.lifecycle_status,p.reason,p.company_id,p.company_name,p.organization_kind,p.name,'',p.description,m.rowid \
      FROM chat_position_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id \
     WHERE p.installation_id = ? AND p.session_id = ? AND p.lifecycle_status = 'pending' AND p.result_message_id IS NULL \
    UNION ALL \
    SELECT p.id,p.action,p.lifecycle_status,p.reason,p.company_id,p.company_name,p.organization_kind,p.employee_name,'',('Departamento: ' || p.department_name || char(10) || 'Ferramentas: ' || p.tool_keys),m.rowid \
      FROM chat_tool_access_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id \
     WHERE p.installation_id = ? AND p.session_id = ? AND p.lifecycle_status = 'pending' AND p.result_message_id IS NULL \
    UNION ALL \
    SELECT p.id,'generate_contract',p.lifecycle_status,p.reason,p.company_id,p.company_name,p.organization_kind,p.name,'',p.description,m.rowid \
      FROM chat_contract_drafts p JOIN chat_messages m ON m.id = p.proposal_message_id \
     WHERE p.installation_id = ? AND p.session_id = ? AND p.lifecycle_status = 'pending' AND p.result_message_id IS NULL \
    ORDER BY ord DESC LIMIT 1)";

const PENDING_BY_ID: &str = "SELECT id,tool_name,lifecycle_status,reason,company_id,company_name,organization_kind,name,code,description FROM ( \
    SELECT p.id AS id,'create_department' AS tool_name,p.lifecycle_status,p.reason,p.company_id,p.company_name,p.organization_kind,p.name,p.code,p.description,m.rowid AS ord \
      FROM chat_department_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id \
     WHERE p.installation_id = ? AND p.session_id = ? AND p.id = ? AND p.lifecycle_status = 'pending' AND p.result_message_id IS NULL \
    UNION ALL \
    SELECT p.id,'create_organization',p.lifecycle_status,p.reason,'',p.name,p.organization_kind,p.name,'',NULL,m.rowid \
      FROM chat_organization_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id \
     WHERE p.installation_id = ? AND p.session_id = ? AND p.id = ? AND p.lifecycle_status = 'pending' AND p.result_message_id IS NULL \
    UNION ALL \
    SELECT p.id,'create_position',p.lifecycle_status,p.reason,p.company_id,p.company_name,p.organization_kind,p.name,'',p.description,m.rowid \
      FROM chat_position_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id \
     WHERE p.installation_id = ? AND p.session_id = ? AND p.id = ? AND p.lifecycle_status = 'pending' AND p.result_message_id IS NULL \
    UNION ALL \
    SELECT p.id,p.action,p.lifecycle_status,p.reason,p.company_id,p.company_name,p.organization_kind,p.employee_name,'',('Departamento: ' || p.department_name || char(10) || 'Ferramentas: ' || p.tool_keys),m.rowid \
      FROM chat_tool_access_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id \
     WHERE p.installation_id = ? AND p.session_id = ? AND p.id = ? AND p.lifecycle_status = 'pending' AND p.result_message_id IS NULL \
    UNION ALL \
    SELECT p.id,'generate_contract',p.lifecycle_status,p.reason,p.company_id,p.company_name,p.organization_kind,p.name,'',p.description,m.rowid \
      FROM chat_contract_drafts p JOIN chat_messages m ON m.id = p.proposal_message_id \
     WHERE p.installation_id = ? AND p.session_id = ? AND p.id = ? AND p.lifecycle_status = 'pending' AND p.result_message_id IS NULL \
    ORDER BY ord DESC LIMIT 1)";

async fn pending_in(
    tx: &mut Transaction<'_, Sqlite>,
    installation: &str,
    session: &str,
) -> Result<Option<DepartmentProposal>> {
    Ok(sqlx::query_as(PENDING)
        .bind(installation)
        .bind(session)
        .bind(installation)
        .bind(session)
        .bind(installation)
        .bind(session)
        .bind(installation)
        .bind(session)
        .bind(installation)
        .bind(session)
        .fetch_optional(&mut **tx)
        .await?)
}

pub(crate) async fn pending(
    pool: &SqlitePool,
    installation: &str,
    session: &str,
) -> Result<Option<DepartmentProposal>> {
    Ok(sqlx::query_as(PENDING)
        .bind(installation)
        .bind(session)
        .bind(installation)
        .bind(session)
        .bind(installation)
        .bind(session)
        .bind(installation)
        .bind(session)
        .bind(installation)
        .bind(session)
        .fetch_optional(pool)
        .await?)
}

async fn pending_by_id(
    tx: &mut Transaction<'_, Sqlite>,
    installation: &str,
    session: &str,
    proposal: &str,
) -> Result<Option<DepartmentProposal>> {
    Ok(sqlx::query_as(PENDING_BY_ID)
        .bind(installation)
        .bind(session)
        .bind(proposal)
        .bind(installation)
        .bind(session)
        .bind(proposal)
        .bind(installation)
        .bind(session)
        .bind(proposal)
        .bind(installation)
        .bind(session)
        .bind(proposal)
        .bind(installation)
        .bind(session)
        .bind(proposal)
        .fetch_optional(&mut **tx)
        .await?)
}

async fn proposal_state(
    tx: &mut Transaction<'_, Sqlite>,
    installation: &str,
    session: &str,
    proposal: &str,
) -> Result<Option<(String, Option<String>)>> {
    Ok(sqlx::query_as(
        "SELECT lifecycle_status,result_message_id FROM chat_department_proposals \
             WHERE id = ? AND installation_id = ? AND session_id = ? \
         UNION ALL \
         SELECT lifecycle_status,result_message_id FROM chat_organization_proposals \
             WHERE id = ? AND installation_id = ? AND session_id = ? \
         UNION ALL \
         SELECT lifecycle_status,result_message_id FROM chat_position_proposals \
             WHERE id = ? AND installation_id = ? AND session_id = ? \
         UNION ALL \
         SELECT lifecycle_status,result_message_id FROM chat_tool_access_proposals \
             WHERE id = ? AND installation_id = ? AND session_id = ? \
         UNION ALL \
         SELECT lifecycle_status,result_message_id FROM chat_contract_drafts \
             WHERE id = ? AND installation_id = ? AND session_id = ?",
    )
    .bind(proposal)
    .bind(installation)
    .bind(session)
    .bind(proposal)
    .bind(installation)
    .bind(session)
    .bind(proposal)
    .bind(installation)
    .bind(session)
    .bind(proposal)
    .bind(installation)
    .bind(session)
    .bind(proposal)
    .bind(installation)
    .bind(session)
    .fetch_optional(&mut **tx)
    .await?)
}

async fn result_message(
    tx: &mut Transaction<'_, Sqlite>,
    installation: &str,
    session: &str,
    message: &str,
) -> Result<ChatMessage> {
    Ok(sqlx::query_as(
        "SELECT id,role,content,created_at FROM chat_messages \
             WHERE id = ? AND installation_id = ? AND session_id = ?",
    )
    .bind(message)
    .bind(installation)
    .bind(session)
    .fetch_one(&mut **tx)
    .await?)
}

fn same_department(p: &DepartmentProposal, args: &Args, company_id: &str) -> bool {
    p.tool_name == "create_department"
        && p.company_id == company_id
        && p.organization_kind == args.organization_kind
        && p.name == args.name
        && p.code == args.code
        && p.description.as_deref() == args.description.as_deref()
        && p.reason.as_deref() == args.reason.as_deref()
}

fn same_organization(p: &DepartmentProposal, args: &OrganizationArgs) -> bool {
    p.tool_name == "create_organization"
        && p.organization_kind == args.organization_kind
        && p.name == args.name
        && p.reason.as_deref() == Some(args.reason.as_str())
}

pub(crate) async fn propose(
    pool: &SqlitePool,
    installation: &str,
    session: &str,
    user: &str,
    name: &str,
    arguments: &str,
) -> Result<ChatMessage> {
    let mut tx = pool.begin().await?;
    let latest: Option<String> = sqlx::query_scalar(
        "SELECT id FROM chat_messages WHERE installation_id = ? AND session_id = ? AND role = 'user' AND id = (SELECT id FROM chat_messages WHERE installation_id = ? AND session_id = ? ORDER BY created_at DESC, rowid DESC LIMIT 1)"
    ).bind(installation).bind(session).bind(installation).bind(session).fetch_optional(&mut *tx).await?;
    if latest.as_deref() != Some(user) {
        return Err(TinaError::Unauthorized(
            "mensagem de origem indisponível".into(),
        ));
    }
    if name == "create_organization" {
        let args = parse_organization(arguments)?;
        let response = propose_organization(&mut tx, installation, session, user, &args).await?;
        tx.commit().await?;
        return Ok(response);
    }
    if super::tool_access::is_management(name) {
        let response =
            super::tool_access::propose(&mut tx, installation, session, user, name, arguments)
                .await?;
        tx.commit().await?;
        return Ok(response);
    }
    if name == "generate_contract" {
        let response =
            super::hr_contracts::propose(&mut tx, installation, session, user, arguments).await?;
        tx.commit().await?;
        return Ok(response);
    }
    if name == "create_position" {
        let response =
            super::hr_tools::propose_position(&mut tx, installation, session, user, arguments)
                .await?;
        tx.commit().await?;
        return Ok(response);
    }
    let args = parse(name, arguments)?;
    let organizations: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, name FROM companies WHERE installation_id = ? AND kind = ? AND status = 'active'"
    ).bind(installation).bind(&args.organization_kind).fetch_all(&mut *tx).await?;
    let mut matching = organizations.iter().filter(|(_, name)| {
        args.company_name
            .as_ref()
            .is_none_or(|requested| requested.to_lowercase() == name.to_lowercase())
    });
    let organization = matching.next();
    if organization.is_none() || matching.next().is_some() {
        if args.organization_kind == "personal" && organizations.is_empty() {
            let personal = OrganizationArgs {
                organization_kind: "personal".into(),
                name: "Pessoal".into(),
                reason: format!(
                    "Organizar a vida pessoal e permitir a criação do departamento {}.",
                    args.name
                ),
            };
            let response =
                propose_organization(&mut tx, installation, session, user, &personal).await?;
            tx.commit().await?;
            return Ok(response);
        }
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
    let existing: Vec<(String, String)> =
        sqlx::query_as("SELECT name,code FROM departments WHERE company_id = ?")
            .bind(company_id)
            .fetch_all(&mut *tx)
            .await?;
    if existing.iter().any(|(name, code)| {
        name.to_lowercase() == args.name.to_lowercase() || code.eq_ignore_ascii_case(&args.code)
    }) {
        let response = assistant(&mut tx, installation, session,
            "Já existe um departamento com esse nome ou código nessa organização. Não vou propor uma duplicação; podemos verificar o cadastro existente.").await?;
        tx.commit().await?;
        return Ok(response);
    }
    if let Some(active) = pending_in(&mut tx, installation, session).await? {
        if same_department(&active, &args, company_id) {
            let response = assistant(&mut tx, installation, session,
                "Essa proposta já está pendente nesta conversa. Revise o cartão e confirme, recuse ou cancele antes de sugerir outra.").await?;
            tx.commit().await?;
            return Ok(response);
        }
    }
    let response = assistant(&mut tx, installation, session,
        "Identifiquei uma possibilidade de organização. Posso criar este departamento? Revise a proposta e confirme ou recuse. Ainda não criei nada.").await?;
    // O gatilho da migration 019 marca a proposta anterior como `superseded`.
    sqlx::query("INSERT INTO chat_department_proposals(id,installation_id,session_id,user_message_id,proposal_message_id,company_id,company_name,organization_kind,name,code,description,reason) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(Uuid::new_v4().to_string()).bind(installation).bind(session).bind(user)
        .bind(&response.id).bind(company_id).bind(company_name).bind(&args.organization_kind)
        .bind(&args.name).bind(&args.code).bind(&args.description).bind(&args.reason).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(response)
}

async fn propose_organization(
    tx: &mut Transaction<'_, Sqlite>,
    installation: &str,
    session: &str,
    user: &str,
    args: &OrganizationArgs,
) -> Result<ChatMessage> {
    let existing: Vec<(String, String)> =
        sqlx::query_as("SELECT name,kind FROM companies WHERE installation_id = ?")
            .bind(installation)
            .fetch_all(&mut **tx)
            .await?;
    if existing.iter().any(|(name, kind)| {
        name.to_lowercase() == args.name.to_lowercase()
            || (args.organization_kind == "personal" && kind == "personal")
    }) {
        return assistant(tx, installation, session,
            "Já existe uma organização com esse nome ou uma estrutura pessoal cadastrada. Podemos aproveitar a existente em vez de duplicá-la.").await;
    }
    if let Some(active) = pending_in(tx, installation, session).await? {
        if same_organization(&active, args) {
            return assistant(tx, installation, session,
                "Essa proposta de organização já está pendente nesta conversa. Revise o cartão e confirme, recuse ou cancele antes de sugerir outra.").await;
        }
    }
    let response = assistant(tx, installation, session,
        "Identifiquei uma oportunidade de organizar melhor sua rotina ou operação. Posso criar esta organização? Confira o motivo e os dados antes de autorizar.").await?;
    // O gatilho da migration 019 marca a proposta anterior como `superseded`.
    sqlx::query("INSERT INTO chat_organization_proposals(id,installation_id,session_id,user_message_id,proposal_message_id,name,organization_kind,reason) VALUES (?,?,?,?,?,?,?,?)")
        .bind(Uuid::new_v4().to_string()).bind(installation).bind(session).bind(user)
        .bind(&response.id).bind(&args.name).bind(&args.organization_kind).bind(&args.reason)
        .execute(&mut **tx).await?;
    Ok(response)
}

/// Valida o estado anterior e mantém as transições idempotentes. Erros aqui
/// fazem rollback da transação, inclusive da mensagem já inserida.
async fn ensure_decidable(
    tx: &mut Transaction<'_, Sqlite>,
    installation: &str,
    session: &str,
    proposal: &str,
    approved: bool,
) -> Result<bool> {
    let Some((lifecycle, message_id)) = proposal_state(tx, installation, session, proposal).await?
    else {
        return Ok(false);
    };
    match lifecycle.as_str() {
        "pending" if message_id.is_some() => Err(TinaError::Validation(
            "proposta pendente com resultado inconsistente".into(),
        )),
        "pending" => Ok(false),
        "approved" if approved => {
            let message = message_id.ok_or_else(|| {
                TinaError::Validation("aprovação anterior sem mensagem registrada".into())
            })?;
            let _ = result_message(tx, installation, session, &message).await?;
            Ok(true)
        }
        "rejected" if !approved => {
            let message = message_id.ok_or_else(|| {
                TinaError::Validation("recusa anterior sem mensagem registrada".into())
            })?;
            let _ = result_message(tx, installation, session, &message).await?;
            Ok(true)
        }
        "superseded" => Err(TinaError::Validation(
            "proposta substituída por uma versão mais recente".into(),
        )),
        "cancelled" => Err(TinaError::Validation(
            "proposta cancelada pelo proprietário".into(),
        )),
        "approved" => Err(TinaError::Validation(
            "proposta já foi aprovada; não é possível recusar".into(),
        )),
        "rejected" => Err(TinaError::Validation(
            "proposta já foi recusada; não é possível aprovar".into(),
        )),
        _ => Err(TinaError::Validation(
            "estado de proposta desconhecido".into(),
        )),
    }
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
    if ensure_decidable(&mut tx, installation, session, proposal, approved).await? {
        let (_, message_id) = proposal_state(&mut tx, installation, session, proposal)
            .await?
            .ok_or_else(|| TinaError::Validation("proposta inexistente".into()))?;
        let message = result_message(
            &mut tx,
            installation,
            session,
            &message_id.ok_or_else(|| TinaError::Validation("resultado ausente".into()))?,
        )
        .await?;
        commit_owner_write(tx, installation).await?;
        return Ok(message);
    }
    let p: DepartmentProposal = pending_by_id(&mut tx, installation, session, proposal)
        .await?
        .ok_or_else(|| TinaError::Unauthorized("proposta inexistente ou substituída".into()))?;
    if super::tool_access::is_management(&p.tool_name) {
        return super::tool_access::decide(tx, installation, owner, session, proposal, approved)
            .await;
    }
    if p.tool_name == "generate_contract" {
        return super::hr_contracts::decide(tx, installation, owner, session, proposal, approved)
            .await;
    }
    if p.tool_name == "create_position" {
        return super::hr_tools::decide_position(
            tx,
            installation,
            owner,
            session,
            proposal,
            approved,
        )
        .await;
    }
    if p.tool_name == "create_organization" {
        return decide_organization(tx, installation, owner, session, &p, approved).await;
    }
    let status;
    let content;
    let entity_id;
    if approved {
        let existing: Vec<(String, String)> =
            sqlx::query_as("SELECT name,code FROM departments WHERE company_id = ?")
                .bind(&p.company_id)
                .fetch_all(&mut *tx)
                .await?;
        if existing.iter().any(|(name, code)| {
            name.to_lowercase() == p.name.to_lowercase() || code.eq_ignore_ascii_case(&p.code)
        }) {
            return Err(TinaError::Validation("departamento já existente".into()));
        }
        entity_id = Uuid::new_v4().to_string();
        let result = sqlx::query(
            "INSERT INTO departments(id, company_id, code, name, description, status) SELECT ?,c.id,?,?,?,'active' FROM companies c WHERE c.id = ? AND c.installation_id = ? AND c.status = 'active' AND c.kind = ? AND c.name = ?"
        ).bind(&entity_id).bind(&p.code).bind(&p.name).bind(&p.description)
        .bind(&p.company_id).bind(installation).bind(&p.organization_kind).bind(&p.company_name)
        .execute(&mut *tx).await?;
        if result.rows_affected() != 1 {
            return Err(TinaError::Validation(
                "organização alterada ou indisponível; refaça o pedido".into(),
            ));
        }
        status = "approved";
        content = format!("Departamento criado com sucesso: {} ({}), na organização {}. Consulte a área Organizações.", p.name, p.code, p.company_name);
    } else {
        entity_id = p.id.clone();
        status = "rejected";
        content =
            "Criação de departamento recusada. Nenhum departamento foi criado por esta proposta."
                .to_string();
    }
    sqlx::query("INSERT INTO organizational_audit_events(id,installation_id,company_id,entity_type,entity_id,action,outcome,actor_type,actor_id,metadata_json) VALUES (?,?,?,'tool',?,'create_department',?,'owner',?,?)")
        .bind(Uuid::new_v4().to_string()).bind(installation).bind(&p.company_id).bind(entity_id)
        .bind(if approved {"success"} else {"denied"}).bind(owner)
        .bind(json!({"proposal_id":p.id,"session_id":session,"decision":status}).to_string()).execute(&mut *tx).await?;
    let response = assistant(&mut tx, installation, session, &content).await?;
    let updated = sqlx::query("UPDATE chat_department_proposals SET status = ?, lifecycle_status = ?, result_message_id = ?, decided_at = datetime('now'), updated_at = datetime('now') WHERE id = ? AND installation_id = ? AND session_id = ? AND lifecycle_status = 'pending'")
        .bind(status).bind(status).bind(&response.id).bind(proposal).bind(installation).bind(session)
        .execute(&mut *tx).await?;
    if updated.rows_affected() != 1 {
        return Err(TinaError::Validation(
            "proposta não está mais pendente; nenhuma alteração foi gravada".into(),
        ));
    }
    commit_owner_write(tx, installation).await?;
    Ok(response)
}

async fn decide_organization(
    mut tx: Transaction<'_, Sqlite>,
    installation: &str,
    owner: &str,
    session: &str,
    p: &DepartmentProposal,
    approved: bool,
) -> Result<ChatMessage> {
    let (status, entity, company, content) = if approved {
        let existing: Vec<(String, String)> =
            sqlx::query_as("SELECT name,kind FROM companies WHERE installation_id = ?")
                .bind(installation)
                .fetch_all(&mut *tx)
                .await?;
        if existing.iter().any(|(name, kind)| {
            name.to_lowercase() == p.name.to_lowercase()
                || (p.organization_kind == "personal" && kind == "personal")
        }) {
            return Err(TinaError::Validation("organização já existente".into()));
        }
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO companies(id,installation_id,name,kind,status) VALUES (?,?,?,?,'active')",
        )
        .bind(&id)
        .bind(installation)
        .bind(&p.name)
        .bind(&p.organization_kind)
        .execute(&mut *tx)
        .await?;
        let content = format!("Organização criada com sucesso: {}. Consulte a área Organizações. Nenhum departamento foi criado automaticamente.", p.name);
        ("approved", id.clone(), Some(id), content)
    } else {
        (
            "rejected",
            p.id.clone(),
            None,
            "Criação de organização recusada. Nenhuma organização foi criada por esta proposta."
                .into(),
        )
    };
    sqlx::query("INSERT INTO organizational_audit_events(id,installation_id,company_id,entity_type,entity_id,action,outcome,actor_type,actor_id,metadata_json) VALUES (?,?,?,'tool',?,'create_organization',?,'owner',?,?)")
        .bind(Uuid::new_v4().to_string()).bind(installation).bind(company).bind(entity)
        .bind(if approved {"success"} else {"denied"}).bind(owner)
        .bind(json!({"proposal_id":p.id,"session_id":session,"decision":status}).to_string()).execute(&mut *tx).await?;
    let response = assistant(&mut tx, installation, session, &content).await?;
    let updated = sqlx::query("UPDATE chat_organization_proposals SET status = ?, lifecycle_status = ?, result_message_id = ?, decided_at = datetime('now'), updated_at = datetime('now') WHERE id = ? AND installation_id = ? AND session_id = ? AND lifecycle_status = 'pending'")
        .bind(status).bind(status).bind(&response.id).bind(&p.id).bind(installation).bind(session)
        .execute(&mut *tx).await?;
    if updated.rows_affected() != 1 {
        return Err(TinaError::Validation(
            "proposta não está mais pendente; nenhuma alteração foi gravada".into(),
        ));
    }
    commit_owner_write(tx, installation).await?;
    Ok(response)
}

/// Cancela sem criar nada. Cancelamento repetido devolve a mesma mensagem;
/// aprovar uma proposta cancelada é sempre erro.
pub(crate) async fn cancel(
    pool: &SqlitePool,
    installation: &str,
    owner: &str,
    session: &str,
    proposal: &str,
) -> Result<ChatMessage> {
    let mut tx = begin_owner_write(pool, installation, owner).await?;
    let state = proposal_state(&mut tx, installation, session, proposal).await?;
    if let Some((lifecycle, message_id)) = state {
        match lifecycle.as_str() {
            "cancelled" => {
                let message = message_id.ok_or_else(|| {
                    TinaError::Validation("cancelamento sem mensagem registrada".into())
                })?;
                let response = result_message(&mut tx, installation, session, &message).await?;
                commit_owner_write(tx, installation).await?;
                return Ok(response);
            }
            "superseded" => {
                return Err(TinaError::Validation(
                    "proposta substituída por uma versão mais recente".into(),
                ))
            }
            "pending" if message_id.is_some() => {
                return Err(TinaError::Validation(
                    "proposta pendente com resultado inconsistente".into(),
                ))
            }
            "pending" => {}
            _ => {
                return Err(TinaError::Validation(
                    "proposta já decidida; não é possível cancelar".into(),
                ))
            }
        }
    }
    let p: DepartmentProposal = pending_by_id(&mut tx, installation, session, proposal)
        .await?
        .ok_or_else(|| TinaError::Unauthorized("proposta inexistente ou substituída".into()))?;
    let response = assistant(
        &mut tx,
        installation,
        session,
        "Proposta cancelada. Nada foi criado.",
    )
    .await?;
    let company = if p.tool_name == "create_organization" {
        None
    } else {
        Some(p.company_id.as_str())
    };
    sqlx::query("INSERT INTO organizational_audit_events(id,installation_id,company_id,entity_type,entity_id,action,outcome,actor_type,actor_id,metadata_json) VALUES (?,?,?,'tool',?,'cancel_proposal','success','owner',?,?)")
        .bind(Uuid::new_v4().to_string()).bind(installation).bind(company).bind(&p.id)
        .bind(owner)
        .bind(json!({"proposal_id":p.id,"session_id":session,"decision":"cancelled"}).to_string()).execute(&mut *tx).await?;
    const CANCEL: &str = "UPDATE {table} SET status = 'rejected', lifecycle_status = 'cancelled', result_message_id = ?, decided_at = datetime('now'), cancelled_at = datetime('now'), cancel_reason = 'Pedido do proprietário', updated_at = datetime('now') WHERE id = ? AND installation_id = ? AND session_id = ? AND lifecycle_status = 'pending'";
    let table = if p.tool_name == "create_organization" {
        "chat_organization_proposals"
    } else if p.tool_name == "create_position" {
        "chat_position_proposals"
    } else if p.tool_name == "generate_contract" {
        "chat_contract_drafts"
    } else if super::tool_access::is_management(&p.tool_name) {
        "chat_tool_access_proposals"
    } else {
        "chat_department_proposals"
    };
    let sql = CANCEL.replace("{table}", table);
    let updated = sqlx::query(&sql)
        .bind(&response.id)
        .bind(&p.id)
        .bind(installation)
        .bind(session)
        .execute(&mut *tx)
        .await?;
    if updated.rows_affected() != 1 {
        return Err(TinaError::Validation(
            "proposta não está mais pendente; nenhuma alteração foi gravada".into(),
        ));
    }
    commit_owner_write(tx, installation).await?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(prefix: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("tina-{}-{}.db", prefix, Uuid::new_v4()))
    }

    async fn append_user(app: &crate::TinaApp, session: &str, content: &str) -> ChatMessage {
        app.owner_append_chat_message(session, content)
            .await
            .unwrap()
    }

    async fn lifecycle(pool: &SqlitePool, table: &str, id: &str) -> (String, Option<String>) {
        let sql = format!(
            "SELECT lifecycle_status,superseded_by FROM {} WHERE id = ?",
            table
        );
        sqlx::query_as(&sql).bind(id).fetch_one(pool).await.unwrap()
    }

    #[test]
    fn definitions_expose_all_gated_tools() {
        let tools = definitions();
        let names: Vec<&str> = tools
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["function"]["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            vec![
                "create_department",
                "create_organization",
                "create_position",
                "generate_contract",
                "assign_tool",
                "revoke_tool",
                "assign_app",
                "revoke_app"
            ]
        );
        for tool in tools.as_array().unwrap() {
            assert!(tool["function"]["parameters"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|field| field.as_str() == Some("reason")));
        }
    }

    #[test]
    fn rejects_unknown_tools_and_forged_authority() {
        let args = r#"{"organization_kind":"personal","name":"Finanças","code":"fin"}"#;
        assert_eq!(parse("create_department", args).unwrap().code, "FIN");
        assert!(parse("shell", args).is_err());
        assert!(parse(
            "create_department",
            r#"{"organization_kind":"personal","name":"Finanças","code":"fin","owner_id":"fake"}"#
        )
        .is_err());
        assert!(parse(
            "create_department",
            r#"{"organization_kind":"business","name":"Finanças","code":"fin"}"#
        )
        .is_err());
    }

    #[tokio::test]
    async fn approval_is_persistent_scoped_and_idempotent() {
        let path = temp_path("tool");
        let app = crate::TinaApp::initialize_at(&path, "tool-install", "tool-owner")
            .await
            .unwrap();
        app.unlock_owner_for_test().unwrap();
        let company = app
            .owner_create_company_with_kind("Pessoal", "personal")
            .await
            .unwrap();
        let session = app.owner_create_chat("Ferramenta").await.unwrap();
        let user = append_user(&app, &session.id, "Crie Finanças pessoais").await;
        propose(
            app.database().pool(),
            "tool-install",
            &session.id,
            &user.id,
            "create_department",
            r#"{"organization_kind":"personal","name":"Finanças","code":"FIN"}"#,
        )
        .await
        .unwrap();
        // Mensagens posteriores não podem esconder a proposta pendente.
        append_user(&app, &session.id, "Agora fale sobre rotina").await;
        infra_assistant(&app, &session.id, "Vamos organizar isso depois.").await;
        let proposal = app
            .owner_pending_department(&session.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(proposal.lifecycle_status, "pending");
        assert!(
            crate::infra::DepartmentRepository::new(app.database().pool())
                .list_by_company(&company.id)
                .await
                .unwrap()
                .is_empty()
        );
        let first = app
            .owner_decide_department(&session.id, &proposal.id, true)
            .await
            .unwrap();
        let again = app
            .owner_decide_department(&session.id, &proposal.id, true)
            .await
            .unwrap();
        assert_eq!(first.id, again.id);
        assert!(app
            .owner_decide_department(&session.id, &proposal.id, false)
            .await
            .is_err());
        assert_eq!(
            crate::infra::DepartmentRepository::new(app.database().pool())
                .list_by_company(&company.id)
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(app
            .owner_pending_department(&session.id)
            .await
            .unwrap()
            .is_none());
        assert!(app
            .owner_decide_department("other-session", &proposal.id, true)
            .await
            .is_err());
        app.database().pool().close().await;
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn proactive_organization_requires_approval_and_is_idempotent() {
        let path = temp_path("org-tool");
        let app = crate::TinaApp::initialize_at(&path, "org-tool-install", "org-tool-owner")
            .await
            .unwrap();
        app.unlock_owner_for_test().unwrap();
        let session = app.owner_create_chat("Rotina").await.unwrap();
        let user = append_user(
            &app,
            &session.id,
            "Estou com dificuldade de organizar minha rotina",
        )
        .await;
        let pool = app.database().pool();
        propose(
            pool,
            "org-tool-install",
            &session.id,
            &user.id,
            "create_organization",
            r#"{"organization_kind":"personal","name":"Pessoal","reason":"Separar rotina e compromissos pessoais"}"#,
        )
        .await
        .unwrap();
        let repo = crate::infra::CompanyRepository::new(pool);
        assert!(repo
            .list_by_installation("org-tool-install")
            .await
            .unwrap()
            .is_empty());
        append_user(&app, &session.id, "Outra mensagem qualquer").await;
        let proposal = app
            .owner_pending_department(&session.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(proposal.tool_name, "create_organization");
        let first = app
            .owner_decide_department(&session.id, &proposal.id, true)
            .await
            .unwrap();
        let again = app
            .owner_decide_department(&session.id, &proposal.id, true)
            .await
            .unwrap();
        assert_eq!(first.id, again.id);
        let rows = repo.list_by_installation("org-tool-install").await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, "personal");
        assert!(app
            .owner_pending_department(&session.id)
            .await
            .unwrap()
            .is_none());
        pool.close().await;
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn new_proposal_supersedes_previous_and_stale_approval_fails() {
        let path = temp_path("supersede");
        let app = crate::TinaApp::initialize_at(&path, "supersede-install", "supersede-owner")
            .await
            .unwrap();
        app.unlock_owner_for_test().unwrap();
        app.owner_create_company_with_kind("Pessoal", "personal")
            .await
            .unwrap();
        let session = app.owner_create_chat("Substituição").await.unwrap();
        let first_user = append_user(&app, &session.id, "Crie Finanças").await;
        propose(
            app.database().pool(),
            "supersede-install",
            &session.id,
            &first_user.id,
            "create_department",
            r#"{"organization_kind":"personal","name":"Finanças","code":"FIN","reason":"Separar contas"}"#,
        )
        .await
        .unwrap();
        let first = app
            .owner_pending_department(&session.id)
            .await
            .unwrap()
            .unwrap();
        let next_user = append_user(&app, &session.id, "Na verdade, crie Operações").await;
        propose(
            app.database().pool(),
            "supersede-install",
            &session.id,
            &next_user.id,
            "create_department",
            r#"{"organization_kind":"personal","name":"Operações","code":"OPS","reason":"Rotina operacional"}"#,
        )
        .await
        .unwrap();
        let second = app
            .owner_pending_department(&session.id)
            .await
            .unwrap()
            .unwrap();
        assert_ne!(first.id, second.id);
        let (state, by) = lifecycle(
            app.database().pool(),
            "chat_department_proposals",
            &first.id,
        )
        .await;
        assert_eq!(state, "superseded");
        assert_eq!(by.as_deref(), Some(second.id.as_str()));
        assert!(app
            .owner_decide_department(&session.id, &first.id, true)
            .await
            .is_err());
        // Proposta idêntica não cria duplicata nem substitui a ativa.
        let same_user = append_user(&app, &session.id, "Crie Operações novamente").await;
        propose(
            app.database().pool(),
            "supersede-install",
            &session.id,
            &same_user.id,
            "create_department",
            r#"{"organization_kind":"personal","name":"Operações","code":"OPS","reason":"Rotina operacional"}"#,
        )
        .await
        .unwrap();
        let still = app
            .owner_pending_department(&session.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(still.id, second.id);
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM chat_department_proposals WHERE installation_id = 'supersede-install' AND lifecycle_status = 'pending'",
        )
        .fetch_one(app.database().pool())
        .await
        .unwrap();
        assert_eq!(count, 1);
        app.database().pool().close().await;
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn cancel_is_owner_scoped_idempotent_and_blocks_approval() {
        let path = temp_path("cancel");
        let app = crate::TinaApp::initialize_at(&path, "cancel-install", "cancel-owner")
            .await
            .unwrap();
        app.unlock_owner_for_test().unwrap();
        app.owner_create_company_with_kind("Pessoal", "personal")
            .await
            .unwrap();
        let session = app.owner_create_chat("Cancelamento").await.unwrap();
        let user = append_user(&app, &session.id, "Crie Finanças").await;
        propose(
            app.database().pool(),
            "cancel-install",
            &session.id,
            &user.id,
            "create_department",
            r#"{"organization_kind":"personal","name":"Finanças","code":"FIN","reason":"Separar contas"}"#,
        )
        .await
        .unwrap();
        append_user(&app, &session.id, "Deixa para lá").await;
        let proposal = app
            .owner_pending_department(&session.id)
            .await
            .unwrap()
            .unwrap();
        assert!(app
            .owner_cancel_proposal("outra-sessao", &proposal.id)
            .await
            .is_err());
        let first = app
            .owner_cancel_proposal(&session.id, &proposal.id)
            .await
            .unwrap();
        let again = app
            .owner_cancel_proposal(&session.id, &proposal.id)
            .await
            .unwrap();
        assert_eq!(first.id, again.id);
        assert!(app
            .owner_decide_department(&session.id, &proposal.id, true)
            .await
            .is_err());
        let (state, _) = lifecycle(
            app.database().pool(),
            "chat_department_proposals",
            &proposal.id,
        )
        .await;
        assert_eq!(state, "cancelled");
        assert!(app
            .owner_pending_department(&session.id)
            .await
            .unwrap()
            .is_none());
        let audit: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM organizational_audit_events WHERE installation_id = 'cancel-install' AND action = 'cancel_proposal' AND actor_id = 'cancel-owner'",
        )
        .fetch_one(app.database().pool())
        .await
        .unwrap();
        assert_eq!(audit, 1);
        let departments: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM departments")
            .fetch_one(app.database().pool())
            .await
            .unwrap();
        assert_eq!(departments, 0);
        app.database().pool().close().await;
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn personal_organization_is_resolved_by_kind_not_by_name() {
        let path = temp_path("kind");
        let app = crate::TinaApp::initialize_at(&path, "kind-install", "kind-owner")
            .await
            .unwrap();
        app.unlock_owner_for_test().unwrap();
        let business = app
            .owner_create_company_with_kind("Pessoal", "business")
            .await
            .unwrap();
        let personal = app
            .owner_create_company_with_kind("Vida", "personal")
            .await
            .unwrap();
        let session = app.owner_create_chat("Tipo").await.unwrap();
        let user = append_user(&app, &session.id, "Crie Finanças na vida pessoal").await;
        propose(
            app.database().pool(),
            "kind-install",
            &session.id,
            &user.id,
            "create_department",
            r#"{"organization_kind":"personal","name":"Finanças","code":"FIN","reason":"Organizar gastos"}"#,
        )
        .await
        .unwrap();
        let proposal = app
            .owner_pending_department(&session.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(proposal.company_id, personal.id);
        assert_ne!(proposal.company_id, business.id);
        app.owner_decide_department(&session.id, &proposal.id, true)
            .await
            .unwrap();
        let in_business: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM departments WHERE company_id = ?")
                .bind(&business.id)
                .fetch_one(app.database().pool())
                .await
                .unwrap();
        let in_personal: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM departments WHERE company_id = ?")
                .bind(&personal.id)
                .fetch_one(app.database().pool())
                .await
                .unwrap();
        assert_eq!((in_business, in_personal), (0, 1));
        app.database().pool().close().await;
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn rh_proposals_share_lifecycle_and_cancel() {
        let path = temp_path("rh-lifecycle");
        let app = crate::TinaApp::initialize_at(&path, "rh-life-install", "rh-life-owner")
            .await
            .unwrap();
        app.unlock_owner_for_test().unwrap();
        let company = app
            .owner_create_company_with_kind("Pessoal", "personal")
            .await
            .unwrap();
        let department = app
            .owner_create_department(&company.id, "FIN", "Finanças", None)
            .await
            .unwrap();
        let session = app.owner_create_chat("RH ciclo").await.unwrap();
        let pool = app.database().pool();

        let user = append_user(&app, &session.id, "Preciso de um cargo").await;
        propose(
            pool,
            "rh-life-install",
            &session.id,
            &user.id,
            "create_position",
            &format!(
                r#"{{"organization_kind":"personal","company_name":"Pessoal","department_name":"{}","name":"Analista","level":1,"description":"Acompanhar orçamento","reason":"Organizar finanças"}}"#,
                department.name
            ),
        )
        .await
        .unwrap();
        let proposal = app
            .owner_pending_department(&session.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(proposal.tool_name, "create_position");
        let first = proposal.id.clone();
        let cancelled = app
            .owner_cancel_proposal(&session.id, &first)
            .await
            .unwrap();
        let again = app
            .owner_cancel_proposal(&session.id, &first)
            .await
            .unwrap();
        assert_eq!(cancelled.id, again.id);
        assert!(app
            .owner_decide_department(&session.id, &first, true)
            .await
            .is_err());
        assert_eq!(
            lifecycle(pool, "chat_position_proposals", &first).await.0,
            "cancelled"
        );
        let positions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM positions")
            .fetch_one(pool)
            .await
            .unwrap();
        assert_eq!(positions, 0);

        let user = append_user(&app, &session.id, "Crie o cargo novamente").await;
        propose(
            pool,
            "rh-life-install",
            &session.id,
            &user.id,
            "create_position",
            &format!(
                r#"{{"organization_kind":"personal","company_name":"Pessoal","department_name":"{}","name":"Analista II","level":1,"description":"Apoiar rotina","reason":"Nova demanda"}}"#,
                department.name
            ),
        )
        .await
        .unwrap();
        let second = app
            .owner_pending_department(&session.id)
            .await
            .unwrap()
            .unwrap();
        let user = append_user(&app, &session.id, "Na verdade, crie Operações").await;
        propose(
            pool,
            "rh-life-install",
            &session.id,
            &user.id,
            "create_department",
            r#"{"organization_kind":"personal","name":"Operações","code":"OPS","reason":"Rotina operacional"}"#,
        )
        .await
        .unwrap();
        let current = app
            .owner_pending_department(&session.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(current.tool_name, "create_department");
        let (state, by) = lifecycle(pool, "chat_position_proposals", &second.id).await;
        assert_eq!(state, "superseded");
        assert_eq!(by.as_deref(), Some(current.id.as_str()));
        let pending: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM chat_position_proposals WHERE installation_id='rh-life-install' AND lifecycle_status='pending'",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(pending, 0);
        pool.close().await;
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn contract_draft_approval_is_idempotent_and_lifecycle_aware() {
        let path = temp_path("contract-life");
        let app =
            crate::TinaApp::initialize_at(&path, "contract-life-install", "contract-life-owner")
                .await
                .unwrap();
        app.unlock_owner_for_test().unwrap();
        let company = app
            .owner_create_company_with_kind("Pessoal", "personal")
            .await
            .unwrap();
        let department = app
            .owner_create_department(&company.id, "FIN", "Finanças", None)
            .await
            .unwrap();
        app.owner_create_position(
            &department.id,
            "Analista",
            1,
            0,
            Some("Acompanhar orçamento"),
        )
        .await
        .unwrap();
        let session = app.owner_create_chat("Minuta").await.unwrap();
        let pool = app.database().pool();
        let user = append_user(&app, &session.id, "Prepare uma minuta").await;
        propose(
            pool,
            "contract-life-install",
            &session.id,
            &user.id,
            "generate_contract",
            r#"{"organization_kind":"personal","company_name":"Pessoal","department_name":"Finanças","position_name":"Analista","name":"Ana","personality":"Objetiva","specialty":"Finanças","responsibilities":"Acompanhar orçamento","objectives":"Relatório semanal","supervision":"Tina","limits":"Somente leitura","requested_tools":"Planilha","budget":"Desconhecido","review":"Mensal","reason":"Demanda concreta"}"#,
        )
        .await
        .unwrap();
        let proposal = app
            .owner_pending_department(&session.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(proposal.tool_name, "generate_contract");
        let first = app
            .owner_decide_department(&session.id, &proposal.id, true)
            .await
            .unwrap();
        let again = app
            .owner_decide_department(&session.id, &proposal.id, true)
            .await
            .unwrap();
        assert_eq!(first.id, again.id);
        assert_eq!(
            lifecycle(pool, "chat_contract_drafts", &proposal.id)
                .await
                .0,
            "approved"
        );
        let employees: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM employees")
            .fetch_one(pool)
            .await
            .unwrap();
        assert_eq!(employees, 0);
        pool.close().await;
        let _ = std::fs::remove_file(path);
    }

    /// Escreve uma resposta assistente apenas para simular mensagens posteriores.
    async fn infra_assistant(app: &crate::TinaApp, session: &str, content: &str) {
        sqlx::query(
            "INSERT INTO chat_messages(id,session_id,installation_id,role,content) VALUES (?,?,'tool-install','assistant',?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(session)
        .bind(content)
        .execute(app.database().pool())
        .await
        .unwrap();
    }
}
