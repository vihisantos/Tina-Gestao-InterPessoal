//! Commands Tauri para gerenciamento de departamentos

use serde::{Deserialize, Serialize};
use tauri::State;
use tina::TinaApp;

#[derive(Debug, Serialize, Clone)]
pub struct DepartmentDto {
    pub id: String,
    pub company_id: String,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub created_at: String,
}

impl From<tina::core::Department> for DepartmentDto {
    fn from(dept: tina::core::Department) -> Self {
        Self {
            id: dept.id,
            company_id: dept.company_id,
            code: dept.code,
            name: dept.name,
            description: dept.description,
            status: dept.status.to_string(),
            created_at: dept.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateDepartmentRequest {
    pub company_id: String,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDepartmentRequest {
    pub name: String,
    pub description: Option<String>,
}

fn get_installation_id(state: &TinaApp) -> String {
    state.config().installation_id.clone()
}

/// Lista departamentos de uma empresa
#[tauri::command]
pub async fn list_departments(
    state: State<'_, TinaApp>,
    company_id: String,
) -> Result<Vec<DepartmentDto>, String> {
    let db = state
        .owner_database(tina::core::authorization::Operation::Read)
        .map_err(|e| e.to_string())?;
    let dept_repo = db.departments();
    let company_repo = db.companies();
    let installation_id = get_installation_id(&state);

    // Verificar se a empresa pertence à instalação
    if !company_repo
        .belongs_to_installation(&company_id, &installation_id)
        .await
        .map_err(|e| e.to_string())?
    {
        return Err("Empresa não pertence à instalação atual".to_string());
    }

    let depts = dept_repo
        .list_by_company(&company_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(depts.into_iter().map(DepartmentDto::from).collect())
}

/// Busca departamento pelo ID (apenas se pertencer à instalação)
#[tauri::command]
pub async fn get_department(
    state: State<'_, TinaApp>,
    department_id: String,
) -> Result<DepartmentDto, String> {
    let db = state
        .owner_database(tina::core::authorization::Operation::Read)
        .map_err(|e| e.to_string())?;
    let dept_repo = db.departments();
    let installation_id = get_installation_id(&state);

    let dept = dept_repo
        .find_by_id(&department_id)
        .await
        .map_err(|e| e.to_string())?;

    // Verificar se pertence à instalação (via cadeia empresa → departamento)
    let company_repo = db.companies();
    let companies = company_repo
        .list_by_installation(&installation_id)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|c| c.id)
        .collect::<Vec<_>>();

    let mut found = false;
    for company_id in companies {
        if dept_repo
            .belongs_to_company(&department_id, &company_id)
            .await
            .map_err(|e| e.to_string())?
        {
            found = true;
            break;
        }
    }

    if !found {
        return Err("Departamento não pertence à instalação atual".to_string());
    }

    Ok(DepartmentDto::from(dept))
}

/// Cria um novo departamento (com auditoria de autoria transacional)
#[tauri::command]
pub async fn create_department(
    state: State<'_, TinaApp>,
    request: CreateDepartmentRequest,
) -> Result<DepartmentDto, String> {
    let dept = state
        .owner_create_department(
            &request.company_id,
            &request.code,
            &request.name,
            request.description.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(DepartmentDto::from(dept))
}

/// Atualiza o nome do departamento (com auditoria de autoria transacional)
#[tauri::command]
pub async fn update_department(
    state: State<'_, TinaApp>,
    department_id: String,
    request: UpdateDepartmentRequest,
) -> Result<(), String> {
    state
        .owner_update_department_name(&department_id, &request.name)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Archiva um departamento (não pode ter cargos ativos) - com auditoria de autoria transacional
#[tauri::command]
pub async fn archive_department(
    state: State<'_, TinaApp>,
    department_id: String,
) -> Result<(), String> {
    state.owner_archive_department(&department_id).await.map_err(|e| e.to_string())
}
