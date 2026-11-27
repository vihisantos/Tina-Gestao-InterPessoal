//! Validação determinística de dependências e referências simbólicas.
//!
//! O modelo pode declarar dependências, mas o backend nunca confia apenas nele:
//! a ordem também é derivada das regras de negócio conhecidas (organização →
//! departamento → cargo → minuta). Referências simbólicas `@id.campo` só são
//! resolvidas depois que a etapa anterior foi executada e autorizada.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;

use crate::error::{Result, TinaError};
use crate::orchestration::calls::ToolCall;

use super::plan::PreparedStep;

/// Campos permitidos em uma referência simbólica a uma etapa anterior.
pub const SYMBOLIC_FIELDS: [&str; 5] = ["id", "name", "company_id", "code", "status"];

/// Referência simbólica encontrada nos argumentos de uma etapa.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolicReference {
    /// Identificador da chamada que produz o valor.
    pub tool_call_id: String,
    /// Campo do resultado que deve ser substituído.
    pub field: String,
}

/// Extrai referências `@<tool_call_id>.<campo>` de qualquer string dos argumentos.
pub fn symbolic_references(value: &Value) -> Vec<SymbolicReference> {
    let mut found = Vec::new();
    collect_references(value, &mut found);
    found.sort_by(|a, b| {
        a.tool_call_id
            .cmp(&b.tool_call_id)
            .then_with(|| a.field.cmp(&b.field))
    });
    found.dedup();
    found
}

fn collect_references(value: &Value, output: &mut Vec<SymbolicReference>) {
    match value {
        Value::String(text) => {
            let Some(rest) = text.strip_prefix('@') else {
                return;
            };
            let Some((call_id, field)) = rest.split_once('.') else {
                return;
            };
            if !call_id.is_empty()
                && !field.is_empty()
                && SYMBOLIC_FIELDS.contains(&field)
                && call_id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            {
                output.push(SymbolicReference {
                    tool_call_id: call_id.to_owned(),
                    field: field.to_owned(),
                });
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_references(item, output);
            }
        }
        Value::Object(map) => {
            for item in map.values() {
                collect_references(item, output);
            }
        }
        _ => {}
    }
}

/// Substitui referências simbólicas pelos resultados reais já executados.
pub fn resolve_arguments(arguments: &str, results: &HashMap<String, Value>) -> Result<String> {
    let mut value: Value = serde_json::from_str(arguments)
        .map_err(|_| TinaError::ToolError("argumentos inválidos para resolução".into()))?;
    replace_references(&mut value, results)?;
    serde_json::to_string(&value)
        .map_err(|_| TinaError::ToolError("argumentos resolvidos inválidos".into()))
}

fn replace_references(value: &mut Value, results: &HashMap<String, Value>) -> Result<()> {
    match value {
        Value::String(text) => {
            let Some(rest) = text.strip_prefix('@') else {
                return Ok(());
            };
            let Some((call_id, field)) = rest.split_once('.') else {
                return Ok(());
            };
            if !SYMBOLIC_FIELDS.contains(&field) {
                return Ok(());
            }
            let result = results.get(call_id).ok_or_else(|| {
                TinaError::ToolError(format!(
                    "referência simbólica sem resultado autorizado: {call_id}"
                ))
            })?;
            let replacement = result.get(field).and_then(Value::as_str).ok_or_else(|| {
                TinaError::ToolError(format!(
                    "campo {field} indisponível no resultado de {call_id}"
                ))
            })?;
            *text = replacement.to_owned();
            Ok(())
        }
        Value::Array(items) => {
            for item in items {
                replace_references(item, results)?;
            }
            Ok(())
        }
        Value::Object(map) => {
            for item in map.values_mut() {
                replace_references(item, results)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn text_argument(args: &Value, field: &str) -> Option<String> {
    args.get(field)
        .and_then(Value::as_str)
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
}

/// Regras fixas do backend: dependências que o modelo não pode suprimir.
pub(crate) fn backend_dependencies(
    call: &ToolCall,
    index: usize,
    all: &[ToolCall],
) -> BTreeSet<String> {
    let mut deps = BTreeSet::new();
    let args = serde_json::from_str::<Value>(&call.arguments).unwrap_or(Value::Null);
    let organization_kind = text_argument(&args, "organization_kind");
    let company_name = text_argument(&args, "company_name");
    let department_name = text_argument(&args, "department_name");
    let position_name = text_argument(&args, "position_name");

    for (other_index, other) in all.iter().enumerate().take(index) {
        if other_index == index {
            continue;
        }
        let other_args = serde_json::from_str::<Value>(&other.arguments).unwrap_or(Value::Null);
        let same_org = organization_kind.is_some()
            && organization_kind == text_argument(&other_args, "organization_kind");
        match call.name.as_str() {
            "create_department" if other.name == "create_organization" => {
                let same_name = same_org
                    && text_argument(&other_args, "name").is_some()
                    && company_name.as_ref().is_none_or(|requested| {
                        text_argument(&other_args, "name").as_ref() == Some(requested)
                    });
                if same_name {
                    deps.insert(other.id.clone());
                }
            }
            "create_position" if other.name == "create_department" => {
                if same_org
                    && company_name.is_some()
                    && company_name == text_argument(&other_args, "company_name")
                    && department_name.is_some()
                    && department_name == text_argument(&other_args, "name")
                {
                    deps.insert(other.id.clone());
                }
            }
            "generate_contract" if other.name == "create_position" => {
                if same_org
                    && company_name.is_some()
                    && company_name == text_argument(&other_args, "company_name")
                    && department_name.is_some()
                    && department_name == text_argument(&other_args, "department_name")
                    && position_name.is_some()
                    && position_name == text_argument(&other_args, "name")
                {
                    deps.insert(other.id.clone());
                }
            }
            _ => {}
        }
    }
    deps
}

/// Ordena as etapas topologicamente, preservando a ordem recebida como desempate.
pub fn topological_order(steps: Vec<PreparedStep>) -> Result<Vec<PreparedStep>> {
    let by_id: BTreeMap<String, usize> = steps
        .iter()
        .enumerate()
        .map(|(index, step)| (step.tool_call_id.clone(), index))
        .collect();
    let mut indegree: Vec<usize> = vec![0; steps.len()];
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); steps.len()];
    for (index, step) in steps.iter().enumerate() {
        for dep in &step.depends_on {
            let dep_index = by_id
                .get(dep)
                .ok_or_else(|| TinaError::ToolError(format!("dependência desconhecida: {dep}")))?;
            if *dep_index >= index {
                return Err(TinaError::ToolError(
                    "dependência deve apontar para etapa anterior".into(),
                ));
            }
            indegree[index] = indegree[index].saturating_add(1);
            dependents[*dep_index].push(index);
        }
    }
    let mut ready: Vec<usize> = (0..steps.len()).filter(|i| indegree[*i] == 0).collect();
    let mut ordered = Vec::with_capacity(steps.len());
    while let Some(next) = ready
        .iter()
        .copied()
        .min_by_key(|index| steps[*index].original_index)
    {
        ready.retain(|candidate| candidate != &next);
        ordered.push(next);
        for dependent in &dependents[next] {
            indegree[*dependent] = indegree[*dependent].saturating_sub(1);
            if indegree[*dependent] == 0 {
                ready.push(*dependent);
            }
        }
    }
    if ordered.len() != steps.len() {
        return Err(TinaError::ToolError(
            "ciclo de dependências detectado".into(),
        ));
    }
    Ok(ordered
        .into_iter()
        .enumerate()
        .map(|(position, index)| {
            let mut step = steps[index].clone();
            step.step_index = position;
            step
        })
        .collect())
}

/// Valida alvos e dependências contra o estado real do banco, sem executar nada.
pub async fn validate_against_database(
    pool: &SqlitePool,
    installation: &str,
    steps: &[PreparedStep],
) -> Result<()> {
    let catalog: Vec<String> = crate::infra::chat_tools::definitions()
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|definition| {
            definition
                .pointer("/function/name")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .collect();
    for step in steps {
        if !catalog.iter().any(|name| name == &step.tool_name) {
            return Err(TinaError::ToolError(format!(
                "ferramenta não catalogada: {}",
                step.tool_name
            )));
        }
        let args: Value = serde_json::from_str(&step.arguments_json)
            .map_err(|_| TinaError::ToolError("argumentos inválidos".into()))?;
        if args
            .get("reason")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            return Err(TinaError::ToolError(
                "toda ação proposta exige justificativa".into(),
            ));
        }
        if !step.depends_on.is_empty() {
            continue;
        }
        match step.tool_name.as_str() {
            "create_department" => {
                let kind = text_argument(&args, "organization_kind").unwrap_or_default();
                let name = text_argument(&args, "company_name");
                let rows: Vec<String> = sqlx::query_scalar(
                    "SELECT name FROM companies WHERE installation_id=? AND kind=? AND status='active'",
                )
                .bind(installation)
                .bind(&kind)
                .fetch_all(pool)
                .await?;
                let matches = rows
                    .iter()
                    .filter(|row| {
                        name.as_ref()
                            .is_none_or(|wanted| row.to_lowercase() == *wanted)
                    })
                    .count();
                if matches > 1 || (matches == 0 && kind == "business") {
                    return Err(TinaError::ToolError(
                        "organização de destino ambígua ou inexistente".into(),
                    ));
                }
            }
            "create_position" | "generate_contract" => {
                let kind = text_argument(&args, "organization_kind").unwrap_or_default();
                let company = text_argument(&args, "company_name").unwrap_or_default();
                let department = text_argument(&args, "department_name").unwrap_or_default();
                let query = if step.tool_name == "create_position" {
                    "SELECT d.id FROM departments d JOIN companies c ON c.id=d.company_id \
                     WHERE c.installation_id=? AND c.kind=? AND c.status='active' AND d.status='active' \
                       AND lower(c.name)=? AND lower(d.name)=?"
                } else {
                    "SELECT p.id FROM positions p JOIN departments d ON d.id=p.department_id \
                     JOIN companies c ON c.id=d.company_id \
                     WHERE c.installation_id=? AND c.kind=? AND c.status='active' AND d.status='active' \
                       AND p.status='active' AND lower(c.name)=? AND lower(d.name)=?"
                };
                let rows: Vec<String> = sqlx::query_scalar(query)
                    .bind(installation)
                    .bind(&kind)
                    .bind(&company)
                    .bind(&department)
                    .fetch_all(pool)
                    .await?;
                if rows.len() != 1 {
                    return Err(TinaError::ToolError(
                        "destino organizacional ambíguo ou inexistente".into(),
                    ));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Verifica se um plano recém-chegado é idêntico ao plano autorizado anterior.
pub fn materially_expanded(authorized: &[PreparedStep], incoming: &[ToolCall]) -> bool {
    if authorized.len() != incoming.len() {
        return true;
    }
    authorized
        .iter()
        .zip(incoming)
        .any(|(step, call)| step.tool_name != call.name || step.arguments_json != call.arguments)
}
