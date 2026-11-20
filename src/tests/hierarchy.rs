//! Testes de integração da hierarquia dinâmica e do histórico de relações.
//!
//! Usam SQLite real em arquivos temporários — sem mocks — porque as regras
//! dependem de constraints, transações e consultas recursivas.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use crate::core::employee::Employee;
use crate::core::hierarchy::HierarchyOperation;
use crate::error::TinaError;
use crate::infra::{
    CompanyRepository, DepartmentRepository, EmployeeRepository, HierarchyService,
    InstallationRepository, PositionRepository,
};
use crate::Database;

/// Organização mínima para os testes: instalação → empresa → departamento → cargo.
struct Org {
    installation_id: String,
    company_id: String,
    position_id: String,
}

async fn setup_test_db(test_name: &str) -> (Database, String) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let path = std::env::temp_dir().join(format!(
        "tina-hierarchy-test-{}-{}.db",
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
    let installations = InstallationRepository::new(db.pool());
    let installation = installations
        .create(&format!("Instalação {label}"), &format!("owner-{label}"))
        .await
        .expect("Failed to create installation");

    let companies = CompanyRepository::new(db.pool());
    let company = companies
        .create(&installation.id, &format!("Empresa {label}"))
        .await
        .expect("Failed to create company");

    let departments = DepartmentRepository::new(db.pool());
    let department = departments
        .create(
            &company.id,
            &format!("DEPT-{label}"),
            &format!("Departamento {label}"),
            None,
        )
        .await
        .expect("Failed to create department");

    let positions = PositionRepository::new(db.pool());
    let position = positions
        .create(&department.id, &format!("Cargo {label}"), 1, 100, None)
        .await
        .expect("Failed to create position");

    Org {
        installation_id: installation.id,
        company_id: company.id,
        position_id: position.id,
    }
}

async fn create_employee(db: &Database, position_id: &str, name: &str) -> Employee {
    EmployeeRepository::new(db.pool())
        .create(position_id, name, None, None)
        .await
        .expect("Failed to create employee")
}

fn assert_hierarchy_error<T: std::fmt::Debug>(result: crate::error::Result<T>) {
    match result {
        Err(TinaError::Hierarchy(_)) => {}
        other => panic!("Expected a typed hierarchy error, got {other:?}"),
    }
}

async fn history_len(db: &Database, employee_id: &str, company_id: &str) -> usize {
    HierarchyService::new(db.pool())
        .history(employee_id, company_id)
        .await
        .expect("Failed to load history")
        .len()
}

#[tokio::test]
async fn test_valid_relation_created() {
    let (db, path) = setup_test_db("valid_relation").await;
    let org = setup_organization(&db, "valid").await;

    let supervisor = create_employee(&db, &org.position_id, "Supervisor").await;
    let employee = create_employee(&db, &org.position_id, "Subordinado").await;

    let service = HierarchyService::new(db.pool());
    let change = service
        .set_supervisor(
            &employee.id,
            &org.company_id,
            Some(&supervisor.id),
            Some("estrutura inicial"),
            None,
        )
        .await
        .expect("Valid relation should be accepted");

    assert_eq!(change.operation, Some(HierarchyOperation::Assigned));
    assert_eq!(change.previous_supervisor_id, None);
    assert_eq!(
        change.new_supervisor_id.as_deref(),
        Some(supervisor.id.as_str())
    );

    let stored = EmployeeRepository::new(db.pool())
        .find_by_id(&employee.id)
        .await
        .expect("Failed to reload employee");
    assert_eq!(
        stored.supervisor_id.as_deref(),
        Some(supervisor.id.as_str())
    );

    let history = service
        .history(&employee.id, &org.company_id)
        .await
        .expect("Failed to load history");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].operation, HierarchyOperation::Assigned);
    assert_eq!(history[0].previous_supervisor_id, None);
    assert_eq!(
        history[0].new_supervisor_id.as_deref(),
        Some(supervisor.id.as_str())
    );
    assert_eq!(history[0].reason.as_deref(), Some("estrutura inicial"));
    assert_eq!(
        history[0].author_id, None,
        "No authenticated author exists in this context"
    );

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_three_level_hierarchy_and_queries() {
    let (db, path) = setup_test_db("three_levels").await;
    let org = setup_organization(&db, "three").await;

    let root = create_employee(&db, &org.position_id, "Raiz").await;
    let middle = create_employee(&db, &org.position_id, "Meio").await;
    let leaf = create_employee(&db, &org.position_id, "Folha").await;

    let service = HierarchyService::new(db.pool());
    service
        .set_supervisor(&middle.id, &org.company_id, Some(&root.id), None, None)
        .await
        .expect("middle → root");
    service
        .set_supervisor(&leaf.id, &org.company_id, Some(&middle.id), None, None)
        .await
        .expect("leaf → middle");

    let chain = service
        .supervisor_chain(&leaf.id, &org.company_id)
        .await
        .expect("chain query");
    assert_eq!(
        chain.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
        vec!["Meio", "Raiz"]
    );

    let direct = service
        .direct_subordinates(&middle.id, &org.company_id, false)
        .await
        .expect("direct subordinates");
    assert_eq!(direct.len(), 1);
    assert_eq!(direct[0].id, leaf.id);

    let descendants = service
        .descendants(&root.id, &org.company_id, false)
        .await
        .expect("descendants");
    assert_eq!(descendants.len(), 2);
    assert_eq!(descendants[0].employee.id, middle.id);
    assert_eq!(descendants[0].depth, Some(1));
    assert_eq!(descendants[1].employee.id, leaf.id);
    assert_eq!(descendants[1].depth, Some(2));

    let indirect = descendants
        .iter()
        .filter(|node| node.depth.unwrap_or(0) >= 2)
        .count();
    assert_eq!(
        indirect, 1,
        "leaf must be reachable as indirect subordinate"
    );

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_deep_hierarchy_without_artificial_limit() {
    let (db, path) = setup_test_db("deep_hierarchy").await;
    let org = setup_organization(&db, "deep").await;

    const LEVELS: usize = 50;
    let mut employees = Vec::with_capacity(LEVELS);
    for index in 0..LEVELS {
        employees.push(create_employee(&db, &org.position_id, &format!("Nível {index:02}")).await);
    }

    let service = HierarchyService::new(db.pool());
    for pair in employees.windows(2) {
        service
            .set_supervisor(&pair[1].id, &org.company_id, Some(&pair[0].id), None, None)
            .await
            .expect("Failed to build deep hierarchy");
    }

    let deepest = employees.last().unwrap();
    let chain = service
        .supervisor_chain(&deepest.id, &org.company_id)
        .await
        .expect("chain query");
    assert_eq!(chain.len(), LEVELS - 1);

    let descendants = service
        .descendants(&employees[0].id, &org.company_id, false)
        .await
        .expect("descendants query");
    assert_eq!(descendants.len(), LEVELS - 1);
    assert_eq!(descendants.last().unwrap().depth, Some((LEVELS - 1) as i64));

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_change_supervisor_records_previous_and_new() {
    let (db, path) = setup_test_db("change_supervisor").await;
    let org = setup_organization(&db, "change").await;

    let first = create_employee(&db, &org.position_id, "Primeiro Superior").await;
    let second = create_employee(&db, &org.position_id, "Segundo Superior").await;
    let employee = create_employee(&db, &org.position_id, "Subordinado").await;

    let service = HierarchyService::new(db.pool());
    service
        .set_supervisor(&employee.id, &org.company_id, Some(&first.id), None, None)
        .await
        .expect("initial assignment");

    let change = service
        .set_supervisor(
            &employee.id,
            &org.company_id,
            Some(&second.id),
            Some("reorganização"),
            None,
        )
        .await
        .expect("supervisor change");

    assert_eq!(change.operation, Some(HierarchyOperation::Changed));
    assert_eq!(
        change.previous_supervisor_id.as_deref(),
        Some(first.id.as_str())
    );
    assert_eq!(
        change.new_supervisor_id.as_deref(),
        Some(second.id.as_str())
    );

    let history = service
        .history(&employee.id, &org.company_id)
        .await
        .expect("history");
    assert_eq!(history.len(), 2);
    assert_eq!(history[1].operation, HierarchyOperation::Changed);
    assert_eq!(
        history[1].previous_supervisor_id.as_deref(),
        Some(first.id.as_str())
    );
    assert_eq!(
        history[1].new_supervisor_id.as_deref(),
        Some(second.id.as_str())
    );
    assert_eq!(history[1].reason.as_deref(), Some("reorganização"));

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_remove_relation_preserves_history() {
    let (db, path) = setup_test_db("remove_relation").await;
    let org = setup_organization(&db, "remove").await;

    let supervisor = create_employee(&db, &org.position_id, "Supervisor").await;
    let employee = create_employee(&db, &org.position_id, "Subordinado").await;

    let service = HierarchyService::new(db.pool());
    service
        .set_supervisor(
            &employee.id,
            &org.company_id,
            Some(&supervisor.id),
            None,
            None,
        )
        .await
        .expect("assign");

    let change = service
        .set_supervisor(
            &employee.id,
            &org.company_id,
            None,
            Some("fim da relação"),
            None,
        )
        .await
        .expect("remove");

    assert_eq!(change.operation, Some(HierarchyOperation::Removed));
    assert_eq!(
        change.previous_supervisor_id.as_deref(),
        Some(supervisor.id.as_str())
    );
    assert_eq!(change.new_supervisor_id, None);

    let stored = EmployeeRepository::new(db.pool())
        .find_by_id(&employee.id)
        .await
        .expect("reload");
    assert_eq!(stored.supervisor_id, None);

    let history = service
        .history(&employee.id, &org.company_id)
        .await
        .expect("history");
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].operation, HierarchyOperation::Assigned);
    assert_eq!(history[1].operation, HierarchyOperation::Removed);
    assert_eq!(
        history[1].previous_supervisor_id.as_deref(),
        Some(supervisor.id.as_str())
    );

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_full_history_is_chronological() {
    let (db, path) = setup_test_db("full_history").await;
    let org = setup_organization(&db, "history").await;

    let a = create_employee(&db, &org.position_id, "A").await;
    let b = create_employee(&db, &org.position_id, "B").await;
    let employee = create_employee(&db, &org.position_id, "Histórico").await;

    let service = HierarchyService::new(db.pool());
    service
        .set_supervisor(
            &employee.id,
            &org.company_id,
            Some(&a.id),
            Some("início"),
            None,
        )
        .await
        .expect("assign");
    service
        .set_supervisor(
            &employee.id,
            &org.company_id,
            Some(&b.id),
            Some("troca"),
            None,
        )
        .await
        .expect("change");
    service
        .set_supervisor(&employee.id, &org.company_id, None, Some("saída"), None)
        .await
        .expect("remove");

    let history = service
        .history(&employee.id, &org.company_id)
        .await
        .expect("history");
    assert_eq!(history.len(), 3);
    assert_eq!(
        history
            .iter()
            .map(|entry| entry.operation)
            .collect::<Vec<_>>(),
        vec![
            HierarchyOperation::Assigned,
            HierarchyOperation::Changed,
            HierarchyOperation::Removed
        ]
    );
    assert_eq!(
        history
            .iter()
            .map(|entry| entry.reason.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("início"), Some("troca"), Some("saída")]
    );
    assert!(history.iter().all(|entry| entry.author_id.is_none()));
    assert!(history.iter().all(|entry| !entry.created_at.is_empty()));

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_noop_assignment_does_not_create_history() {
    let (db, path) = setup_test_db("noop_assignment").await;
    let org = setup_organization(&db, "noop").await;

    let supervisor = create_employee(&db, &org.position_id, "Supervisor").await;
    let employee = create_employee(&db, &org.position_id, "Subordinado").await;

    let service = HierarchyService::new(db.pool());
    service
        .set_supervisor(
            &employee.id,
            &org.company_id,
            Some(&supervisor.id),
            None,
            None,
        )
        .await
        .expect("assign");

    let change = service
        .set_supervisor(
            &employee.id,
            &org.company_id,
            Some(&supervisor.id),
            None,
            None,
        )
        .await
        .expect("same relation");
    assert_eq!(change.operation, None);
    assert_eq!(history_len(&db, &employee.id, &org.company_id).await, 1);

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_self_supervision_rejected() {
    let (db, path) = setup_test_db("self_supervision").await;
    let org = setup_organization(&db, "self").await;
    let employee = create_employee(&db, &org.position_id, "Autônomo").await;

    let result = HierarchyService::new(db.pool())
        .set_supervisor(
            &employee.id,
            &org.company_id,
            Some(&employee.id),
            None,
            None,
        )
        .await;

    assert_hierarchy_error(result);
    assert_eq!(history_len(&db, &employee.id, &org.company_id).await, 0);

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_direct_cycle_rejected() {
    let (db, path) = setup_test_db("direct_cycle").await;
    let org = setup_organization(&db, "direct_cycle").await;

    let a = create_employee(&db, &org.position_id, "A").await;
    let b = create_employee(&db, &org.position_id, "B").await;

    let service = HierarchyService::new(db.pool());
    service
        .set_supervisor(&b.id, &org.company_id, Some(&a.id), None, None)
        .await
        .expect("B → A");

    let result = service
        .set_supervisor(&a.id, &org.company_id, Some(&b.id), None, None)
        .await;
    assert_hierarchy_error(result);

    let stored = EmployeeRepository::new(db.pool())
        .find_by_id(&a.id)
        .await
        .expect("reload A");
    assert_eq!(stored.supervisor_id, None);
    assert_eq!(history_len(&db, &a.id, &org.company_id).await, 0);

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_indirect_cycle_rejected() {
    let (db, path) = setup_test_db("indirect_cycle").await;
    let org = setup_organization(&db, "indirect_cycle").await;

    let a = create_employee(&db, &org.position_id, "A").await;
    let b = create_employee(&db, &org.position_id, "B").await;
    let c = create_employee(&db, &org.position_id, "C").await;

    let service = HierarchyService::new(db.pool());
    service
        .set_supervisor(&b.id, &org.company_id, Some(&a.id), None, None)
        .await
        .expect("B → A");
    service
        .set_supervisor(&c.id, &org.company_id, Some(&b.id), None, None)
        .await
        .expect("C → B");

    // A respondendo a C fecharia A → B → C → A.
    let result = service
        .set_supervisor(&a.id, &org.company_id, Some(&c.id), None, None)
        .await;
    assert_hierarchy_error(result);

    let stored = EmployeeRepository::new(db.pool())
        .find_by_id(&a.id)
        .await
        .expect("reload A");
    assert_eq!(stored.supervisor_id, None);

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_relation_between_companies_rejected() {
    let (db, path) = setup_test_db("cross_company").await;
    let first = setup_organization(&db, "company_a").await;
    let second = setup_organization(&db, "company_b").await;

    let employee = create_employee(&db, &first.position_id, "Funcionário A").await;
    let foreign_supervisor = create_employee(&db, &second.position_id, "Supervisor B").await;

    let result = HierarchyService::new(db.pool())
        .set_supervisor(
            &employee.id,
            &first.company_id,
            Some(&foreign_supervisor.id),
            None,
            None,
        )
        .await;
    assert_hierarchy_error(result);

    let stored = EmployeeRepository::new(db.pool())
        .find_by_id(&employee.id)
        .await
        .expect("reload");
    assert_eq!(stored.supervisor_id, None);

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_relation_between_installations_rejected() {
    let (db, path) = setup_test_db("cross_installation").await;
    let first = setup_organization(&db, "install_a").await;
    let second = setup_organization(&db, "install_b").await;

    assert_ne!(first.installation_id, second.installation_id);

    let employee = create_employee(&db, &first.position_id, "Funcionário Inst A").await;
    let foreign_supervisor = create_employee(&db, &second.position_id, "Supervisor Inst B").await;

    let result = HierarchyService::new(db.pool())
        .set_supervisor(
            &employee.id,
            &first.company_id,
            Some(&foreign_supervisor.id),
            None,
            None,
        )
        .await;
    assert_hierarchy_error(result);

    let stored = EmployeeRepository::new(db.pool())
        .find_by_id(&employee.id)
        .await
        .expect("reload");
    assert_eq!(stored.supervisor_id, None);

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_nonexistent_supervisor_rejected() {
    let (db, path) = setup_test_db("missing_supervisor").await;
    let org = setup_organization(&db, "missing").await;
    let employee = create_employee(&db, &org.position_id, "Funcionário").await;

    let result = HierarchyService::new(db.pool())
        .set_supervisor(
            &employee.id,
            &org.company_id,
            Some("00000000-0000-0000-0000-000000000000"),
            None,
            None,
        )
        .await;
    assert_hierarchy_error(result);

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_inactive_supervisor_rejected_by_status() {
    let (db, path) = setup_test_db("inactive_supervisor").await;
    let org = setup_organization(&db, "inactive").await;

    let supervisor = create_employee(&db, &org.position_id, "Supervisor").await;
    let employee = create_employee(&db, &org.position_id, "Subordinado").await;

    EmployeeRepository::new(db.pool())
        .deactivate(&supervisor.id, &org.company_id)
        .await
        .expect("deactivate supervisor");

    let status: (String, i64) =
        sqlx::query_as("SELECT status, is_active FROM employees WHERE id = ?")
            .bind(&supervisor.id)
            .fetch_one(db.pool())
            .await
            .expect("status row");
    assert_eq!(status, ("inactive".to_string(), 0));

    let result = HierarchyService::new(db.pool())
        .set_supervisor(
            &employee.id,
            &org.company_id,
            Some(&supervisor.id),
            None,
            None,
        )
        .await;
    assert_hierarchy_error(result);

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_failure_updating_relation_leaves_no_partial_state() {
    let (db, path) = setup_test_db("relation_rollback").await;
    let org = setup_organization(&db, "relation_rollback").await;

    let supervisor = create_employee(&db, &org.position_id, "Supervisor").await;
    let employee = create_employee(&db, &org.position_id, "Subordinado").await;

    sqlx::query(
        r#"
        CREATE TRIGGER fail_supervisor_update
        BEFORE UPDATE OF supervisor_id ON employees
        BEGIN
            SELECT RAISE(ABORT, 'forced supervisor update failure');
        END
        "#,
    )
    .execute(db.pool())
    .await
    .expect("create failure trigger");

    let result = HierarchyService::new(db.pool())
        .set_supervisor(
            &employee.id,
            &org.company_id,
            Some(&supervisor.id),
            None,
            None,
        )
        .await;
    assert!(result.is_err(), "Update failure must surface as an error");

    let stored = EmployeeRepository::new(db.pool())
        .find_by_id(&employee.id)
        .await
        .expect("reload");
    assert_eq!(stored.supervisor_id, None, "No partial relation may remain");
    assert_eq!(history_len(&db, &employee.id, &org.company_id).await, 0);

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_failure_writing_history_rolls_back_relation() {
    let (db, path) = setup_test_db("history_rollback").await;
    let org = setup_organization(&db, "history_rollback").await;

    let supervisor = create_employee(&db, &org.position_id, "Supervisor").await;
    let employee = create_employee(&db, &org.position_id, "Subordinado").await;

    sqlx::query(
        r#"
        CREATE TRIGGER fail_history_insert
        BEFORE INSERT ON employee_supervisor_history
        BEGIN
            SELECT RAISE(ABORT, 'forced history failure');
        END
        "#,
    )
    .execute(db.pool())
    .await
    .expect("create failure trigger");

    let result = HierarchyService::new(db.pool())
        .set_supervisor(
            &employee.id,
            &org.company_id,
            Some(&supervisor.id),
            None,
            None,
        )
        .await;
    assert!(
        result.is_err(),
        "History failure must abort the transaction"
    );

    let stored = EmployeeRepository::new(db.pool())
        .find_by_id(&employee.id)
        .await
        .expect("reload");
    assert_eq!(
        stored.supervisor_id, None,
        "Relation must be rolled back when history cannot be written"
    );
    assert_eq!(history_len(&db, &employee.id, &org.company_id).await, 0);

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_relation_and_history_persist_after_reopen() {
    let (db, path) = setup_test_db("persistence").await;
    let org = setup_organization(&db, "persistence").await;

    let supervisor = create_employee(&db, &org.position_id, "Supervisor").await;
    let employee = create_employee(&db, &org.position_id, "Subordinado").await;

    HierarchyService::new(db.pool())
        .set_supervisor(
            &employee.id,
            &org.company_id,
            Some(&supervisor.id),
            Some("persistir"),
            None,
        )
        .await
        .expect("assign");

    db.pool().close().await;
    drop(db);

    let reopened = Database::open(Path::new(&path))
        .await
        .expect("reopen database");

    let stored = EmployeeRepository::new(reopened.pool())
        .find_by_id(&employee.id)
        .await
        .expect("reload after reopen");
    assert_eq!(
        stored.supervisor_id.as_deref(),
        Some(supervisor.id.as_str())
    );

    let history = HierarchyService::new(reopened.pool())
        .history(&employee.id, &org.company_id)
        .await
        .expect("history after reopen");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].reason.as_deref(), Some("persistir"));

    reopened.pool().close().await;
    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_company_isolation_for_tree_and_queries() {
    let (db, path) = setup_test_db("company_isolation").await;
    let first = setup_organization(&db, "isolation_a").await;
    let second = setup_organization(&db, "isolation_b").await;

    let root_a = create_employee(&db, &first.position_id, "Raiz A").await;
    let child_a = create_employee(&db, &first.position_id, "Filho A").await;
    let root_b = create_employee(&db, &second.position_id, "Raiz B").await;

    let service = HierarchyService::new(db.pool());
    service
        .set_supervisor(&child_a.id, &first.company_id, Some(&root_a.id), None, None)
        .await
        .expect("A relation");

    // A árvore da empresa B não contém funcionários da empresa A.
    let tree_b = service
        .company_tree(&second.company_id, false)
        .await
        .expect("tree B");
    assert_eq!(tree_b.len(), 1);
    assert_eq!(tree_b[0].employee.id, root_b.id);

    let tree_a = service
        .company_tree(&first.company_id, false)
        .await
        .expect("tree A");
    assert_eq!(tree_a.len(), 2);
    assert!(tree_a.iter().all(|node| node.employee.id != root_b.id));

    // Consultar a cadeia de A usando o escopo da empresa B não vaza dados.
    let chain_scope_b = service
        .supervisor_chain(&child_a.id, &second.company_id)
        .await
        .expect("scoped chain");
    assert!(chain_scope_b.is_empty());

    // Subordinados diretos também respeitam a empresa.
    let subordinates_scope_b = service
        .direct_subordinates(&root_a.id, &second.company_id, false)
        .await
        .expect("scoped subordinates");
    assert!(subordinates_scope_b.is_empty());

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_deactivated_employee_relation_is_preserved() {
    let (db, path) = setup_test_db("inactive_relation").await;
    let org = setup_organization(&db, "inactive_relation").await;

    let supervisor = create_employee(&db, &org.position_id, "Supervisor").await;
    let employee = create_employee(&db, &org.position_id, "Subordinado").await;

    let service = HierarchyService::new(db.pool());
    service
        .set_supervisor(
            &employee.id,
            &org.company_id,
            Some(&supervisor.id),
            None,
            None,
        )
        .await
        .expect("assign");

    EmployeeRepository::new(db.pool())
        .deactivate(&employee.id, &org.company_id)
        .await
        .expect("deactivate subordinate");

    // A relação e o histórico não são removidos automaticamente.
    let stored = EmployeeRepository::new(db.pool())
        .find_by_id(&employee.id)
        .await
        .expect("reload");
    assert!(!stored.is_active());
    assert_eq!(
        stored.supervisor_id.as_deref(),
        Some(supervisor.id.as_str())
    );
    assert_eq!(history_len(&db, &employee.id, &org.company_id).await, 1);

    // Funcionário inativo não pode ter a relação alterada.
    let result = service
        .set_supervisor(&employee.id, &org.company_id, None, None, None)
        .await;
    assert_hierarchy_error(result);

    // Reativação restaura a participação, mantendo a relação.
    EmployeeRepository::new(db.pool())
        .reactivate(&employee.id, &org.company_id)
        .await
        .expect("reactivate");
    let chain = service
        .supervisor_chain(&employee.id, &org.company_id)
        .await
        .expect("chain after reactivation");
    assert_eq!(chain.len(), 1);
    assert_eq!(chain[0].id, supervisor.id);

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_migration_005_preserves_existing_relations() {
    // Simula um banco legado (001–004) com uma relação já existente e aplica
    // a migration 005 sobre ele.
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("tina-hierarchy-legacy-{timestamp}.db"));
    let path_str = path.to_string_lossy().to_string();

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .expect("Failed to open legacy database");

    for migration in [
        include_str!("../../migration/001_initial.sql"),
        include_str!("../../migration/002_employee_deactivated_at.sql"),
        include_str!("../../migration/003_employee_status_column.sql"),
        include_str!("../../migration/004_employee_status_sync.sql"),
    ] {
        sqlx::raw_sql(migration)
            .execute(&pool)
            .await
            .expect("Failed to apply legacy migration");
    }

    for statement in [
        "INSERT INTO installations (id, name, root_path, version, owner_uuid) \
         VALUES ('inst-legacy', 'Legacy', '/tmp/legacy-hierarchy', '0.1.0', 'owner-legacy')",
        "INSERT INTO companies (id, installation_id, name) \
         VALUES ('comp-legacy', 'inst-legacy', 'Empresa Legacy')",
        "INSERT INTO departments (id, company_id, name, code) \
         VALUES ('dept-legacy', 'comp-legacy', 'Dept Legacy', 'DEPT-LEGACY-H')",
        "INSERT INTO positions (id, department_id, name) \
         VALUES ('pos-legacy', 'dept-legacy', 'Cargo Legacy')",
        "INSERT INTO employees (id, position_id, name, model_config, is_active, status, supervisor_id) \
         VALUES ('legacy-root', 'pos-legacy', 'Raiz Legacy', 'm', 1, 'active', NULL)",
        "INSERT INTO employees (id, position_id, name, model_config, is_active, status, supervisor_id) \
         VALUES ('legacy-child', 'pos-legacy', 'Filho Legacy', 'm', 1, 'active', 'legacy-root')",
    ] {
        sqlx::query(statement)
            .execute(&pool)
            .await
            .expect("Failed to seed legacy hierarchy");
    }

    sqlx::raw_sql(include_str!(
        "../../migration/005_employee_supervisor_history.sql"
    ))
    .execute(&pool)
    .await
    .expect("Failed to apply migration 005");

    let relation: Option<String> =
        sqlx::query_scalar("SELECT supervisor_id FROM employees WHERE id = 'legacy-child'")
            .fetch_one(&pool)
            .await
            .expect("Failed to read legacy relation");
    assert_eq!(relation.as_deref(), Some("legacy-root"));

    let history: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM employee_supervisor_history")
        .fetch_one(&pool)
        .await
        .expect("Failed to count history");
    assert_eq!(history, 0, "Upgrade must not invent history events");

    // A tabela nova aceita eventos e a relação pode ser alterada.
    let service = HierarchyService::new(&pool);
    service
        .set_supervisor(
            "legacy-child",
            "comp-legacy",
            None,
            Some("limpeza pós-upgrade"),
            None,
        )
        .await
        .expect("Failed to change relation after upgrade");
    assert_eq!(
        service
            .history("legacy-child", "comp-legacy")
            .await
            .expect("history after upgrade")
            .len(),
        1
    );

    pool.close().await;
    cleanup_test_db(&path_str);
}

#[tokio::test]
async fn test_concurrent_changes_cannot_create_cycle() {
    let (db, path) = setup_test_db("concurrent_cycles").await;
    let org = setup_organization(&db, "concurrent").await;

    let a = create_employee(&db, &org.position_id, "A").await;
    let b = create_employee(&db, &org.position_id, "B").await;

    let db_a = db.clone();
    let db_b = db.clone();
    let company_a = org.company_id.clone();
    let company_b = org.company_id.clone();
    let a1 = a.id.clone();
    let a2 = a.id.clone();
    let b1 = b.id.clone();
    let b2 = b.id.clone();

    let task_a = tokio::spawn(async move {
        HierarchyService::new(db_a.pool())
            .set_supervisor(&a1, &company_a, Some(&b1), None, None)
            .await
    });
    let task_b = tokio::spawn(async move {
        HierarchyService::new(db_b.pool())
            .set_supervisor(&b2, &company_b, Some(&a2), None, None)
            .await
    });

    let (first, second) = tokio::join!(task_a, task_b);
    let first = first.expect("task A panicked");
    let second = second.expect("task B panicked");

    let successes = [first.is_ok(), second.is_ok()]
        .iter()
        .filter(|ok| **ok)
        .count();
    assert_eq!(
        successes, 1,
        "Exactly one concurrent change must win; the other must be rejected"
    );

    let rejected = if first.is_err() { first } else { second };
    assert!(matches!(rejected, Err(TinaError::Hierarchy(_))));

    // O estado final não contém ciclo: uma única aresta existe.
    let edges: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM employees WHERE supervisor_id IS NOT NULL")
            .fetch_one(db.pool())
            .await
            .expect("count edges");
    assert_eq!(edges, 1);

    cleanup_test_db(&path);
}
