//! Commands Tauri para gerenciamento de empresas

use serde::{Deserialize, Serialize};
use tauri::State;
use tina::TinaApp;

#[derive(Debug, Serialize, Clone)]
pub struct CompanyDto {
    pub id: String,
    pub installation_id: String,
    pub name: String,
    pub kind: String,
    pub status: String,
    pub created_at: String,
}

impl From<tina::core::Company> for CompanyDto {
    fn from(company: tina::core::Company) -> Self {
        Self {
            id: company.id,
            installation_id: company.installation_id,
            name: company.name,
            kind: company.kind,
            status: company.status.to_string(),
            created_at: company.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateCompanyRequest {
    pub name: String,
    pub kind: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCompanyRequest {
    pub name: String,
}

fn get_installation_id(state: &TinaApp) -> String {
    state.config().installation_id.clone()
}

/// Lista todas as empresas da instalação atual
#[tauri::command]
pub async fn list_companies(state: State<'_, TinaApp>) -> Result<Vec<CompanyDto>, String> {
    let db = state
        .owner_database(tina::core::authorization::Operation::Read)
        .map_err(|e| e.to_string())?;
    let repo = db.companies();
    let installation_id = get_installation_id(&state);

    let companies = repo
        .list_by_installation(&installation_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(companies.into_iter().map(CompanyDto::from).collect())
}

/// Busca empresa pelo ID (apenas se pertencer à instalação atual)
#[tauri::command]
pub async fn get_company(
    state: State<'_, TinaApp>,
    company_id: String,
) -> Result<CompanyDto, String> {
    let db = state
        .owner_database(tina::core::authorization::Operation::Read)
        .map_err(|e| e.to_string())?;
    let repo = db.companies();
    let installation_id = get_installation_id(&state);

    let company = repo
        .find_by_id(&company_id)
        .await
        .map_err(|e| e.to_string())?;

    if !repo
        .belongs_to_installation(&company.id, &installation_id)
        .await
        .map_err(|e| e.to_string())?
    {
        return Err("Empresa não pertence à instalação atual".to_string());
    }

    Ok(CompanyDto::from(company))
}

/// Cria uma nova empresa (o installation_id é obtido automaticamente do contexto confiável)
#[tauri::command]
pub async fn create_company(
    state: State<'_, TinaApp>,
    request: CreateCompanyRequest,
) -> Result<CompanyDto, String> {
    let company = state
        .owner_create_company_with_kind(&request.name, request.kind.as_deref().unwrap_or("business"))
        .await
        .map_err(|e| e.to_string())?;

    Ok(CompanyDto::from(company))
}

/// Atualiza o nome da empresa (com auditoria de autoria transacional)
#[tauri::command]
pub async fn update_company(
    state: State<'_, TinaApp>,
    company_id: String,
    request: UpdateCompanyRequest,
) -> Result<(), String> {
    state
        .owner_update_company_name(&company_id, &request.name)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Archiva uma empresa (não pode ter departamentos ativos) - com auditoria de autoria transacional
#[tauri::command]
pub async fn archive_company(state: State<'_, TinaApp>, company_id: String) -> Result<(), String> {
    state
        .owner_archive_company(&company_id)
        .await
        .map_err(|e| e.to_string())
}
