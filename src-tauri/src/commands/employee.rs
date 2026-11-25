//! Commands Tauri para gerenciamento de funcionários (employees)
//!
//! Toda validação ocorre no backend Rust. Não confiar em IDs enviados pelo
//! frontend como prova de autorização. Validar a cadeia
//! funcionário → cargo → departamento → empresa → instalação.

use serde::{Deserialize, Serialize};
use sqlx;
use tauri::State;
use tina::infra::HierarchyService;
use tina::TinaApp;

#[derive(Debug, Serialize, Clone)]
pub struct EmployeeDto {
    pub id: String,
    pub name: String,
    pub position_id: String,
    pub responsibilities: Option<String>,
    pub supervisor_id: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub deactivated_at: Option<String>,
}

impl From<tina::core::Employee> for EmployeeDto {
    fn from(employee: tina::core::Employee) -> Self {
        Self {
            id: employee.id,
            name: employee.name,
            position_id: employee.position_id,
            responsibilities: employee.responsibilities,
            supervisor_id: employee.supervisor_id,
            status: employee.status.to_string(),
            created_at: employee.created_at,
            updated_at: employee.updated_at,
            deactivated_at: employee.deactivated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateEmployeeRequest {
    pub position_id: String,
    pub name: String,
    pub responsibilities: Option<String>,
    pub supervisor_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEmployeeRequest {
    pub name: String,
    pub responsibilities: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateResponsibilitiesRequest {
    pub responsibilities: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChangePositionRequest {
    pub position_id: String,
    /// Motivo opcional da movimentação, registrado no histórico.
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AssignSupervisorRequest {
    pub supervisor_id: Option<String>,
    /// Motivo opcional da alteração, registrado no histórico.
    pub reason: Option<String>,
}

fn get_installation_id(state: &TinaApp) -> String {
    state.config().installation_id.clone()
}

/// Cria um novo funcionário associado a um cargo existente da instalação (com auditoria de autoria transacional)
#[tauri::command]
pub async fn create_employee(
    state: State<'_, TinaApp>,
    request: CreateEmployeeRequest,
) -> Result<EmployeeDto, String> {
    let employee = state
        .owner_create_employee(
            &request.position_id,
            &request.name,
            request.responsibilities.as_deref(),
            request.supervisor_id.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(EmployeeDto::from(employee))
}

/// Busca funcionário pelo ID (apenas se pertencer à instalação atual)
#[tauri::command]
pub async fn get_employee(
    state: State<'_, TinaApp>,
    employee_id: String,
) -> Result<EmployeeDto, String> {
    let db = state
        .owner_database(tina::core::authorization::Operation::Read)
        .map_err(|e| e.to_string())?;
    let repo = db.employees();
    let installation_id = get_installation_id(&state);

    let employee = repo
        .find_by_id(&employee_id)
        .await
        .map_err(|e| e.to_string())?;

    if !repo
        .belongs_to_installation(&employee_id, &installation_id)
        .await
        .map_err(|e| e.to_string())?
    {
        return Err("Funcionário não pertence à instalação atual".to_string());
    }

    Ok(EmployeeDto::from(employee))
}

/// Lista funcionários de uma empresa (apenas ativos, com cargos e departamentos ativos)
#[tauri::command]
pub async fn list_employees_by_company(
    state: State<'_, TinaApp>,
    company_id: String,
) -> Result<Vec<EmployeeDto>, String> {
    let db = state
        .owner_database(tina::core::authorization::Operation::Read)
        .map_err(|e| e.to_string())?;
    let repo = db.employees();
    let installation_id = get_installation_id(&state);

    // Validar que a empresa pertence à instalação
    if !company_belongs_to_installation(&db, &company_id, &installation_id).await? {
        return Err("Empresa não pertence à instalação atual".to_string());
    }

    let employees = repo
        .list_by_company(&company_id, false)
        .await
        .map_err(|e| e.to_string())?;

    Ok(employees.into_iter().map(EmployeeDto::from).collect())
}

/// Lista funcionários de um departamento (apenas ativos, com cargos ativos)
#[tauri::command]
pub async fn list_employees_by_department(
    state: State<'_, TinaApp>,
    department_id: String,
) -> Result<Vec<EmployeeDto>, String> {
    let db = state
        .owner_database(tina::core::authorization::Operation::Read)
        .map_err(|e| e.to_string())?;
    let repo = db.employees();
    let installation_id = get_installation_id(&state);

    // Validar que o departamento pertence à instalação (via cadeia empresa → instalação)
    if !department_belongs_to_installation(&db, &department_id, &installation_id).await? {
        return Err("Departamento não pertence à instalação atual".to_string());
    }

    let employees = repo
        .list_by_department(&department_id, false)
        .await
        .map_err(|e| e.to_string())?;

    Ok(employees.into_iter().map(EmployeeDto::from).collect())
}

/// Atualiza nome e responsabilidades do funcionário (com auditoria de autoria transacional)
#[tauri::command]
pub async fn update_employee(
    state: State<'_, TinaApp>,
    employee_id: String,
    request: UpdateEmployeeRequest,
) -> Result<(), String> {
    state
        .owner_update_employee_info(
            &employee_id,
            &request.name,
            request.responsibilities.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Atualiza apenas as responsabilidades do funcionário (com auditoria de autoria transacional)
#[tauri::command]
pub async fn update_employee_responsibilities(
    state: State<'_, TinaApp>,
    employee_id: String,
    request: UpdateResponsibilitiesRequest,
) -> Result<(), String> {
    state
        .owner_update_employee_responsibilities(
            &employee_id,
            request.responsibilities.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Altera o cargo do funcionário (respeitando consistência: mesmo empresa)
#[tauri::command]
pub async fn change_employee_position(
    state: State<'_, TinaApp>,
    employee_id: String,
    request: ChangePositionRequest,
) -> Result<(), String> {
    let db = state
        .owner_database(tina::core::authorization::Operation::Write)
        .map_err(|e| e.to_string())?;
    let repo = db.employees();
    let installation_id = get_installation_id(&state);

    let company_id = get_employee_company_id(&db, &employee_id, &installation_id).await?;

    repo.change_position_with_reason(
        &employee_id,
        &company_id,
        &request.position_id,
        request.reason.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Associa (ou remove) um superior direto ao funcionário.
///
/// Validação de empresa, estado ativo e ciclos, além do registro de histórico,
/// acontece no `HierarchyService` (backend). O frontend não é fonte de
/// autorização; a empresa é derivada do funcionário na instalação atual.
#[tauri::command]
pub async fn assign_employee_supervisor(
    state: State<'_, TinaApp>,
    employee_id: String,
    request: AssignSupervisorRequest,
) -> Result<(), String> {
    state
        .owner_update_employee_supervisor(
            &employee_id,
            request.supervisor_id.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Desativa um funcionário (preserva histórico, não exclui fisicamente) - com auditoria de autoria transacional
#[tauri::command]
pub async fn deactivate_employee(
    state: State<'_, TinaApp>,
    employee_id: String,
) -> Result<(), String> {
    state.owner_deactivate_employee(&employee_id).await.map_err(|e| e.to_string())
}

/// Reativa um funcionário previamente desativado - com auditoria de autoria transacional
#[tauri::command]
pub async fn reactivate_employee(
    state: State<'_, TinaApp>,
    employee_id: String,
) -> Result<(), String> {
    state.owner_reactivate_employee(&employee_id).await.map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Funções auxiliares de validação (backend apenas)
// ---------------------------------------------------------------------------

struct PositionCompanyInfo {
    company_id: String,
}

/// Valida que o cargo existe, está ativo e pertence à instalação.
/// Retorna o company_id do cargo.
async fn get_position_company_and_validate(
    db: &tina::AuthorizedOwnerDatabase<'_>,
    position_id: &str,
    installation_id: &str,
) -> Result<PositionCompanyInfo, String> {
    let row: Option<(String, String)> = sqlx::query_as(
        r#"
        SELECT c.id, c.installation_id
        FROM positions p
        JOIN departments d ON p.department_id = d.id
        JOIN companies c ON d.company_id = c.id
        WHERE p.id = ? AND p.status = 'active' AND d.status = 'active' AND c.status = 'active'
        "#,
    )
    .bind(position_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| e.to_string())?;

    let (company_id, inst_id) = row
        .ok_or_else(|| "Cargo não encontrado, inativo, ou não pertence à instalação".to_string())?;

    if inst_id != installation_id {
        return Err("Cargo não pertence à instalação atual".to_string());
    }

    Ok(PositionCompanyInfo { company_id })
}

/// Valida que a empresa pertence à instalação
async fn company_belongs_to_installation(
    db: &tina::AuthorizedOwnerDatabase<'_>,
    company_id: &str,
    installation_id: &str,
) -> Result<bool, String> {
    let repo = db.companies();
    repo.belongs_to_installation(company_id, installation_id)
        .await
        .map_err(|e| e.to_string())
}

/// Valida que o departamento pertence à instalação (via cadeia empresa → instalação)
async fn department_belongs_to_installation(
    db: &tina::AuthorizedOwnerDatabase<'_>,
    department_id: &str,
    installation_id: &str,
) -> Result<bool, String> {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM departments d
        JOIN companies c ON d.company_id = c.id
        WHERE d.id = ? AND d.status = 'active' AND c.installation_id = ? AND c.status = 'active'
        "#,
    )
    .bind(department_id)
    .bind(installation_id)
    .fetch_one(db.pool())
    .await
    .map_err(|e| e.to_string())?;

    Ok(count > 0)
}

/// Obtém o company_id de um funcionário via cadeia completa, validando instalação
async fn get_employee_company_id(
    db: &tina::AuthorizedOwnerDatabase<'_>,
    employee_id: &str,
    installation_id: &str,
) -> Result<String, String> {
    let row: Option<String> = sqlx::query_scalar(
        r#"
        SELECT c.id
        FROM employees e
        JOIN positions p ON e.position_id = p.id
        JOIN departments d ON p.department_id = d.id
        JOIN companies c ON d.company_id = c.id
        WHERE e.id = ? AND c.installation_id = ? AND c.status = 'active'
        "#,
    )
    .bind(employee_id)
    .bind(installation_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| e.to_string())?;

    row.ok_or_else(|| "Funcionário não encontrado ou não pertence à instalação atual".to_string())
}
