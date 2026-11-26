//! Minutas operacionais de RH, sem ativação de agentes ou concessão de acesso.
use crate::error::{Result, TinaError};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

use super::audit::commit_owner_write;
use super::chat::ChatMessage;
use super::chat_tools::assistant;

pub(super) fn definition() -> Value {
    json!({
        "type":"function","function":{
            "name":"generate_contract",
            "description":"Prepara uma minuta operacional de funcionário IA para cargo existente. Solicita autorização para gravar a minuta. Não contrata, não escolhe modelo e não concede ferramentas.",
            "parameters":{
                "type":"object","additionalProperties":false,
                "properties":{
                    "organization_kind":{"type":"string","enum":["personal","business"]},
                    "company_name":{"type":"string"},"department_name":{"type":"string"},
                    "position_name":{"type":"string"},"name":{"type":"string"},
                    "personality":{"type":"string"},"specialty":{"type":"string"},
                    "responsibilities":{"type":"string"},"objectives":{"type":"string"},
                    "supervision":{"type":"string","description":"Superior proposto, não atribuição efetiva."},
                    "limits":{"type":"string","description":"Autonomia e limites de acesso a dados."},
                    "requested_tools":{"type":"string","description":"Ferramentas desejadas, não permissões."},
                    "budget":{"type":"string","description":"Orçamento proposto ou desconhecido."},
                    "review":{"type":"string","description":"Critérios para revisão do contrato."},
                    "reason":{"type":"string"}
                },
                "required":["organization_kind","company_name","department_name","position_name","name","personality","specialty","responsibilities","objectives","supervision","limits","requested_tools","budget","review","reason"]
            }
        }
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    organization_kind: String,
    company_name: String,
    department_name: String,
    position_name: String,
    name: String,
    personality: String,
    specialty: String,
    responsibilities: String,
    objectives: String,
    supervision: String,
    limits: String,
    requested_tools: String,
    budget: String,
    review: String,
    reason: String,
}

fn parse(input: &str) -> Result<Args> {
    if input.len() > 16_384 {
        return Err(TinaError::Validation("minuta extensa".into()));
    }
    let mut a: Args = serde_json::from_str(input)
        .map_err(|_| TinaError::Validation("argumentos de contrato inválidos".into()))?;
    if !matches!(a.organization_kind.as_str(), "personal" | "business") {
        return Err(TinaError::Validation("tipo de organização inválido".into()));
    }
    for (field, max) in [
        (&mut a.company_name, 160),
        (&mut a.department_name, 120),
        (&mut a.position_name, 120),
        (&mut a.name, 120),
        (&mut a.personality, 500),
        (&mut a.specialty, 500),
        (&mut a.responsibilities, 1000),
        (&mut a.objectives, 1000),
        (&mut a.supervision, 500),
        (&mut a.limits, 1000),
        (&mut a.requested_tools, 500),
        (&mut a.budget, 500),
        (&mut a.review, 1000),
        (&mut a.reason, 500),
    ] {
        *field = field.trim().to_owned();
        if field.is_empty() || field.chars().count() > max || field.chars().any(char::is_control) {
            return Err(TinaError::Validation("campo de contrato inválido".into()));
        }
    }
    Ok(a)
}

pub(super) async fn propose(
    tx: &mut Transaction<'_, Sqlite>,
    installation: &str,
    session: &str,
    user: &str,
    input: &str,
) -> Result<ChatMessage> {
    let a = parse(input)?;
    let rows: Vec<(String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT c.id,c.name,d.id,d.name,p.id,p.name FROM positions p JOIN departments d ON d.id=p.department_id JOIN companies c ON c.id=d.company_id WHERE c.installation_id=? AND c.kind=? AND c.status='active' AND d.status='active' AND p.status='active'"
    ).bind(installation).bind(&a.organization_kind).fetch_all(&mut **tx).await?;
    let mut matches = rows.iter().filter(|(_, c, _, d, _, p)| {
        c.to_lowercase() == a.company_name.to_lowercase()
            && d.to_lowercase() == a.department_name.to_lowercase()
            && p.to_lowercase() == a.position_name.to_lowercase()
    });
    let Some((
        company_id,
        company_name,
        department_id,
        department_name,
        position_id,
        position_name,
    )) = matches.next()
    else {
        return assistant(tx, installation, session, "Não encontrei o cargo ativo. Precisamos identificar o destino antes de preparar o contrato.").await;
    };
    if matches.next().is_some() {
        return assistant(
            tx,
            installation,
            session,
            "O destino é ambíguo. Precisamos identificar o cargo exato.",
        )
        .await;
    }
    let description = format!(
        "Minuta operacional — versão 1\nDepartamento: {department_name}\nCargo: {position_name}\nPersonalidade: {}\nEspecialidade: {}\nResponsabilidades: {}\nObjetivos: {}\nSupervisão proposta: {}\nLimites e dados: {}\nFerramentas solicitadas (não concedidas): {}\nOrçamento proposto: {}\nRevisão: {}\nModelo: pendente de avaliação. Nenhum funcionário será ativado.",
        a.personality, a.specialty, a.responsibilities, a.objectives, a.supervision,
        a.limits, a.requested_tools, a.budget, a.review
    );
    let response = assistant(tx, installation, session,
        "Preparei uma minuta de contrato operacional. Posso salvá-la para avaliação? Isso ainda não contrata o funcionário.").await?;
    sqlx::query("INSERT INTO chat_contract_drafts(id,installation_id,session_id,user_message_id,proposal_message_id,company_id,company_name,organization_kind,department_id,department_name,position_id,position_name,name,description,reason) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(Uuid::new_v4().to_string()).bind(installation).bind(session).bind(user)
        .bind(&response.id).bind(company_id).bind(company_name).bind(&a.organization_kind)
        .bind(department_id).bind(department_name).bind(position_id).bind(position_name)
        .bind(&a.name).bind(description).bind(&a.reason).execute(&mut **tx).await?;
    Ok(response)
}

pub(super) async fn decide(
    mut tx: Transaction<'_, Sqlite>,
    installation: &str,
    owner: &str,
    session: &str,
    proposal: &str,
    approved: bool,
) -> Result<ChatMessage> {
    let (company, name, description): (String, String, String) = sqlx::query_as(
        "SELECT company_id,name,description FROM chat_contract_drafts WHERE id=? AND installation_id=? AND session_id=? AND lifecycle_status='pending'"
    ).bind(proposal).bind(installation).bind(session).fetch_one(&mut *tx).await?;
    if approved {
        let valid: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chat_contract_drafts a JOIN positions p ON p.id=a.position_id JOIN departments d ON d.id=p.department_id JOIN companies c ON c.id=d.company_id WHERE a.id=? AND a.installation_id=? AND c.installation_id=a.installation_id AND c.id=a.company_id AND c.name=a.company_name AND c.kind=a.organization_kind AND d.id=a.department_id AND d.name=a.department_name AND p.name=a.position_name AND c.status='active' AND d.status='active' AND p.status='active'")
            .bind(proposal).bind(installation).fetch_one(&mut *tx).await?;
        if valid != 1 {
            return Err(TinaError::Validation(
                "destino alterado ou indisponível".into(),
            ));
        }
    }
    let content = if approved {
        format!("Minuta de contrato salva para {name} (referência: {proposal}). Ainda falta avaliar e selecionar o modelo e autorizar a contratação. Nenhuma ferramenta foi concedida.\n\n{description}")
    } else {
        "Minuta recusada. Nenhum agente foi contratado.".to_owned()
    };
    let response = assistant(&mut tx, installation, session, &content).await?;
    sqlx::query("INSERT INTO organizational_audit_events(id,installation_id,company_id,entity_type,entity_id,action,outcome,actor_type,actor_id,metadata_json) VALUES (?,?,?,'tool',?,'generate_contract',?,'owner',?,?)")
        .bind(Uuid::new_v4().to_string()).bind(installation).bind(company).bind(proposal)
        .bind(if approved {"success"} else {"denied"}).bind(owner)
        .bind(json!({"proposal_id":proposal,"session_id":session,"draft_only":true}).to_string())
        .execute(&mut *tx).await?;
    let status = if approved { "approved" } else { "rejected" };
    let updated = sqlx::query("UPDATE chat_contract_drafts SET status=?,lifecycle_status=?,result_message_id=?,decided_at=datetime('now'),updated_at=datetime('now') WHERE id=? AND installation_id=? AND session_id=? AND lifecycle_status='pending'")
        .bind(status).bind(status).bind(&response.id)
        .bind(proposal).bind(installation).bind(session).execute(&mut *tx).await?;
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
    fn contract_cannot_accept_injected_permissions() {
        let mut args = json!({
            "organization_kind":"personal","company_name":"Pessoal","department_name":"Finanças",
            "position_name":"Analista","name":"Ana","personality":"Objetiva","specialty":"Finanças",
            "responsibilities":"Acompanhar orçamento","objectives":"Relatório semanal",
            "supervision":"Tina","limits":"Somente leitura","requested_tools":"Planilha",
            "budget":"Desconhecido","review":"Revisão mensal","reason":"Demanda concreta"
        });
        assert!(parse(&args.to_string()).is_ok());
        args["authority_level"] = json!(99);
        assert!(parse(&args.to_string()).is_err());
    }
}
