//! Ferramentas de RH. A primeira etapa propõe cargos, sem contratar agentes.

use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

use super::audit::commit_owner_write;
use super::chat::ChatMessage;
use super::chat_tools::assistant;
use crate::error::{Result, TinaError};

pub(super) fn position_definition() -> Value {
    json!({
        "type":"function",
        "function":{
            "name":"create_position",
            "description":"Propõe um cargo para necessidade de RH concreta, em departamento existente. Não contrata agente e não concede autoridade adicional. Exige aprovação.",
            "parameters":{
                "type":"object",
                "additionalProperties":false,
                "properties":{
                    "organization_kind":{"type":"string","enum":["personal","business"]},
                    "company_name":{"type":"string"},
                    "department_name":{"type":"string"},
                    "name":{"type":"string"},
                    "level":{"type":"integer","minimum":0},
                    "description":{"type":"string","description":"Responsabilidades e finalidade do cargo."},
                    "reason":{"type":"string","description":"Necessidade de RH identificada."}
                },
                "required":["organization_kind","company_name","department_name","name","level","description","reason"]
            }
        }
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PositionArgs {
    organization_kind: String,
    company_name: String,
    department_name: String,
    name: String,
    level: i32,
    description: String,
    reason: String,
}

fn parse(input: &str) -> Result<PositionArgs> {
    if input.len() > 16_384 {
        return Err(TinaError::Validation("argumentos extensos".into()));
    }
    let mut args: PositionArgs = serde_json::from_str(input)
        .map_err(|_| TinaError::Validation("argumentos de cargo inválidos".into()))?;
    for field in [
        &mut args.company_name,
        &mut args.department_name,
        &mut args.name,
        &mut args.description,
        &mut args.reason,
    ] {
        *field = field.trim().to_owned();
    }
    if !matches!(args.organization_kind.as_str(), "personal" | "business")
        || args.level < 0
        || [
            (&args.company_name, 160),
            (&args.department_name, 120),
            (&args.name, 120),
            (&args.description, 2000),
            (&args.reason, 500),
        ]
        .iter()
        .any(|(s, max)| s.is_empty() || s.chars().count() > *max || s.chars().any(char::is_control))
    {
        return Err(TinaError::Validation("dados de cargo inválidos".into()));
    }
    Ok(args)
}

pub(super) async fn propose_position(
    tx: &mut Transaction<'_, Sqlite>,
    installation: &str,
    session: &str,
    user: &str,
    input: &str,
) -> Result<ChatMessage> {
    let args = parse(input)?;
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT c.id,c.name,d.id,d.name FROM companies c JOIN departments d ON d.company_id=c.id WHERE c.installation_id=? AND c.kind=? AND c.status='active' AND d.status='active'"
    ).bind(installation).bind(&args.organization_kind).fetch_all(&mut **tx).await?;
    let mut matches = rows.iter().filter(|(_, company, _, department)| {
        company.to_lowercase() == args.company_name.to_lowercase()
            && department.to_lowercase() == args.department_name.to_lowercase()
    });
    let target = matches.next();
    if target.is_none() || matches.next().is_some() {
        return assistant(tx, installation, session,
            "Não encontrei um departamento ativo e inequívoco nessa organização. Precisamos identificar o destino antes de criar um cargo.").await;
    }
    let (company_id, company_name, department_id, department_name) = target.unwrap();
    let existing: Vec<String> =
        sqlx::query_scalar("SELECT name FROM positions WHERE department_id=?")
            .bind(department_id)
            .fetch_all(&mut **tx)
            .await?;
    if existing
        .iter()
        .any(|name| name.to_lowercase() == args.name.to_lowercase())
    {
        return assistant(tx, installation, session,
            "Já existe um cargo com esse nome no departamento. Vamos avaliar o cargo existente antes de duplicá-lo.").await;
    }
    let response = assistant(tx, installation, session,
        "Identifiquei uma necessidade de RH. Posso criar este cargo? Isso não contrata um agente nem concede permissões.").await?;
    sqlx::query("INSERT INTO chat_position_proposals(id,installation_id,session_id,user_message_id,proposal_message_id,company_id,company_name,organization_kind,department_id,department_name,name,level,description,reason) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(Uuid::new_v4().to_string()).bind(installation).bind(session).bind(user)
        .bind(&response.id).bind(company_id).bind(company_name).bind(&args.organization_kind)
        .bind(department_id).bind(department_name).bind(&args.name).bind(args.level)
        .bind(&args.description).bind(&args.reason).execute(&mut **tx).await?;
    Ok(response)
}

#[derive(sqlx::FromRow)]
struct StoredPosition {
    company_id: String,
    company_name: String,
    organization_kind: String,
    department_id: String,
    department_name: String,
    name: String,
    level: i32,
    description: String,
}

pub(super) async fn decide_position(
    mut tx: Transaction<'_, Sqlite>,
    installation: &str,
    owner: &str,
    session: &str,
    proposal: &str,
    approved: bool,
) -> Result<ChatMessage> {
    let p: StoredPosition = sqlx::query_as("SELECT company_id,company_name,organization_kind,department_id,department_name,name,level,description FROM chat_position_proposals WHERE id=? AND installation_id=? AND session_id=? AND lifecycle_status='pending'")
        .bind(proposal).bind(installation).bind(session).fetch_one(&mut *tx).await?;
    let (status, entity, content) = if approved {
        let names: Vec<String> =
            sqlx::query_scalar("SELECT name FROM positions WHERE department_id=?")
                .bind(&p.department_id)
                .fetch_all(&mut *tx)
                .await?;
        if names
            .iter()
            .any(|name| name.to_lowercase() == p.name.to_lowercase())
        {
            return Err(TinaError::Validation("cargo já existente".into()));
        }
        let id = Uuid::new_v4().to_string();
        let result = sqlx::query("INSERT INTO positions(id,department_id,name,level,authority_level,description,status) SELECT ?,d.id,?,?,0,?,'active' FROM departments d JOIN companies c ON c.id=d.company_id WHERE d.id=? AND d.name=? AND d.status='active' AND c.id=? AND c.name=? AND c.kind=? AND c.installation_id=? AND c.status='active'")
            .bind(&id).bind(&p.name).bind(p.level).bind(&p.description).bind(&p.department_id)
            .bind(&p.department_name).bind(&p.company_id).bind(&p.company_name)
            .bind(&p.organization_kind).bind(installation).execute(&mut *tx).await?;
        if result.rows_affected() != 1 {
            return Err(TinaError::Validation(
                "destino alterado ou indisponível".into(),
            ));
        }
        let content = format!(
            "Cargo criado: {} em {} / {}. Ainda não há agente contratado por esta proposta.",
            p.name, p.company_name, p.department_name
        );
        ("approved", id, content)
    } else {
        (
            "rejected",
            proposal.to_owned(),
            "Criação de cargo recusada. Nenhum cargo ou agente foi criado por esta proposta."
                .to_owned(),
        )
    };
    sqlx::query("INSERT INTO organizational_audit_events(id,installation_id,company_id,entity_type,entity_id,action,outcome,actor_type,actor_id,metadata_json) VALUES (?,?,?,'tool',?,'create_position',?,'owner',?,?)")
        .bind(Uuid::new_v4().to_string()).bind(installation).bind(&p.company_id).bind(entity)
        .bind(if approved {"success"} else {"denied"}).bind(owner)
        .bind(json!({"proposal_id":proposal,"session_id":session}).to_string()).execute(&mut *tx).await?;
    let response = assistant(&mut tx, installation, session, &content).await?;
    let updated = sqlx::query("UPDATE chat_position_proposals SET status=?,lifecycle_status=?,result_message_id=?,decided_at=datetime('now'),updated_at=datetime('now') WHERE id=? AND installation_id=? AND session_id=? AND lifecycle_status='pending'")
        .bind(status).bind(status).bind(&response.id).bind(proposal).bind(installation).bind(session).execute(&mut *tx).await?;
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

    #[test]
    fn rejects_untrusted_authority() {
        assert!(parse(r#"{"organization_kind":"personal","company_name":"Pessoal","department_name":"Finanças","name":"Analista","level":1,"description":"Acompanhar orçamento","reason":"Organização","authority_level":9}"#).is_err());
    }

    #[tokio::test]
    async fn position_proposal_requires_approval_and_does_not_hire() {
        let path = std::env::temp_dir().join(format!("tina-rh-{}.db", Uuid::new_v4()));
        let app = crate::TinaApp::initialize_at(&path, "rh-install", "rh-owner")
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
        let session = app.owner_create_chat("RH").await.unwrap();
        let user = app
            .owner_append_chat_message(&session.id, "Preciso acompanhar melhor meu orçamento")
            .await
            .unwrap();
        let pool = app.database().pool();
        super::super::chat_tools::propose(pool, "rh-install", &session.id, &user.id, "create_position",
            r#"{"organization_kind":"personal","company_name":"Pessoal","department_name":"Finanças","name":"Analista","level":1,"description":"Acompanhar orçamento","reason":"Organizar finanças"}"#).await.unwrap();
        let count = || {
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM positions WHERE department_id=?")
                .bind(&department.id)
                .fetch_one(pool)
        };
        assert_eq!(count().await.unwrap(), 0);
        let proposal = app
            .owner_pending_department(&session.id)
            .await
            .unwrap()
            .unwrap();
        // Consulta a identidade persistida da proposta sem aceitar IDs do modelo.
        let id: String =
            sqlx::query_scalar("SELECT id FROM chat_position_proposals WHERE session_id=?")
                .bind(&session.id)
                .fetch_one(pool)
                .await
                .unwrap();
        drop(proposal);
        let first = app
            .owner_decide_department(&session.id, &id, true)
            .await
            .unwrap();
        let again = app
            .owner_decide_department(&session.id, &id, true)
            .await
            .unwrap();
        assert_eq!(first.id, again.id);
        assert_eq!(count().await.unwrap(), 1);
        let employees: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM employees")
            .fetch_one(pool)
            .await
            .unwrap();
        assert_eq!(employees, 0);
        pool.close().await;
        let _ = std::fs::remove_file(path);
    }
}
