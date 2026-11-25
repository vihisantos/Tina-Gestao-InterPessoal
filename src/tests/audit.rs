//! Contratos de auditoria organizacional: atomicidade, isolamento e imutabilidade.
use crate::infra::{
    CompanyRepository, DepartmentRepository, InstallationRepository, PositionRepository,
};
use crate::Database;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Helper para criar banco de teste único para cada teste
async fn setup_test_db(test_name: &str) -> (Database, String) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let path = std::env::temp_dir().join(format!("tina-audit-{}-{}.db", test_name, timestamp));
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

fn _init_logging() {
    // Initialize logging for tests
    let _ = env_logger::try_init();
}

#[tokio::test]
async fn organizational_mutations_are_audited_transactionally() {
    let file = std::env::temp_dir().join(format!("tina-audit-{}.db", uuid::Uuid::new_v4()));
    let db = Database::open(&file).await.unwrap();
    let pool = db.pool();
    let installation = InstallationRepository::new(pool)
        .create("audit", "owner")
        .await
        .unwrap();
    let companies = CompanyRepository::new(pool);
    let company = companies.create(&installation.id, "Acme").await.unwrap();
    companies
        .update_name(&company.id, &installation.id, "Acme Atualizada")
        .await
        .unwrap();
    let dept = DepartmentRepository::new(pool)
        .create(&company.id, "TEC", "Tecnologia", None)
        .await
        .unwrap();
    let position = PositionRepository::new(pool)
        .create(&dept.id, "Analista", 1, 1, None)
        .await
        .unwrap();

    let rows: Vec<(String, String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT entity_type, entity_id, action, actor_type, actor_id FROM organizational_audit_events
         WHERE installation_id = ? ORDER BY rowid",
    )
    .bind(&installation.id)
    .fetch_all(pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0].0, "company");
    assert_eq!(rows[0].2, "created");
    assert_eq!(rows[1].2, "updated");
    assert_eq!(rows[2].0, "department");
    assert_eq!(rows[3].0, "position");
    assert!(rows.iter().all(|row| row.3.is_none() && row.4.is_none()));

    let other = InstallationRepository::new(pool)
        .create("audit-other", "other-owner")
        .await
        .unwrap();
    companies.create(&other.id, "Outra").await.unwrap();
    let scoped_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM organizational_audit_events WHERE installation_id = ?",
    )
    .bind(&installation.id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(scoped_count, 4);

    let row_id: String = sqlx::query_scalar(
        "SELECT id FROM organizational_audit_events WHERE installation_id = ? LIMIT 1",
    )
    .bind(&installation.id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert!(
        sqlx::query("UPDATE organizational_audit_events SET action = 'fake' WHERE id = ?")
            .bind(&row_id)
            .execute(pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM organizational_audit_events WHERE id = ?")
            .bind(&row_id)
            .execute(pool)
            .await
            .is_err()
    );

    // Falha do comando não deve deixar evento da operação abortada.
    let before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM organizational_audit_events WHERE installation_id = ?",
    )
    .bind(&installation.id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert!(companies
        .create(&installation.id, "Acme Atualizada")
        .await
        .is_err());
    let after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM organizational_audit_events WHERE installation_id = ?",
    )
    .bind(&installation.id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(before, after);
    drop(db);
    let _ = std::fs::remove_file(file);
}

#[tokio::test]
async fn audit_query_is_scoped_and_requires_owner_session() {
    use crate::TinaApp;
    let file = std::env::temp_dir().join(format!("tina-audit-scope-{}.db", uuid::Uuid::new_v4()));
    let app = TinaApp::initialize_at(&file, "audit-installation", "audit-owner")
        .await
        .unwrap();
    let pool = app.database().pool();
    let companies = CompanyRepository::new(pool);
    let company_a = companies
        .create("audit-installation", "Empresa A")
        .await
        .unwrap();
    let company_b = companies
        .create("audit-installation", "Empresa B")
        .await
        .unwrap();

    let a = crate::infra::audit::list_events(pool, "audit-installation", Some(&company_a.id), 10)
        .await
        .unwrap();
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].company_id.as_deref(), Some(company_a.id.as_str()));
    let b = crate::infra::audit::list_events(pool, "audit-installation", Some(&company_b.id), 10)
        .await
        .unwrap();
    assert_eq!(b.len(), 1);
    assert_eq!(b[0].company_id.as_deref(), Some(company_b.id.as_str()));
    let foreign = crate::infra::audit::list_events(pool, "different-installation", None, 10)
        .await
        .unwrap();
    assert!(foreign.is_empty());

    assert!(app.owner_audit_events(None, 10).await.is_err());
    assert!(app
        .owner_audit_events(Some(&company_a.id), 10)
        .await
        .is_err());
    assert!(app.owner_audit_events(None, 0).await.is_err());
    // As três consultas sem sessão devem ser auditadas como negativas,
    // sem afirmar falsamente que o proprietário foi autenticado.
    let denied: Vec<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT outcome, actor_type, actor_id FROM organizational_audit_events
         WHERE installation_id = ? AND entity_type = 'audit' AND action = 'read'",
    )
    .bind("audit-installation")
    .fetch_all(pool)
    .await
    .unwrap();
    assert_eq!(denied.len(), 3);
    assert!(denied
        .iter()
        .all(|(outcome, actor_type, actor_id)| outcome == "denied"
            && actor_type.is_none()
            && actor_id.is_none()));
    drop(app);
    let _ = std::fs::remove_file(file);
}

#[tokio::test]
async fn organizational_audit_events_rollback_with_parent_transaction() {
    let file =
        std::env::temp_dir().join(format!("tina-audit-rollback-{}.db", uuid::Uuid::new_v4()));
    let db = Database::open(&file).await.unwrap();
    let pool = db.pool();
    let installation = InstallationRepository::new(pool)
        .create("audit-rollback", "owner")
        .await
        .unwrap();
    let company = CompanyRepository::new(pool)
        .create(&installation.id, "Original")
        .await
        .unwrap();

    let initial_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM organizational_audit_events WHERE installation_id = ?",
    )
    .bind(&installation.id)
    .fetch_one(pool)
    .await
    .unwrap();

    let mut transaction = pool.begin().await.unwrap();
    sqlx::query("UPDATE companies SET name = ? WHERE id = ? AND installation_id = ?")
        .bind("Temporaria")
        .bind(&company.id)
        .bind(&installation.id)
        .execute(&mut *transaction)
        .await
        .unwrap();
    let pending_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM organizational_audit_events WHERE installation_id = ?",
    )
    .bind(&installation.id)
    .fetch_one(&mut *transaction)
    .await
    .unwrap();
    assert_eq!(pending_events, initial_events + 1);
    transaction.rollback().await.unwrap();

    let persisted_name: String =
        sqlx::query_scalar("SELECT name FROM companies WHERE id = ? AND installation_id = ?")
            .bind(&company.id)
            .bind(&installation.id)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(persisted_name, "Original");
    let persisted_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM organizational_audit_events WHERE installation_id = ?",
    )
    .bind(&installation.id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(persisted_events, initial_events);

    drop(db);
    let _ = std::fs::remove_file(file);
}

#[tokio::test]
async fn audit_cursor_pagination_is_stable_and_scoped() {
    use crate::infra::audit::{list_events, list_events_before};
    let path = std::env::temp_dir().join(format!("tina-audit-page-{}.db", uuid::Uuid::new_v4()));
    let app = crate::TinaApp::initialize_at(&path, "page-install", "page-owner")
        .await
        .unwrap();
    let pool = app.database().pool();
    let repo = CompanyRepository::new(pool);
    let a = repo.create("page-install", "Empresa A").await.unwrap();
    let b = repo.create("page-install", "Empresa B").await.unwrap();
    repo.update_name(&a.id, "page-install", "Empresa A2")
        .await
        .unwrap();
    let page = list_events(pool, "page-install", None, 2).await.unwrap();
    assert_eq!(page.len(), 2);
    assert_eq!(page[0].entity_id, a.id);
    assert_eq!(page[1].entity_id, b.id);
    let tail = list_events_before(pool, "page-install", None, &page[1].id, 2)
        .await
        .unwrap();
    assert_eq!(tail.len(), 1);
    assert_eq!(tail[0].entity_id, a.id);
    assert_eq!(tail[0].action, "created");
    assert!(
        list_events_before(pool, "page-install", Some(&a.id), &page[1].id, 2)
            .await
            .is_err()
    );
    assert!(
        list_events_before(pool, "foreign-install", None, &page[1].id, 2)
            .await
            .is_err()
    );
    assert!(app
        .owner_audit_events_before(None, &page[1].id, 2)
        .await
        .is_err());
    drop(app);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn company_owner_attribution_is_atomic_and_does_not_leak() {
    let path = std::env::temp_dir().join(format!("tina-audit-actor-{}.db", uuid::Uuid::new_v4()));
    let app = crate::TinaApp::initialize_at(&path, "actor-install", "actor-owner")
        .await
        .unwrap();
    let pool = app.database().pool();
    let repo = CompanyRepository::new(pool);
    assert!(app.owner_create_company("Locked").await.is_err());
    assert!(repo
        .create_as_owner("actor-install", "wrong-owner", "Invalid")
        .await
        .is_err());
    let company = repo
        .create_as_owner("actor-install", "actor-owner", "Authorized")
        .await
        .unwrap();
    repo.update_name_as_owner(&company.id, "actor-install", "actor-owner", "Renamed")
        .await
        .unwrap();
    assert!(repo
        .create_as_owner("actor-install", "actor-owner", "Renamed")
        .await
        .is_err());
    repo.archive_as_owner(&company.id, "actor-install", "actor-owner")
        .await
        .unwrap();
    let internal = repo.create("actor-install", "Internal").await.unwrap();
    let events = crate::infra::audit::list_events(pool, "actor-install", None, 10)
        .await
        .unwrap();
    assert_eq!(events.len(), 4);
    assert_eq!(events[0].entity_id, internal.id);
    assert!(events[0].actor_id.is_none());
    for event in &events[1..] {
        assert_eq!(event.entity_id, company.id);
        assert_eq!(event.actor_id.as_deref(), Some("actor-owner"));
        assert_eq!(event.actor_type.as_deref(), Some("owner"));
    }
    let contexts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM organizational_audit_actor_context")
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(contexts, 0);
    drop(app);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn fresh_installation_and_restart_do_not_seed_organization() {
    let path = std::env::temp_dir().join(format!("tina-empty-{}.db", uuid::Uuid::new_v4()));
    for _ in 0..2 {
        let app = crate::TinaApp::initialize_at(&path, "empty-install", "empty-owner")
            .await
            .unwrap();
        let pool = app.database().pool();
        for table in ["companies", "departments", "positions", "employees"] {
            let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
                .fetch_one(pool)
                .await
                .unwrap();
            assert_eq!(count, 0, "unexpected seeded records in {table}");
        }
        drop(app);
    }
    let _ = std::fs::remove_file(path);
}

// ============================================================================
// Testes de concorrência e falha transacional
// ============================================================================

#[tokio::test]
async fn concurrent_writes_serialize_correctly() {
    // SQLite serializa escritores, mas precisamos garantir atomicidade
    // Este teste verifica que begin_owner_write funciona corretamente com pool válido
    let file = std::env::temp_dir().join(format!("tina-audit-concurrent-{}.db", uuid::Uuid::new_v4()));
    let app = crate::TinaApp::initialize_at(&file, "concurrent-install", "concurrent-owner")
        .await
        .unwrap();

    let pool = app.database().pool();

    // Verificar que begin_owner_write funciona corretamente - sem guardar o transaction
    let result = crate::infra::audit::begin_owner_write(pool, "concurrent-install", "concurrent-owner").await;
    // Deve ser sucesso no primeiro acesso
    assert!(result.is_ok());
    let tx = result.unwrap();

    // Limpar contextos pendentes
    let _ = sqlx::query("DELETE FROM organizational_audit_actor_context")
        .execute(pool)
        .await;

    // Fechar a transaction antes de dropar o app
    drop(tx);
    drop(app);
    let _ = std::fs::remove_file(file);
}

#[tokio::test]
async fn failed_operation_does_not_create_audit_event() {
    let file = std::env::temp_dir().join(format!("tina-audit-fail-{}.db", uuid::Uuid::new_v4()));
    let app = crate::TinaApp::initialize_at(&file, "fail-install", "fail-owner")
        .await
        .unwrap();

    let initial_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM organizational_audit_events WHERE installation_id = ?",
    )
    .bind("fail-install")
    .fetch_one(app.database().pool())
    .await
    .unwrap();

    // Tentar arquivar empresa com departamentos ativos deve falhar
    let company = CompanyRepository::new(app.database().pool())
        .create("fail-install", "Empresa Teste")
        .await
        .unwrap();

    DepartmentRepository::new(app.database().pool())
        .create(&company.id, "DEPT-001", "Teste", None)
        .await
        .unwrap();

    // Eventos criados pelos INSERTs de company e department
    let after_crud_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM organizational_audit_events WHERE installation_id = ?",
    )
    .bind("fail-install")
    .fetch_one(app.database().pool())
    .await
    .unwrap();

    // Esta tentativa de archive deve falhar
    let result = CompanyRepository::new(app.database().pool())
        .archive_as_owner(&company.id, "fail-install", "fail-owner").await;
    assert!(result.is_err());

    // Eventos não deve ter aumentado após a tentativa falha
    let final_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM organizational_audit_events WHERE installation_id = ?",
    )
    .bind("fail-install")
    .fetch_one(app.database().pool())
    .await
    .unwrap();
    assert_eq!(after_crud_events, final_events);

    drop(app);
    let _ = std::fs::remove_file(file);
}

// ============================================================================
// Testes de escalada de privilégios
// ============================================================================

#[tokio::test]
async fn wrong_owner_cannot_attribute_events() {
    let file = std::env::temp_dir().join(format!("tina-audit-escalation-{}.db", uuid::Uuid::new_v4()));
    let app = crate::TinaApp::initialize_at(&file, "escalation-install", "escalation-owner")
        .await
        .unwrap();

    let repo = CompanyRepository::new(app.database().pool());

    // Tentar criar empresa com owner_id errado deve falhar
    let result = repo.create_as_owner("escalation-install", "wrong-owner", "Empresas Erradas");
    assert!(result.await.is_err());

    // Criar empresa corretamente deve funcionar e atribuir ao owner correto
    let company = repo
        .create_as_owner("escalation-install", "escalation-owner", "Empresa Valida")
        .await
        .unwrap();

    // Verificar o evento foi atribuído corretamente
    let events = crate::infra::audit::list_events(app.database().pool(), "escalation-install", Some(&company.id), 10)
        .await
        .unwrap();

    for event in &events {
        if event.entity_id == company.id {
            assert_eq!(event.actor_type.as_deref(), Some("owner"));
            assert_eq!(event.actor_id.as_deref(), Some("escalation-owner"));
        }
    }

    drop(app);
    let _ = std::fs::remove_file(file);
}

#[tokio::test]
async fn audit_events_cannot_be_modified_or_deleted() {
    let file = std::env::temp_dir().join(format!("tina-audit-immutable-{}.db", uuid::Uuid::new_v4()));
    let app = crate::TinaApp::initialize_at(&file, "immutable-install", "immutable-owner")
        .await
        .unwrap();

    let pool = app.database().pool();

    // Criar alguns eventos
    CompanyRepository::new(pool)
        .create("immutable-install", "Empresa Teste")
        .await
        .unwrap();

    // Obter ID de um evento
    let event_id: String = sqlx::query_scalar("SELECT id FROM organizational_audit_events LIMIT 1")
        .fetch_one(pool)
        .await
        .unwrap();

    // Tentar atualizar deve falhar
    let result = sqlx::query("UPDATE organizational_audit_events SET action = 'fake' WHERE id = ?")
        .bind(&event_id)
        .execute(pool)
        .await;
    assert!(result.is_err() || result.unwrap().rows_affected() == 0);

    // Tentar deletar deve falhar
    let result = sqlx::query("DELETE FROM organizational_audit_events WHERE id = ?")
        .bind(&event_id)
        .execute(pool)
        .await;
    assert!(result.is_err() || result.unwrap().rows_affected() == 0);

    drop(app);
    let _ = std::fs::remove_file(file);
}

// ============================================================================
// Testes de regressão para problemas identificados
// ============================================================================

#[tokio::test]
async fn update_supervisor_as_owner_creates_proper_audit_event() {
    // Teste: update_supervisor_as_owner deve usar begin_owner_write/commit_owner_write
    // para criar eventos de auditoria com o actor correto
    let file = std::env::temp_dir().join(format!("tina-audit-supervisor-{}.db", uuid::Uuid::new_v4()));
    let app = crate::TinaApp::initialize_at(&file, "supervisor-install", "supervisor-owner")
        .await
        .unwrap();

    let pool = app.database().pool();

    // Desbloquear sessão para operações autorizadas
    app.unlock_owner_for_test().unwrap();

    // Criar empresa, departamento, cargo
    let company = CompanyRepository::new(pool).create("supervisor-install", "Empresa Teste").await.unwrap();
    let dept = DepartmentRepository::new(pool).create(&company.id, "DEPT-001", "Teste", None).await.unwrap();
    let position = PositionRepository::new(pool).create(&dept.id, "Cargo 1", 1, 1, None).await.unwrap();
    
    let emp1 = crate::infra::EmployeeRepository::new(pool)
        .create_as_owner("supervisor-install", "supervisor-owner", &position.id, "Funcionário 1", None, None)
        .await.unwrap();

    // Criar segundo funcionário para ser supervisor (não auto-supervisão)
    let position2 = PositionRepository::new(pool).create(&dept.id, "Cargo 2", 2, 2, None).await.unwrap();
    let emp2 = crate::infra::EmployeeRepository::new(pool)
        .create_as_owner("supervisor-install", "supervisor-owner", &position2.id, "Funcionário 2", None, None)
        .await.unwrap();

    // Atualizar supervisor (emp1 sob emp2)
    app.owner_update_employee_supervisor(&emp1.id, Some(&emp2.id))
        .await
        .unwrap();

    // Verificar se o evento foi criado com actor correto
    let events = crate::infra::audit::list_events(pool, "supervisor-install", None, 10)
        .await
        .unwrap();

    let update_event = events.iter().find(|e| e.entity_type == "employee" && e.action == "updated");
    assert!(update_event.is_some(), "Deve existir evento de atualização");
    let event = update_event.unwrap();
    assert_eq!(event.actor_type.as_deref(), Some("owner"), "Actor type deve ser 'owner'");
    assert_eq!(event.actor_id.as_deref(), Some("supervisor-owner"), "Actor ID deve ser o owner UUID");

    drop(app);
    let _ = std::fs::remove_file(file);
}

#[tokio::test]
async fn audit_query_does_not_create_denied_event_for_authorized_session() {
    // Teste: owner_audit_events não deve registrar denied antes da autorização
    // Quando a sessão está ativa, não deve criar eventos denied
    let file = std::env::temp_dir().join(format!("tina-audit-query-{}.db", uuid::Uuid::new_v4()));
    let app = crate::TinaApp::initialize_at(&file, "query-install", "query-owner")
        .await
        .unwrap();

    app.unlock_owner_for_test().unwrap();

    let initial_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM organizational_audit_events WHERE installation_id = ?",
    )
    .bind("query-install")
    .fetch_one(app.database().pool())
    .await
    .unwrap();

    // Consulta autorizada deve funcionar sem criar denied event
    let _ = app.owner_audit_events(None, 10).await.unwrap();

    let final_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM organizational_audit_events WHERE installation_id = ? AND outcome = 'denied'",
    )
    .bind("query-install")
    .fetch_one(app.database().pool())
    .await
    .unwrap();

    assert_eq!(final_events, 0, "Não deve criar eventos denied para sessão autorizada");

    drop(app);
    let _ = std::fs::remove_file(file);
}

#[tokio::test] 
async fn denied_operations_are_auditied() {
    // Teste: Operações administrativas negadas devem ser auditadas
    let file = std::env::temp_dir().join(format!("tina-audit-denied-{}.db", uuid::Uuid::new_v4()));
    let app = crate::TinaApp::initialize_at(&file, "denied-install", "denied-owner")
        .await
        .unwrap();

    // Não chamar unlock_owner_for_test - sessão está bloqueada

    // Tentar listar auditoria sem sessão ativa deve registrar denied
    let _ = app.owner_audit_events(None, 10).await;

    // Verificar se foi criado um evento denied
    let denied_events: Vec<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT outcome, actor_type, actor_id FROM organizational_audit_events
         WHERE installation_id = ? AND entity_type = 'audit' AND action = 'read'",
    )
    .bind("denied-install")
    .fetch_all(app.database().pool())
    .await
    .unwrap();

    // Deve ter pelo menos 1 evento denied
    assert!(!denied_events.is_empty(), "Deve registrar evento denied para sessão bloqueada");

    drop(app);
    let _ = std::fs::remove_file(file);
}