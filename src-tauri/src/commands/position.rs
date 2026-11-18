//! Commands Tauri para gerenciamento de cargos (positions)

use serde::{Deserialize, Serialize};
use tauri::State;
use tina::TinaApp;
use tina::infra::{PositionRepository, DepartmentRepository, CompanyRepository, InstallationRepository};

#[derive(Debug, Serialize, Clone)]
pub struct PositionDto {
    pub id: String,
    pub department_id: String,
    pub name: String,
    pub level: i32,
    pub authority_level: i32,
    pub description: Option<String>,
    pub status: String,
    pub created_at: String,
}

impl From<tina::core::Position> for PositionDto {
    fn from(pos: tina::core::Position) -> Self {
        Self {
            id: pos.id,
            department_id: pos.department_id,
            name: pos.name,
            level: pos.level,
            authority_level: pos.authority_level,
            description: pos.description,
            status: pos.status.to_string(),
            created_at: pos.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreatePositionRequest {
    pub department_id: String,
    pub name: String,
    pub level: i32,
    pub authority_level: i32,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePositionRequest {
    pub name: String,
    pub description: Option<String>,
}

fn get_installation_id(state: &TinaApp) -> String {
    state.config().installation_id.clone()
}

/// Lista cargos de um departamento
#[tauri::command]
pub async fn list_positions(
    state: State<'_, TinaApp>,
    department_id: String,
) -> Result<Vec<PositionDto>, String> {
    let db = state.database();
    let pos_repo = PositionRepository::new(db.pool());
    let dept_repo = DepartmentRepository::new(db.pool());
    let installation_id = get_installation_id(&state);
    
    // Verificar se o departamento pertence à instalação (via cadeia)
    let dept = dept_repo.find_by_id(&department_id).await
        .map_err(|e| e.to_string())?;
    
    let company_repo = CompanyRepository::new(db.pool());
    let companies = company_repo.list_by_installation(&installation_id).await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|c| c.id)
        .collect::<Vec<_>>();
    
    let mut found = false;
    for company_id in companies {
        if dept_repo.belongs_to_company(&department_id, &company_id).await
            .map_err(|e| e.to_string())? {
            found = true;
            break;
        }
    }
    
    if !found {
        return Err("Departamento não pertence à instalação atual".to_string());
    }
    
    let positions = pos_repo.list_by_department(&department_id).await
        .map_err(|e| e.to_string())?;
    
    Ok(positions.into_iter().map(PositionDto::from).collect())
}

/// Busca cargo pelo ID (apenas se pertencer à instalação)
#[tauri::command]
pub async fn get_position(
    state: State<'_, TinaApp>,
    position_id: String,
) -> Result<PositionDto, String> {
    let db = state.database();
    let pos_repo = PositionRepository::new(db.pool());
    let dept_repo = DepartmentRepository::new(db.pool());
    let company_repo = CompanyRepository::new(db.pool());
    let installation_id = get_installation_id(&state);
    
    let pos = pos_repo.find_by_id(&position_id).await
        .map_err(|e| e.to_string())?;
    
    // Verificar se alguma empresa da instalação tem este cargo (via cadeia)
    let companies = company_repo.list_by_installation(&installation_id).await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|c| c.id)
        .collect::<Vec<_>>();
    
    let mut found = false;
    for company_id in companies {
        if pos_repo.belongs_to_company(&position_id, &company_id).await
            .map_err(|e| e.to_string())? {
            found = true;
            break;
        }
    }
    
    if !found {
        return Err("Cargo não pertence à instalação atual".to_string());
    }
    
    Ok(PositionDto::from(pos))
}

/// Cria um novo cargo
#[tauri::command]
pub async fn create_position(
    state: State<'_, TinaApp>,
    request: CreatePositionRequest,
) -> Result<PositionDto, String> {
    let db = state.database();
    let pos_repo = PositionRepository::new(db.pool());
    let dept_repo = DepartmentRepository::new(db.pool());
    let installation_id = get_installation_id(&state);
    
    // Verificar se o departamento pertence à instalação (via cadeia)
    let dept = dept_repo.find_by_id(&request.department_id).await
        .map_err(|e| e.to_string())?;
    
    let company_repo = CompanyRepository::new(db.pool());
    let companies = company_repo.list_by_installation(&installation_id).await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|c| c.id)
        .collect::<Vec<_>>();
    
    let mut found = false;
    for company_id in companies {
        if dept_repo.belongs_to_company(&request.department_id, &company_id).await
            .map_err(|e| e.to_string())? {
            found = true;
            break;
        }
    }
    
    if !found {
        return Err("Departamento não pertence à instalação atual".to_string());
    }
    
    let position = pos_repo.create(
        &request.department_id,
        &request.name,
        request.level,
        request.authority_level,
        request.description.as_deref(),
    ).await
    .map_err(|e| e.to_string())?;
    
    Ok(PositionDto::from(position))
}

/// Atualiza o nome e descrição do cargo
#[tauri::command]
pub async fn update_position(
    state: State<'_, TinaApp>,
    position_id: String,
    request: UpdatePositionRequest,
) -> Result<(), String> {
    let db = state.database();
    let pos_repo = PositionRepository::new(db.pool());
    let installation_id = get_installation_id(&state);
    
    // Verificar pertencimento à instalação
    let pos = pos_repo.find_by_id(&position_id).await
        .map_err(|e| e.to_string())?;
    
    let dept_repo = DepartmentRepository::new(db.pool());
    let company_repo = CompanyRepository::new(db.pool());
    
    let companies = company_repo.list_by_installation(&installation_id).await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|c| c.id)
        .collect::<Vec<_>>();
    
    let mut found = false;
    for company_id in companies {
        if dept_repo.belongs_to_company(&pos.department_id, &company_id).await
            .map_err(|e| e.to_string())? {
            found = true;
            break;
        }
    }
    
    if !found {
        return Err("Cargo não pertence à instalação atual".to_string());
    }
    
    pos_repo.update_name(&position_id, &request.name).await
        .map_err(|e| e.to_string())?;
    
    pos_repo.update_description(&position_id, request.description.as_deref()).await
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

/// Archiva um cargo (não pode ter funcionários ativos)
#[tauri::command]
pub async fn archive_position(
    state: State<'_, TinaApp>,
    position_id: String,
) -> Result<(), String> {
    let db = state.database();
    let pos_repo = PositionRepository::new(db.pool());
    let installation_id = get_installation_id(&state);
    
    // Verificar pertencimento à instalação
    let pos = pos_repo.find_by_id(&position_id).await
        .map_err(|e| e.to_string())?;
    
    let dept_repo = DepartmentRepository::new(db.pool());
    let company_repo = CompanyRepository::new(db.pool());
    
    let companies = company_repo.list_by_installation(&installation_id).await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|c| c.id)
        .collect::<Vec<_>>();
    
    let mut found = false;
    for company_id in companies {
        if dept_repo.belongs_to_company(&pos.department_id, &company_id).await
            .map_err(|e| e.to_string())? {
            found = true;
            break;
        }
    }
    
    if !found {
        return Err("Cargo não pertence à instalação atual".to_string());
    }
    
    // Verificar se pode ser arquivado
    if !pos_repo.can_archive(&position_id).await
        .map_err(|e| e.to_string())? {
        return Err("Cargo não pode ser arquivado: ainda possui funcionários ativos".to_string());
    }
    
    pos_repo.archive(&position_id).await
        .map_err(|e| e.to_string())?;
    
    Ok(())
}