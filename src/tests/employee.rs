//! Testes de integração para cadastro e ciclo de vida de funcionários IA
//!
//! Executar com: cargo test employee_tests

use crate::infra::{
    CompanyRepository, DepartmentRepository, EmployeeRepository, InstallationRepository,
    PositionRepository,
};
use crate::Database;
use sqlx;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Helper para criar banco de teste único para cada teste
async fn setup_test_db(test_name: &str) -> (Database, String) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let path =
        std::env::temp_dir().join(format!("tina-employee-test-{}-{}.db", test_name, timestamp));
    let path_str = path.to_string_lossy().to_string();

    let db = Database::open(Path::new(&path))
        .await
        .expect("Failed to create test database");

    (db, path_str)
}

/// Helper para limpar banco de teste
fn cleanup_test_db(path: &String) {
    let _ = std::fs::remove_file(path);
}

/// Aplica o conteúdo de um arquivo de migration em um pool SQLite.
///
/// Usado para simular a atualização de uma instalação existente: o schema
/// antigo é criado primeiro e a migration seguinte é aplicada sobre ele.
async fn apply_migration_sql(pool: &sqlx::SqlitePool, sql: &str) {
    sqlx::raw_sql(sql)
        .execute(pool)
        .await
        .expect("Failed to apply migration SQL");
}

/// Cria instalação, empresa, departamento e cargo para uso nos testes.
/// Retorna (installation_id, company_id, department_id, position_id).
async fn setup_organization(db: &crate::Database) -> (String, String, String, String) {
    let repo = InstallationRepository::new(db.pool());
    let installation = repo
        .create("Employee Test Installation", "owner-employee-test")
        .await
        .expect("Failed to create installation");

    let company_repo = CompanyRepository::new(db.pool());
    let company = company_repo
        .create(&installation.id, "Empresa Teste Employee")
        .await
        .expect("Failed to create company");

    let dept_repo = DepartmentRepository::new(db.pool());
    let dept = dept_repo
        .create(&company.id, "DEPT-EMP", "Departamento Employee", None)
        .await
        .expect("Failed to create department");

    let pos_repo = PositionRepository::new(db.pool());
    let position = pos_repo
        .create(&dept.id, "Cargo Employee", 2, 500, None)
        .await
        .expect("Failed to create position");

    (installation.id, company.id, dept.id, position.id)
}

#[tokio::test]
async fn test_employee_create() {
    let test_name = "employee_create";
    let (db, path) = setup_test_db(test_name).await;

    let (_inst_id, company_id, dept_id, position_id) = setup_organization(&db).await;

    let employee_repo = EmployeeRepository::new(db.pool());
    let employee = employee_repo
        .create(
            &position_id,
            "Funcionário Teste",
            Some("Responsável por sistemas"),
            None,
        )
        .await
        .expect("Failed to create employee");

    assert_eq!(employee.name, "Funcionário Teste");
    assert_eq!(employee.position_id, position_id);
    assert!(employee.is_active());
    assert!(employee.deactivated_at().is_none());
    assert!(!employee.id.is_empty());

    // Validar pertencimento à empresa
    assert!(employee_repo
        .belongs_to_company(&employee.id, &company_id)
        .await
        .expect("Failed to check company membership"));

    // Validar que o cargo pertence ao departamento da empresa
    assert!(employee_repo
        .position_exists_and_active(&position_id)
        .await
        .expect("Query failed"));

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_employee_get() {
    let test_name = "employee_get";
    let (db, path) = setup_test_db(test_name).await;

    let (_inst_id, _company_id, _dept_id, position_id) = setup_organization(&db).await;

    let employee_repo = EmployeeRepository::new(db.pool());
    let employee = employee_repo
        .create(&position_id, "Funcionário Consulta", None, None)
        .await
        .expect("Failed to create employee");

    let found = employee_repo
        .find_by_id(&employee.id)
        .await
        .expect("Failed to find employee");

    assert_eq!(found.id, employee.id);
    assert_eq!(found.name, "Funcionário Consulta");
    assert!(found.is_active());

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_employee_list_by_company() {
    let test_name = "employee_list_company";
    let (db, path) = setup_test_db(test_name).await;

    let (_inst_id, company_id, _dept_id, position_id) = setup_organization(&db).await;

    let employee_repo = EmployeeRepository::new(db.pool());

    // Criar dois funcionários na mesma empresa
    employee_repo
        .create(&position_id, "Funcionário A", None, None)
        .await
        .expect("Failed to create employee A");
    employee_repo
        .create(&position_id, "Funcionário B", None, None)
        .await
        .expect("Failed to create employee B");

    let employees = employee_repo
        .list_by_company(&company_id, false)
        .await
        .expect("Failed to list employees by company");

    assert_eq!(employees.len(), 2);
    assert!(employees.iter().all(|e| e.is_active()));

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_employee_list_by_department() {
    let test_name = "employee_list_department";
    let (db, path) = setup_test_db(test_name).await;

    let (_inst_id, company_id, dept_id, position_id) = setup_organization(&db).await;

    let employee_repo = EmployeeRepository::new(db.pool());

    employee_repo
        .create(&position_id, "Funcionário Dept", None, None)
        .await
        .expect("Failed to create employee");

    let employees = employee_repo
        .list_by_department(&dept_id, false)
        .await
        .expect("Failed to list employees by department");

    assert_eq!(employees.len(), 1);
    assert_eq!(employees[0].name, "Funcionário Dept");

    // Validar que a empresa do departamento está correta
    assert!(employee_repo
        .belongs_to_company(&employees[0].id, &company_id)
        .await
        .expect("Failed to check company membership"));

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_employee_update_info() {
    let test_name = "employee_update";
    let (db, path) = setup_test_db(test_name).await;

    let (_inst_id, company_id, _dept_id, position_id) = setup_organization(&db).await;

    let employee_repo = EmployeeRepository::new(db.pool());
    let employee = employee_repo
        .create(&position_id, "Nome Original", Some("Resp original"), None)
        .await
        .expect("Failed to create employee");

    employee_repo
        .update_info(
            &employee.id,
            &company_id,
            "Nome Atualizado",
            Some("Resp atualizada"),
        )
        .await
        .expect("Failed to update employee");

    let updated = employee_repo
        .find_by_id(&employee.id)
        .await
        .expect("Failed to find updated employee");

    assert_eq!(updated.name, "Nome Atualizado");
    assert_eq!(updated.responsibilities(), Some("Resp atualizada"));
    // updated_at deve ter mudado (ou pelo menos não ser vazio)
    assert!(!updated.updated_at.is_empty());

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_employee_update_responsibilities() {
    let test_name = "employee_update_resp";
    let (db, path) = setup_test_db(test_name).await;

    let (_inst_id, company_id, _dept_id, position_id) = setup_organization(&db).await;

    let employee_repo = EmployeeRepository::new(db.pool());
    let employee = employee_repo
        .create(&position_id, "Funcionário Resp", None, None)
        .await
        .expect("Failed to create employee");

    employee_repo
        .update_responsibilities(&employee.id, &company_id, Some("Novas responsabilidades"))
        .await
        .expect("Failed to update responsibilities");

    let updated = employee_repo
        .find_by_id(&employee.id)
        .await
        .expect("Failed to find updated employee");

    assert_eq!(updated.responsibilities(), Some("Novas responsabilidades"));

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_employee_change_position() {
    let test_name = "employee_change_position";
    let (db, path) = setup_test_db(test_name).await;

    let (_inst_id, company_id, dept_id, position_id) = setup_organization(&db).await;

    // Criar um segundo departamento e cargo na mesma empresa
    let dept_repo = DepartmentRepository::new(db.pool());
    let dept2 = dept_repo
        .create(&company_id, "DEPT-EMP-2", "Departamento Employee 2", None)
        .await
        .expect("Failed to create department 2");

    let pos_repo = PositionRepository::new(db.pool());
    let position2 = pos_repo
        .create(&dept2.id, "Cargo Employee 2", 3, 600, None)
        .await
        .expect("Failed to create position 2");

    let employee_repo = EmployeeRepository::new(db.pool());
    let employee = employee_repo
        .create(&position_id, "Funcionário Mudança", None, None)
        .await
        .expect("Failed to create employee");

    // Alterar para o novo cargo (mesma empresa)
    employee_repo
        .change_position(&employee.id, &company_id, &position2.id)
        .await
        .expect("Failed to change position");

    let updated = employee_repo
        .find_by_id(&employee.id)
        .await
        .expect("Failed to find updated employee");

    assert_eq!(updated.position_id, position2.id);

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_employee_change_position_rejects_other_company() {
    let test_name = "employee_change_position_reject";
    let (db, path) = setup_test_db(test_name).await;

    let (_inst_id, company_id, _dept_id, position_id) = setup_organization(&db).await;

    // Criar uma segunda empresa na mesma instalação
    let repo = InstallationRepository::new(db.pool());
    let installation = repo
        .find_by_id(&_inst_id)
        .await
        .expect("Failed to find installation")
        .expect("Installation not found");

    let company_repo = CompanyRepository::new(db.pool());
    let company_b = company_repo
        .create(&installation.id, "Empresa B")
        .await
        .expect("Failed to create company B");

    let dept_repo = DepartmentRepository::new(db.pool());
    let dept_b = dept_repo
        .create(&company_b.id, "DEPT-B", "Dept B", None)
        .await
        .expect("Failed to create dept B");

    let pos_repo = PositionRepository::new(db.pool());
    let position_b = pos_repo
        .create(&dept_b.id, "Cargo B", 1, 100, None)
        .await
        .expect("Failed to create position B");

    let employee_repo = EmployeeRepository::new(db.pool());
    let employee = employee_repo
        .create(&position_id, "Funcionário", None, None)
        .await
        .expect("Failed to create employee");

    // Tentar alterar para cargo de outra empresa - deve falhar
    let result = employee_repo
        .change_position(&employee.id, &company_id, &position_b.id)
        .await;

    assert!(
        result.is_err(),
        "Should reject position from another company"
    );

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_employee_assign_supervisor() {
    let test_name = "employee_supervisor";
    let (db, path) = setup_test_db(test_name).await;

    let (_inst_id, company_id, dept_id, position_id) = setup_organization(&db).await;

    let employee_repo = EmployeeRepository::new(db.pool());

    // Criar supervisor
    let supervisor = employee_repo
        .create(&position_id, "Supervisor", None, None)
        .await
        .expect("Failed to create supervisor");

    // Criar funcionário subordinado
    let employee = employee_repo
        .create(&position_id, "Subordinado", None, None)
        .await
        .expect("Failed to create employee");

    // Associar superior
    employee_repo
        .assign_supervisor(&employee.id, &company_id, Some(&supervisor.id))
        .await
        .expect("Failed to assign supervisor");

    let updated = employee_repo
        .find_by_id(&employee.id)
        .await
        .expect("Failed to find updated employee");

    assert_eq!(updated.supervisor_id(), Some(supervisor.id.as_str()));

    // Listar subordinados do supervisor
    let subordinates = employee_repo
        .list_subordinates(&supervisor.id)
        .await
        .expect("Failed to list subordinates");

    assert_eq!(subordinates.len(), 1);
    assert_eq!(subordinates[0].id, employee.id);

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_employee_assign_self_supervisor_rejected() {
    let test_name = "employee_self_supervisor";
    let (db, path) = setup_test_db(test_name).await;

    let (_inst_id, company_id, _dept_id, position_id) = setup_organization(&db).await;

    let employee_repo = EmployeeRepository::new(db.pool());
    let employee = employee_repo
        .create(&position_id, "Funcionário", None, None)
        .await
        .expect("Failed to create employee");

    // Tentar auto-atribuir como supervisor - deve falhar
    let result = employee_repo
        .assign_supervisor(&employee.id, &company_id, Some(&employee.id))
        .await;

    assert!(result.is_err(), "Should reject self as supervisor");

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_employee_deactivate_and_reactivate() {
    let test_name = "employee_deactivate";
    let (db, path) = setup_test_db(test_name).await;

    let (_inst_id, company_id, _dept_id, position_id) = setup_organization(&db).await;

    let employee_repo = EmployeeRepository::new(db.pool());
    let employee = employee_repo
        .create(&position_id, "Funcionário Desativar", None, None)
        .await
        .expect("Failed to create employee");

    assert!(employee.is_active());
    assert!(employee.deactivated_at().is_none());

    // Desativar
    employee_repo
        .deactivate(&employee.id, &company_id)
        .await
        .expect("Failed to deactivate employee");

    let deactivated = employee_repo
        .find_by_id(&employee.id)
        .await
        .expect("Failed to find deactivated employee");

    assert!(!deactivated.is_active());
    assert!(deactivated.deactivated_at().is_some());
    assert!(!deactivated.deactivated_at().unwrap().is_empty());

    // Reativar
    employee_repo
        .reactivate(&employee.id, &company_id)
        .await
        .expect("Failed to reactivate employee");

    let reactivated = employee_repo
        .find_by_id(&employee.id)
        .await
        .expect("Failed to find reactivated employee");

    assert!(reactivated.is_active());
    assert!(reactivated.deactivated_at().is_none());

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_employee_deactivate_already_inactive_rejected() {
    let test_name = "employee_deactivate_twice";
    let (db, path) = setup_test_db(test_name).await;

    let (_inst_id, company_id, _dept_id, position_id) = setup_organization(&db).await;

    let employee_repo = EmployeeRepository::new(db.pool());
    let employee = employee_repo
        .create(&position_id, "Funcionário", None, None)
        .await
        .expect("Failed to create employee");

    employee_repo
        .deactivate(&employee.id, &company_id)
        .await
        .expect("Failed to deactivate employee");

    // Tentar desativar novamente - deve falhar
    let result = employee_repo.deactivate(&employee.id, &company_id).await;
    assert!(
        result.is_err(),
        "Should reject deactivating already inactive employee"
    );

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_employee_reactivate_already_active_rejected() {
    let test_name = "employee_reactivate_twice";
    let (db, path) = setup_test_db(test_name).await;

    let (_inst_id, company_id, _dept_id, position_id) = setup_organization(&db).await;

    let employee_repo = EmployeeRepository::new(db.pool());
    let employee = employee_repo
        .create(&position_id, "Funcionário", None, None)
        .await
        .expect("Failed to create employee");

    // Tentar reativar já ativo - deve falhar
    let result = employee_repo.reactivate(&employee.id, &company_id).await;
    assert!(
        result.is_err(),
        "Should reject reactivating already active employee"
    );

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_employee_persistence_after_restart() {
    let test_name = "employee_persistence";
    let (db, path) = setup_test_db(test_name).await;

    let (_inst_id, company_id, _dept_id, position_id) = setup_organization(&db).await;

    let employee_repo = EmployeeRepository::new(db.pool());
    let employee = employee_repo
        .create(&position_id, "Funcionário Persistente", Some("Resp"), None)
        .await
        .expect("Failed to create employee");

    // Simular reinicialização: fechar e reabrir o banco
    drop(db);

    let db2 = Database::open(Path::new(&path))
        .await
        .expect("Failed to reopen database");

    let employee_repo2 = EmployeeRepository::new(db2.pool());
    let found = employee_repo2
        .find_by_id(&employee.id)
        .await
        .expect("Failed to find employee after restart");

    assert_eq!(found.id, employee.id);
    assert_eq!(found.name, "Funcionário Persistente");
    assert_eq!(found.responsibilities(), Some("Resp"));
    assert!(found.is_active());

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_employee_valid_association_company_department_position() {
    let test_name = "employee_valid_assoc";
    let (db, path) = setup_test_db(test_name).await;

    let (_inst_id, company_id, dept_id, position_id) = setup_organization(&db).await;

    let employee_repo = EmployeeRepository::new(db.pool());
    let employee = employee_repo
        .create(&position_id, "Funcionário Válido", None, None)
        .await
        .expect("Failed to create employee");

    // Validar cadeia completa: funcionário → cargo → departamento → empresa
    assert!(employee_repo
        .belongs_to_company(&employee.id, &company_id)
        .await
        .expect("Query failed"));

    // Validar que o cargo pertence ao departamento
    assert!(employee_repo
        .department_exists_and_active(&dept_id)
        .await
        .expect("Query failed"));

    // Validar que o cargo está ativo
    assert!(employee_repo
        .position_exists_and_active(&position_id)
        .await
        .expect("Query failed"));

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_employee_reject_invalid_position() {
    let test_name = "employee_reject_invalid_position";
    let (db, path) = setup_test_db(test_name).await;

    let (_inst_id, _company_id, _dept_id, _position_id) = setup_organization(&db).await;

    let employee_repo = EmployeeRepository::new(db.pool());

    // Tentar criar funcionário com position_id inexistente - deve falhar
    let result = employee_repo
        .create("position-inexistente", "Funcionário", None, None)
        .await;
    assert!(result.is_err(), "Should reject invalid position_id");

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_employee_isolation_between_companies() {
    let test_name = "employee_isolation_company";
    let (db, path) = setup_test_db(test_name).await;

    let repo = InstallationRepository::new(db.pool());
    let installation = repo
        .create("Isolation Company Installation", "owner-iso-company")
        .await
        .expect("Failed to create installation");

    let company_repo = CompanyRepository::new(db.pool());
    let company_a = company_repo
        .create(&installation.id, "Empresa A")
        .await
        .expect("Failed to create company A");
    let company_b = company_repo
        .create(&installation.id, "Empresa B")
        .await
        .expect("Failed to create company B");

    let dept_repo = DepartmentRepository::new(db.pool());
    let dept_a = dept_repo
        .create(&company_a.id, "DEPT-A", "Dept A", None)
        .await
        .expect("Failed to create dept A");
    let dept_b = dept_repo
        .create(&company_b.id, "DEPT-B", "Dept B", None)
        .await
        .expect("Failed to create dept B");

    let pos_repo = PositionRepository::new(db.pool());
    let position_a = pos_repo
        .create(&dept_a.id, "Cargo A", 1, 100, None)
        .await
        .expect("Failed to create position A");
    let position_b = pos_repo
        .create(&dept_b.id, "Cargo B", 1, 200, None)
        .await
        .expect("Failed to create position B");

    let employee_repo = EmployeeRepository::new(db.pool());
    let employee_a = employee_repo
        .create(&position_a.id, "Funcionário A", None, None)
        .await
        .expect("Failed to create employee A");
    let employee_b = employee_repo
        .create(&position_b.id, "Funcionário B", None, None)
        .await
        .expect("Failed to create employee B");

    // Funcionário A pertence à empresa A, não à B
    assert!(employee_repo
        .belongs_to_company(&employee_a.id, &company_a.id)
        .await
        .expect("Query failed"));
    assert!(!employee_repo
        .belongs_to_company(&employee_a.id, &company_b.id)
        .await
        .expect("Query failed"));

    // Funcionário B pertence à empresa B, não à A
    assert!(employee_repo
        .belongs_to_company(&employee_b.id, &company_b.id)
        .await
        .expect("Query failed"));
    assert!(!employee_repo
        .belongs_to_company(&employee_b.id, &company_a.id)
        .await
        .expect("Query failed"));

    // Listar por empresa - isolamento
    let employees_a = employee_repo
        .list_by_company(&company_a.id, false)
        .await
        .expect("Failed to list employees A");
    assert_eq!(employees_a.len(), 1);
    assert_eq!(employees_a[0].id, employee_a.id);

    let employees_b = employee_repo
        .list_by_company(&company_b.id, false)
        .await
        .expect("Failed to list employees B");
    assert_eq!(employees_b.len(), 1);
    assert_eq!(employees_b[0].id, employee_b.id);

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_employee_isolation_between_installations() {
    let test_name = "employee_isolation_installation";
    let (db, path) = setup_test_db(test_name).await;

    let repo = InstallationRepository::new(db.pool());
    let inst1 = repo
        .create("Installation 1", "owner-1")
        .await
        .expect("Failed inst 1");
    let inst2 = repo
        .create("Installation 2", "owner-2")
        .await
        .expect("Failed inst 2");

    let company_repo = CompanyRepository::new(db.pool());
    let comp1 = company_repo
        .create(&inst1.id, "Empresa 1")
        .await
        .expect("Failed comp 1");
    let comp2 = company_repo
        .create(&inst2.id, "Empresa 2")
        .await
        .expect("Failed comp 2");

    let dept_repo = DepartmentRepository::new(db.pool());
    let dept1 = dept_repo
        .create(&comp1.id, "DEPT-1", "Dept 1", None)
        .await
        .expect("Failed dept 1");
    let dept2 = dept_repo
        .create(&comp2.id, "DEPT-2", "Dept 2", None)
        .await
        .expect("Failed dept 2");

    let pos_repo = PositionRepository::new(db.pool());
    let pos1 = pos_repo
        .create(&dept1.id, "Cargo 1", 1, 100, None)
        .await
        .expect("Failed pos 1");
    let pos2 = pos_repo
        .create(&dept2.id, "Cargo 2", 1, 200, None)
        .await
        .expect("Failed pos 2");

    let employee_repo = EmployeeRepository::new(db.pool());
    let emp1 = employee_repo
        .create(&pos1.id, "Funcionário 1", None, None)
        .await
        .expect("Failed emp 1");
    let emp2 = employee_repo
        .create(&pos2.id, "Funcionário 2", None, None)
        .await
        .expect("Failed emp 2");

    // Funcionário 1 pertence à instalação 1, não à 2
    assert!(employee_repo
        .belongs_to_installation(&emp1.id, &inst1.id)
        .await
        .expect("Query failed"));
    assert!(!employee_repo
        .belongs_to_installation(&emp1.id, &inst2.id)
        .await
        .expect("Query failed"));

    // Funcionário 2 pertence à instalação 2, não à 1
    assert!(employee_repo
        .belongs_to_installation(&emp2.id, &inst2.id)
        .await
        .expect("Query failed"));
    assert!(!employee_repo
        .belongs_to_installation(&emp2.id, &inst1.id)
        .await
        .expect("Query failed"));

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_employee_referential_integrity_inactive_position() {
    let test_name = "employee_integrity_inactive_pos";
    let (db, path) = setup_test_db(test_name).await;

    let (_inst_id, company_id, dept_id, _position_id) = setup_organization(&db).await;

    let pos_repo = PositionRepository::new(db.pool());

    // Criar um cargo e depois inativá-lo (via update direto no banco para teste de integridade)
    let position = pos_repo
        .create(&dept_id, "Cargo Inativo", 1, 100, None)
        .await
        .expect("Failed to create position");

    // Inativar o cargo diretamente via SQL (simulando archive)
    sqlx::query("UPDATE positions SET status = 'inactive' WHERE id = ?")
        .bind(&position.id)
        .execute(db.pool())
        .await
        .expect("Failed to deactivate position");

    let employee_repo = EmployeeRepository::new(db.pool());

    // Criar funcionário com cargo inativo - deve ser possível no nível do repositório
    // (o comando Tauri fará a validação de cargo ativo)
    let employee = employee_repo
        .create(&position.id, "Funcionário Cargo Inativo", None, None)
        .await
        .expect("Failed to create employee");

    // Mas listar por departamento não deve incluir funcionários com cargo inativo
    let active_employees = employee_repo
        .list_by_department(&dept_id, false)
        .await
        .expect("Failed to list active employees");

    assert!(
        active_employees.iter().all(|e| e.id != employee.id),
        "Employee with inactive position should not appear in active list"
    );

    // Mas deve aparecer quando incluir inativos
    let all_employees = employee_repo
        .list_by_department(&dept_id, true)
        .await
        .expect("Failed to list all employees");

    assert!(
        all_employees.iter().any(|e| e.id == employee.id),
        "Employee should appear when including inactives"
    );

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_employee_deactivated_not_in_active_list() {
    let test_name = "employee_deactivated_list";
    let (db, path) = setup_test_db(test_name).await;

    let (_inst_id, company_id, _dept_id, position_id) = setup_organization(&db).await;

    let employee_repo = EmployeeRepository::new(db.pool());
    let employee = employee_repo
        .create(&position_id, "Funcionário Desativo", None, None)
        .await
        .expect("Failed to create employee");

    // Listar ativos - deve conter o funcionário
    let active = employee_repo
        .list_by_company(&company_id, false)
        .await
        .expect("Failed to list active");
    assert_eq!(active.len(), 1);

    // Desativar
    employee_repo
        .deactivate(&employee.id, &company_id)
        .await
        .expect("Failed to deactivate");

    // Listar ativos - não deve conter o funcionário
    let active_after = employee_repo
        .list_by_company(&company_id, false)
        .await
        .expect("Failed to list active after");
    assert_eq!(active_after.len(), 0);

    // Listar todos (incluindo inativos) - deve conter o funcionário
    let all = employee_repo
        .list_by_company(&company_id, true)
        .await
        .expect("Failed to list all");
    assert_eq!(all.len(), 1);
    assert!(!all[0].is_active());

    cleanup_test_db(&path);
}

// ============================================================================
// TESTES DE REGRESSÃO — Sincronização de status e is_active
// ============================================================================

/// Regressão: a migration 004, aplicada sobre um banco que só tinha as
/// migrations 001–003, preserva o estado de funcionários legados — quem
/// tinha `is_active = 0` continua inativo e não aparece em `active_employees`.
#[tokio::test]
async fn test_migration_004_backfills_legacy_inactive_employee() {
    let test_name = "employee_legacy_migration";
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("tina-employee-test-{}-{}.db", test_name, timestamp));
    let path_str = path.to_string_lossy().to_string();

    let options = SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .expect("Failed to open test database");

    // Schema anterior à migration 004: 001 + 002 + 003.
    apply_migration_sql(&pool, include_str!("../../migration/001_initial.sql")).await;
    apply_migration_sql(
        &pool,
        include_str!("../../migration/002_employee_deactivated_at.sql"),
    )
    .await;
    apply_migration_sql(
        &pool,
        include_str!("../../migration/003_employee_status_column.sql"),
    )
    .await;

    // Cadeia organizacional mínima.
    for statement in [
        "INSERT INTO installations (id, name, root_path, version, owner_uuid) \
         VALUES ('inst-legacy', 'Legacy', '/tmp/legacy', '0.1.0', 'owner-legacy')",
        "INSERT INTO companies (id, installation_id, name) \
         VALUES ('comp-legacy', 'inst-legacy', 'Empresa Legacy')",
        "INSERT INTO departments (id, company_id, name, code) \
         VALUES ('dept-legacy', 'comp-legacy', 'Dept Legacy', 'DEPT-LEGACY')",
        "INSERT INTO positions (id, department_id, name) \
         VALUES ('pos-legacy', 'dept-legacy', 'Cargo Legacy')",
    ] {
        sqlx::query(statement)
            .execute(&pool)
            .await
            .expect("Failed to seed organizational chain");
    }

    // Funcionários legados: a migration 003 deixou status = 'active' em todos,
    // mas um deles já estava desativado via is_active = 0.
    sqlx::query(
        "INSERT INTO employees (id, position_id, name, model_config, is_active) \
         VALUES ('legacy-inactive', 'pos-legacy', 'Legado Inativo', 'nvidia/nemotron-3.5-bolt-e4b', 0)",
    )
    .execute(&pool)
    .await
    .expect("Failed to insert inactive legacy employee");
    sqlx::query(
        "INSERT INTO employees (id, position_id, name, model_config, is_active) \
         VALUES ('legacy-active', 'pos-legacy', 'Legado Ativo', 'nvidia/nemotron-3.5-bolt-e4b', 1)",
    )
    .execute(&pool)
    .await
    .expect("Failed to insert active legacy employee");

    apply_migration_sql(
        &pool,
        include_str!("../../migration/004_employee_status_sync.sql"),
    )
    .await;

    // As duas colunas permanecem consistentes e o legado inativo continua inativo.
    let legacy_inactive: (String, i64) =
        sqlx::query_as("SELECT status, is_active FROM employees WHERE id = 'legacy-inactive'")
            .fetch_one(&pool)
            .await
            .expect("Failed to fetch legacy inactive employee");
    assert_eq!(
        legacy_inactive,
        ("inactive".to_string(), 0),
        "Legacy employee with is_active=0 must stay inactive after migration 004"
    );

    let legacy_active: (String, i64) =
        sqlx::query_as("SELECT status, is_active FROM employees WHERE id = 'legacy-active'")
            .fetch_one(&pool)
            .await
            .expect("Failed to fetch legacy active employee");
    assert_eq!(
        legacy_active,
        ("active".to_string(), 1),
        "Legacy employee with is_active=1 must stay active after migration 004"
    );

    // A view reflete o estado atual.
    let in_view: Option<String> =
        sqlx::query_scalar("SELECT id FROM active_employees WHERE id = 'legacy-inactive'")
            .fetch_optional(&pool)
            .await
            .expect("Failed to query active_employees view");
    assert!(
        in_view.is_none(),
        "Legacy inactive employee must not appear in active_employees"
    );

    // O repositório também enxerga o legado como inativo.
    let employee_repo = EmployeeRepository::new(&pool);
    let found = employee_repo
        .find_by_id("legacy-inactive")
        .await
        .expect("Failed to find legacy employee");
    assert!(
        !found.is_active(),
        "Repository must expose legacy employee as inactive"
    );

    pool.close().await;
    cleanup_test_db(&path_str);
}

/// Regressão: funcionário desativado não aparece na view active_employees.
#[tokio::test]
async fn test_employee_deactivated_not_in_active_employees_view() {
    let test_name = "employee_deactivated_view";
    let (db, path) = setup_test_db(test_name).await;

    let (_inst_id, company_id, dept_id, position_id) = setup_organization(&db).await;

    let employee_repo = EmployeeRepository::new(db.pool());
    let employee = employee_repo
        .create(&position_id, "Funcionário View", None, None)
        .await
        .expect("Failed to create employee");

    // Aparece na view antes de desativar
    let before: Option<String> = sqlx::query_scalar("SELECT id FROM active_employees WHERE id = ?")
        .bind(&employee.id)
        .fetch_optional(db.pool())
        .await
        .expect("Failed to query active_employees view");
    assert!(
        before.is_some(),
        "Employee must appear in active_employees before deactivation"
    );

    // Desativar
    employee_repo
        .deactivate(&employee.id, &company_id)
        .await
        .expect("Failed to deactivate employee");

    // Não aparece mais na view após desativar
    let after: Option<String> = sqlx::query_scalar("SELECT id FROM active_employees WHERE id = ?")
        .bind(&employee.id)
        .fetch_optional(db.pool())
        .await
        .expect("Failed to query active_employees view");
    assert!(
        after.is_none(),
        "Deactivated employee must not appear in active_employees view"
    );

    // Verificar consistência das duas colunas no banco
    let row: (String, i64, Option<String>) =
        sqlx::query_as("SELECT status, is_active, deactivated_at FROM employees WHERE id = ?")
            .bind(&employee.id)
            .fetch_one(db.pool())
            .await
            .expect("Failed to fetch employee row");
    assert_eq!(row.0, "inactive", "status must be inactive");
    assert_eq!(row.1, 0, "is_active must be 0");
    assert!(row.2.is_some(), "deactivated_at must be set");

    cleanup_test_db(&path);
}

/// Regressão: funcionário desativado não pode ser atribuído como superior.
#[tokio::test]
async fn test_employee_deactivated_cannot_be_supervisor() {
    let test_name = "employee_deactivated_supervisor";
    let (db, path) = setup_test_db(test_name).await;

    let (_inst_id, company_id, dept_id, position_id) = setup_organization(&db).await;

    let employee_repo = EmployeeRepository::new(db.pool());

    // Criar supervisor e funcionário subordinado
    let supervisor = employee_repo
        .create(&position_id, "Supervisor", None, None)
        .await
        .expect("Failed to create supervisor");
    let employee = employee_repo
        .create(&position_id, "Subordinado", None, None)
        .await
        .expect("Failed to create employee");

    // Desativar o supervisor
    employee_repo
        .deactivate(&supervisor.id, &company_id)
        .await
        .expect("Failed to deactivate supervisor");

    // Tentar atribuir o supervisor desativado como superior do funcionário
    let result = employee_repo
        .assign_supervisor(&employee.id, &company_id, Some(&supervisor.id))
        .await;

    assert!(
        result.is_err(),
        "Deactivated employee must not be assignable as supervisor"
    );

    // A mesma regra usada pelo comando Tauri rejeita o superior inativo.
    assert!(
        !employee_repo
            .is_active_in_company(&supervisor.id, &company_id)
            .await
            .expect("Failed to check active supervisor"),
        "Inactive employee must not be considered an active supervisor"
    );

    cleanup_test_db(&path);
}

/// Regressão: funcionário reativado volta a aparecer nas listagens de ativos.
#[tokio::test]
async fn test_employee_reactivated_appears_in_active_list() {
    let test_name = "employee_reactivated_active";
    let (db, path) = setup_test_db(test_name).await;

    let (_inst_id, company_id, dept_id, position_id) = setup_organization(&db).await;

    let employee_repo = EmployeeRepository::new(db.pool());
    let employee = employee_repo
        .create(&position_id, "Funcionário Reativar", None, None)
        .await
        .expect("Failed to create employee");

    // Desativar
    employee_repo
        .deactivate(&employee.id, &company_id)
        .await
        .expect("Failed to deactivate employee");

    let inactive = employee_repo
        .list_by_company(&company_id, false)
        .await
        .expect("Failed to list active after deactivate");
    assert_eq!(
        inactive.len(),
        0,
        "Deactivated employee must not be in active list"
    );

    // Reativar
    employee_repo
        .reactivate(&employee.id, &company_id)
        .await
        .expect("Failed to reactivate employee");

    // Volta a aparecer na listagem de ativos
    let active = employee_repo
        .list_by_company(&company_id, false)
        .await
        .expect("Failed to list active after reactivate");
    assert_eq!(
        active.len(),
        1,
        "Reactivated employee must appear in active list"
    );
    assert_eq!(active[0].id, employee.id);

    // Verificar consistência das duas colunas
    let found = employee_repo
        .find_by_id(&employee.id)
        .await
        .expect("Failed to find reactivated employee");
    assert!(found.is_active(), "Reactivated employee must be active");

    cleanup_test_db(&path);
}

/// Regressão: as colunas status e is_active permanecem consistentes
/// após operações de criação, desativação e reativação.
#[tokio::test]
async fn test_employee_status_is_active_consistency() {
    let test_name = "employee_status_consistency";
    let (db, path) = setup_test_db(test_name).await;

    let (_inst_id, company_id, dept_id, position_id) = setup_organization(&db).await;

    let employee_repo = EmployeeRepository::new(db.pool());
    let employee = employee_repo
        .create(&position_id, "Funcionário Consistência", None, None)
        .await
        .expect("Failed to create employee");

    // Após criação: status = 'active', is_active = 1
    let row: (String, i64) = sqlx::query_as("SELECT status, is_active FROM employees WHERE id = ?")
        .bind(&employee.id)
        .fetch_one(db.pool())
        .await
        .expect("Failed to fetch employee row");
    assert_eq!(row.0, "active", "status must be active after creation");
    assert_eq!(row.1, 1, "is_active must be 1 after creation");

    // Após desativação: status = 'inactive', is_active = 0
    employee_repo
        .deactivate(&employee.id, &company_id)
        .await
        .expect("Failed to deactivate employee");

    let row: (String, i64) = sqlx::query_as("SELECT status, is_active FROM employees WHERE id = ?")
        .bind(&employee.id)
        .fetch_one(db.pool())
        .await
        .expect("Failed to fetch employee row");
    assert_eq!(
        row.0, "inactive",
        "status must be inactive after deactivation"
    );
    assert_eq!(row.1, 0, "is_active must be 0 after deactivation");

    // Após reativação: status = 'active', is_active = 1
    employee_repo
        .reactivate(&employee.id, &company_id)
        .await
        .expect("Failed to reactivate employee");

    let row: (String, i64) = sqlx::query_as("SELECT status, is_active FROM employees WHERE id = ?")
        .bind(&employee.id)
        .fetch_one(db.pool())
        .await
        .expect("Failed to fetch employee row");
    assert_eq!(row.0, "active", "status must be active after reactivation");
    assert_eq!(row.1, 1, "is_active must be 1 after reactivation");

    cleanup_test_db(&path);
}
