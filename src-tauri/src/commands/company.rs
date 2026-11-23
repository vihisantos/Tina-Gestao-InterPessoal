//! Commands Tauri para gerenciamento de empresas

use serde::{Deserialize, Serialize};
use tauri::State;
use tina::infra::CompanyRepository;
use tina::TinaApp;

#[derive(Debug, Serialize, Clone)]
pub struct CompanyDto {
    pub id: String,
    pub installation_id: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
}

impl From<tina::core::Company> for CompanyDto {
    fn from(company: tina::core::Company) -> Self {
        Self {
            id: company.id,
            installation_id: company.installation_id,
            name: company.name,
            status: company.status.to_string(),
            created_at: company.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateCompanyRequest {
    pub name: String,
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
    let repo = CompanyRepository::new(db.pool());
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
    let repo = CompanyRepository::new(db.pool());
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

/// Cria uma nova empresa (o installation_id é obtido automaticamente do contexto)
#[tauri::command]
pub async fn create_company(
    state: State<'_, TinaApp>,
    request: CreateCompanyRequest,
) -> Result<CompanyDto, String> {
    let db = state
        .owner_database(tina::core::authorization::Operation::Write)
        .map_err(|e| e.to_string())?;
    let repo = CompanyRepository::new(db.pool());
    let installation_id = get_installation_id(&state);

    // O installation_id é obtido automaticamente do contexto confiável
    // NÃO confiar no frontend para fornecer esse identificador
    let company = repo
        .create(&installation_id, &request.name)
        .await
        .map_err(|e| e.to_string())?;

    Ok(CompanyDto::from(company))
}

/// Atualiza o nome da empresa
#[tauri::command]
pub async fn update_company(
    state: State<'_, TinaApp>,
    company_id: String,
    request: UpdateCompanyRequest,
) -> Result<(), String> {
    let db = state
        .owner_database(tina::core::authorization::Operation::Write)
        .map_err(|e| e.to_string())?;
    let repo = CompanyRepository::new(db.pool());
    let installation_id = get_installation_id(&state);

    repo.update_name(&company_id, &installation_id, &request.name)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Archiva uma empresa (não pode ter departamentos ativos)
#[tauri::command]
pub async fn archive_company(state: State<'_, TinaApp>, company_id: String) -> Result<(), String> {
    let db = state
        .owner_database(tina::core::authorization::Operation::Write)
        .map_err(|e| e.to_string())?;
    let repo = CompanyRepository::new(db.pool());
    let installation_id = get_installation_id(&state);

    // Verificar pertencimento à instalação (escopo aplicado no SQL)
    if !repo
        .can_archive_for_installation(&company_id, &installation_id)
        .await
        .map_err(|e| e.to_string())?
    {
        return Err(
            "Empresa não pode ser arquivada: ainda possui departamentos ativos ou não pertence à instalação".to_string(),
        );
    }

    repo.archive(&company_id, &installation_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
