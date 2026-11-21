//! Testes de integração da movimentação de cargo/departamento e da
//! imutabilidade dos históricos organizacionais.
//!
//! Usam SQLite real em arquivos temporários — sem mocks — porque as regras
//! dependem de transações, constraints e triggers.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx;

use crate::core::employee::Employee;
use crate::infra::{
    AssignmentService, CompanyRepository, DepartmentRepository, EmployeeRepository,
    HierarchyService, InstallationRepository, PositionRepository,
};
use crate::Database;

struct Org {
    company_id: String,
    department_id: String,
    position_id: String,
}

async fn setup_test_db(test_name: &str) -> (Database, String) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let path = std::env::temp_dir().join(format!(
        "tina-assignment-test-{}-{}.db",
        test_name, timestamp
    ));
    let path_str = path.to_string_lossy().to_string();

    let db = Database::open(Path::new(&path))
        .await
        .expect("Failed to create test database");

    (db, path_str)
}

fn cleanup_test_db(path: &str) {
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}

async fn setup_organization(db: &Database, label: &str) -> Org {
    let installation = InstallationRepository::new(db.pool())
        .create(&format!("Instalação {label}"), &format!("owner-{label}"))
        .await
        .expect("Failed to create installation");

    let company = CompanyRepository::new(db.pool())
        .create(&installation.id, &format!("Empresa {label}"))
        .await
        .expect("Failed to create company");

    let department = DepartmentRepository::new(db.pool())
        .create(
            &company.id,
            &format!("DEPT-{label}"),
            &format!("Departamento {label}"),
            None,
        )
        .await
        .expect("Failed to create department");

    let position = PositionRepository::new(db.pool())
        .create(&department.id, &format!("Cargo {label}"), 1, 100, None)
        .await
        .expect("Failed to create position");

    Org {
        company_id: company.id,
        department_id: department.id,
        position_id: position.id,
    }
}

async fn create_department(db: &Database, company_id: &str, label: &str) -> String {
    DepartmentRepository::new(db.pool())
        .create(
            company_id,
            &format!("DEPT-{label}"),
            &format!("Departamento {label}"),
            None,
        )
        .await
        .expect("Failed to create department")
        .id
}

async fn create_position(db: &Database, department_id: &str, label: &str) -> String {
    PositionRepository::new(db.pool())
        .create(department_id, &format!("Cargo {label}"), 1, 100, None)
        .await
        .expect("Failed to create position")
        .id
}

async fn create_employee(db: &Database, position_id: &str, name: &str) -> Employee {
    EmployeeRepository::new(db.pool())
        .create(position_id, name, None, None)
        .await
        .expect("Failed to create employee")
}

#[tokio::test]
async fn test_change_position_records_history_in_same_department() {
    let (db, path) = setup_test_db("same_department").await;
    let org = setup_organization(&db, "same_department").await;
    let second_position = create_position(&db, &org.department_id, "Segundo").await;

    let employee = create_employee(&db, &org.position_id, "Funcionário Movido").await;

    let change = AssignmentService::new(db.pool())
        .change_position(
            &employee.id,
            &org.company_id,
            &second_position,
            Some("ajuste de escopo"),
            None,
        )
        .await
        .expect("Failed to change position");

    assert!(change.recorded);
    assert_eq!(change.previous_position_id, org.position_id);
    assert_eq!(change.new_position_id, second_position);
    assert_eq!(change.previous_department_id, org.department_id);
    assert_eq!(change.new_department_id, org.department_id);

    let history = AssignmentService::new(db.pool())
        .history(&employee.id, &org.company_id)
        .await
        .expect("Failed to load assignment history");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].previous_position_id, org.position_id);
    assert_eq!(history[0].new_position_id, second_position);
    assert_eq!(history[0].previous_department_id, org.department_id);
    assert_eq!(history[0].new_department_id, org.department_id);
    assert_eq!(history[0].reason.as_deref(), Some("ajuste de escopo"));
    assert_eq!(history[0].author_id, None);
    assert!(!history[0].created_at.is_empty());

    let stored = EmployeeRepository::new(db.pool())
        .find_by_id(&employee.id)
        .await
        .expect("Failed to reload employee");
    assert_eq!(stored.position_id, second_position);

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_change_position_records_previous_and_new_departments() {
    let (db, path) = setup_test_db("new_department").await;
    let org = setup_organization(&db, "new_department").await;
    let other_department = create_department(&db, &org.company_id, "destino").await;
    let other_position = create_position(&db, &other_department, "destino").await;

    let employee = create_employee(&db, &org.position_id, "Funcionário Transferido").await;

    AssignmentService::new(db.pool())
        .change_position(
            &employee.id,
            &org.company_id,
            &other_position,
            Some("transferência entre departamentos"),
            None,
        )
        .await
        .expect("Failed to transfer employee");

    let history = AssignmentService::new(db.pool())
        .history(&employee.id, &org.company_id)
        .await
        .expect("Failed to load assignment history");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].previous_position_id, org.position_id);
    assert_eq!(history[0].new_position_id, other_position);
    assert_eq!(history[0].previous_department_id, org.department_id);
    assert_eq!(history[0].new_department_id, other_department);
    assert_eq!(
        history[0].reason.as_deref(),
        Some("transferência entre departamentos")
    );

    // O histórico permanece escopado à empresa: outra empresa não o enxerga.
    let other_org = setup_organization(&db, "new_department_other").await;
    let foreign = AssignmentService::new(db.pool())
        .history(&employee.id, &other_org.company_id)
        .await
        .expect("Foreign scope query must not fail");
    assert!(foreign.is_empty());

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_same_position_does_not_create_history() {
    let (db, path) = setup_test_db("noop_position").await;
    let org = setup_organization(&db, "noop_position").await;
    let employee = create_employee(&db, &org.position_id, "Funcionário Estável").await;

    let change = AssignmentService::new(db.pool())
        .change_position(&employee.id, &org.company_id, &org.position_id, None, None)
        .await
        .expect("No-op must succeed");

    assert!(!change.recorded);
    assert_eq!(
        AssignmentService::new(db.pool())
            .history(&employee.id, &org.company_id)
            .await
            .expect("Failed to load assignment history")
            .len(),
        0
    );

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_failure_writing_assignment_history_rolls_back_position() {
    let (db, path) = setup_test_db("assignment_rollback").await;
    let org = setup_organization(&db, "assignment_rollback").await;
    let second_position = create_position(&db, &org.department_id, "Rollback").await;
    let employee = create_employee(&db, &org.position_id, "Funcionário Rollback").await;

    sqlx::raw_sql(
        r#"
        CREATE TRIGGER fail_assignment_history_insert
        BEFORE INSERT ON employee_assignment_history
        BEGIN
            SELECT RAISE(ABORT, 'forced assignment history failure');
        END;
        "#,
    )
    .execute(db.pool())
    .await
    .expect("Failed to create failure trigger");

    let result = AssignmentService::new(db.pool())
        .change_position(&employee.id, &org.company_id, &second_position, None, None)
        .await;
    assert!(result.is_err(), "History failure must fail the operation");

    let stored = EmployeeRepository::new(db.pool())
        .find_by_id(&employee.id)
        .await
        .expect("Failed to reload employee");
    assert_eq!(
        stored.position_id, org.position_id,
        "Position must roll back when history cannot be written"
    );
    assert_eq!(
        AssignmentService::new(db.pool())
            .history(&employee.id, &org.company_id)
            .await
            .expect("Failed to load assignment history")
            .len(),
        0
    );

    sqlx::raw_sql("DROP TRIGGER fail_assignment_history_insert")
        .execute(db.pool())
        .await
        .expect("Failed to drop failure trigger");

    // Inserções legítimas continuam funcionando.
    AssignmentService::new(db.pool())
        .change_position(&employee.id, &org.company_id, &second_position, None, None)
        .await
        .expect("Retry after dropping trigger");
    assert_eq!(
        AssignmentService::new(db.pool())
            .history(&employee.id, &org.company_id)
            .await
            .expect("Failed to load assignment history")
            .len(),
        1
    );

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_assignment_history_rejects_update_and_delete() {
    let (db, path) = setup_test_db("assignment_immutable").await;
    let org = setup_organization(&db, "assignment_immutable").await;
    let second_position = create_position(&db, &org.department_id, "Imutável").await;
    let employee = create_employee(&db, &org.position_id, "Funcionário Imutável").await;

    AssignmentService::new(db.pool())
        .change_position(
            &employee.id,
            &org.company_id,
            &second_position,
            Some("original"),
            None,
        )
        .await
        .expect("Failed to change position");

    let update = sqlx::query(
        "UPDATE employee_assignment_history SET reason = 'alterado' WHERE employee_id = ?",
    )
    .bind(&employee.id)
    .execute(db.pool())
    .await;
    assert!(update.is_err(), "UPDATE on assignment history must fail");

    let delete = sqlx::query("DELETE FROM employee_assignment_history WHERE employee_id = ?")
        .bind(&employee.id)
        .execute(db.pool())
        .await;
    assert!(delete.is_err(), "DELETE on assignment history must fail");

    let history = AssignmentService::new(db.pool())
        .history(&employee.id, &org.company_id)
        .await
        .expect("Failed to load assignment history");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].reason.as_deref(), Some("original"));

    // Inserções legítimas seguem permitidas após as tentativas rejeitadas.
    let third_position = create_position(&db, &org.department_id, "Terceiro").await;
    AssignmentService::new(db.pool())
        .change_position(&employee.id, &org.company_id, &third_position, None, None)
        .await
        .expect("Insert after rejected mutations must succeed");
    assert_eq!(
        AssignmentService::new(db.pool())
            .history(&employee.id, &org.company_id)
            .await
            .expect("Failed to load assignment history")
            .len(),
        2
    );

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_supervisor_history_rejects_update_and_delete() {
    let (db, path) = setup_test_db("supervisor_immutable").await;
    let org = setup_organization(&db, "supervisor_immutable").await;

    let supervisor = create_employee(&db, &org.position_id, "Supervisor").await;
    let employee = create_employee(&db, &org.position_id, "Subordinado").await;

    HierarchyService::new(db.pool())
        .set_supervisor(
            &employee.id,
            &org.company_id,
            Some(&supervisor.id),
            Some("original"),
            None,
        )
        .await
        .expect("Failed to assign supervisor");

    let update = sqlx::query(
        "UPDATE employee_supervisor_history SET reason = 'alterado' WHERE employee_id = ?",
    )
    .bind(&employee.id)
    .execute(db.pool())
    .await;
    assert!(update.is_err(), "UPDATE on supervisor history must fail");

    let delete = sqlx::query("DELETE FROM employee_supervisor_history WHERE employee_id = ?")
        .bind(&employee.id)
        .execute(db.pool())
        .await;
    assert!(delete.is_err(), "DELETE on supervisor history must fail");

    let history = HierarchyService::new(db.pool())
        .history(&employee.id, &org.company_id)
        .await
        .expect("Failed to load supervisor history");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].reason.as_deref(), Some("original"));

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_assignment_history_persists_after_reopen() {
    let (db, path) = setup_test_db("assignment_persistence").await;
    let org = setup_organization(&db, "assignment_persistence").await;
    let second_position = create_position(&db, &org.department_id, "Persistente").await;
    let employee = create_employee(&db, &org.position_id, "Funcionário Persistente").await;

    AssignmentService::new(db.pool())
        .change_position(
            &employee.id,
            &org.company_id,
            &second_position,
            Some("persistir"),
            None,
        )
        .await
        .expect("Failed to change position");

    db.pool().close().await;
    drop(db);

    let reopened = Database::open(Path::new(&path))
        .await
        .expect("Failed to reopen database");

    let stored = EmployeeRepository::new(reopened.pool())
        .find_by_id(&employee.id)
        .await
        .expect("Failed to reload employee");
    assert_eq!(stored.position_id, second_position);

    let history = AssignmentService::new(reopened.pool())
        .history(&employee.id, &org.company_id)
        .await
        .expect("Failed to load assignment history after reopen");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].previous_position_id, org.position_id);
    assert_eq!(history[0].new_position_id, second_position);
    assert_eq!(history[0].reason.as_deref(), Some("persistir"));

    reopened.pool().close().await;
    cleanup_test_db(&path);
}
