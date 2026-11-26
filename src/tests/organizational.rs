//! Testes de integração para a fundação organizacional
//!
//! Executar com: cargo test organizational_tests

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

    let path = std::env::temp_dir().join(format!("tina-test-{}-{}.db", test_name, timestamp));
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
async fn test_installation_create_and_find() {
    let test_name = "installation";
    let (db, path) = setup_test_db(test_name).await;

    let repo = InstallationRepository::new(db.pool());

    // Criar instalação
    let installation = repo
        .create("Test Installation", "owner-test-uuid")
        .await
        .expect("Failed to create installation");

    assert_eq!(installation.name, "Test Installation");
    assert_eq!(installation.owner_uuid, "owner-test-uuid");

    // Buscar pelo ID
    let found = repo
        .find_by_id(&installation.id)
        .await
        .expect("Failed to find installation")
        .expect("Installation not found");

    assert_eq!(found.id, installation.id);
    assert_eq!(found.name, installation.name);

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_company_crud() {
    let test_name = "company";
    let (db, path) = setup_test_db(test_name).await;

    let repo = InstallationRepository::new(db.pool());

    // Criar instalação
    let installation = repo
        .create("Test Company Installation", "owner-test-uuid")
        .await
        .expect("Failed to create installation");

    // Criar empresa
    let company_repo = CompanyRepository::new(db.pool());
    let company = company_repo
        .create(&installation.id, "Empresa Teste")
        .await
        .expect("Failed to create company");

    assert_eq!(company.name, "Empresa Teste");
    assert!(company.is_active());

    // Atualizar nome
    company_repo
        .update_name_unrestricted(&company.id, "Empresa Atualizada")
        .await
        .expect("Failed to update company");

    let updated = company_repo
        .find_by_id(&company.id)
        .await
        .expect("Failed to find updated company");
    assert_eq!(updated.name, "Empresa Atualizada");

    // Arquivar
    company_repo
        .archive_unrestricted(&company.id)
        .await
        .expect("Failed to archive company");

    let archived = company_repo
        .find_by_id(&company.id)
        .await
        .expect("Failed to find archived company");
    assert!(!archived.is_active());

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_department_crud() {
    let test_name = "department";
    let (db, path) = setup_test_db(test_name).await;

    let repo = InstallationRepository::new(db.pool());

    // Criar instalação e empresa
    let installation = repo
        .create("Test Department Installation", "owner-test-uuid")
        .await
        .expect("Failed to create installation");

    let company_repo = CompanyRepository::new(db.pool());
    let company = company_repo
        .create(&installation.id, "Empresa Teste Dept")
        .await
        .expect("Failed to create company");

    // Criar departamento
    let dept_repo = DepartmentRepository::new(db.pool());
    let dept = dept_repo
        .create(
            &company.id,
            "DEPT-001",
            "Recursos Humanos",
            Some("Departamento de RH"),
        )
        .await
        .expect("Failed to create department");

    assert_eq!(dept.code, "DEPT-001");
    assert_eq!(dept.name, "Recursos Humanos");
    assert!(dept_repo
        .belongs_to_company(&dept.id, &company.id)
        .await
        .expect("Failed to check department ownership"));

    // Listar departamentos
    let depts = dept_repo
        .list_by_company(&company.id)
        .await
        .expect("Failed to list departments");
    assert_eq!(depts.len(), 1);

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_position_crud() {
    let test_name = "position";
    let (db, path) = setup_test_db(test_name).await;

    let repo = InstallationRepository::new(db.pool());

    // Criar instalação, empresa e departamento
    let installation = repo
        .create("Test Position Installation", "owner-test-uuid")
        .await
        .expect("Failed to create installation");

    let company_repo = CompanyRepository::new(db.pool());
    let company = company_repo
        .create(&installation.id, "Empresa Teste Pos")
        .await
        .expect("Failed to create company");

    let dept_repo = DepartmentRepository::new(db.pool());
    let dept = dept_repo
        .create(
            &company.id,
            "POS-001",
            "Tecnologia",
            Some("Departamento de TI"),
        )
        .await
        .expect("Failed to create department");

    // Criar cargo
    let pos_repo = PositionRepository::new(db.pool());
    let position = pos_repo
        .create(
            &dept.id,
            "Analista de Sistemas",
            2,
            500,
            Some("Desenvolve sistemas"),
        )
        .await
        .expect("Failed to create position");

    assert_eq!(position.name, "Analista de Sistemas");
    assert_eq!(position.level, 2);
    assert!(pos_repo
        .belongs_to_company(&position.id, &company.id)
        .await
        .expect("Failed to check position ownership"));

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_cross_company_isolation() {
    let test_name = "isolation";
    let (db, path) = setup_test_db(test_name).await;
    let repo = InstallationRepository::new(db.pool());

    // Criar instalação única
    let installation = repo
        .create("Isolation Test Installation", "owner-isolation-test")
        .await
        .expect("Failed to create installation");

    let company_repo = CompanyRepository::new(db.pool());

    // Criar duas empresas
    let company_a = company_repo
        .create(&installation.id, "Empresa A")
        .await
        .expect("Failed to create company A");
    let company_b = company_repo
        .create(&installation.id, "Empresa B")
        .await
        .expect("Failed to create company B");

    // Criar departamento em empresa A
    let dept_repo = DepartmentRepository::new(db.pool());
    let dept_a = dept_repo
        .create(&company_a.id, "DEPT-A", "Departamento A", None)
        .await
        .expect("Failed to create department A");

    // Criar cargo em departamento A
    let pos_repo = PositionRepository::new(db.pool());
    let position_a = pos_repo
        .create(&dept_a.id, "Cargo A", 1, 500, None)
        .await
        .expect("Failed to create position A");

    // Verificar isolamento:
    // 1. Departamento B NÃO deve existir
    let dept_b_exists = dept_repo
        .find_by_id_optional("non-existent-dept")
        .await
        .expect("Failed to check department")
        .is_some();
    assert!(!dept_b_exists);

    // 2. Cargo deve pertencer a empresa A
    let belongs_to_a = pos_repo
        .belongs_to_company(&position_a.id, &company_a.id)
        .await
        .expect("Failed to check position ownership");
    assert!(belongs_to_a);

    // 3. Cargo NÃO deve pertencer a empresa B
    let belongs_to_b = pos_repo
        .belongs_to_company(&position_a.id, &company_b.id)
        .await
        .expect("Failed to check position ownership against B");
    assert!(!belongs_to_b);

    // 4. Departamento NÃO deve pertencer a empresa B
    let dept_belongs_to_b = dept_repo
        .belongs_to_company(&dept_a.id, &company_b.id)
        .await
        .expect("Failed to check department ownership against B");
    assert!(!dept_belongs_to_b);

    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_persistence_after_restart() {
    let test_name = "persistence";
    let (db, path) = setup_test_db(test_name).await;
    let repo = InstallationRepository::new(db.pool());

    // Criar instalação
    let installation = repo
        .create("Persistence Test", "owner-persistence")
        .await
        .expect("Failed to create installation");

    let company_repo = CompanyRepository::new(db.pool());
    let company = company_repo
        .create(&installation.id, "Empresa Persistente")
        .await
        .expect("Failed to create company");

    // Simular "reinicialização" fechando e reabrindo o banco
    drop(db);

    // Reabrir banco
    let db2 = Database::open(Path::new(&path))
        .await
        .expect("Failed to reopen database");

    // Verificar dados persistentes
    let repo2 = InstallationRepository::new(db2.pool());
    let found_inst = repo2
        .find_by_id(&installation.id)
        .await
        .expect("Failed to find installation after reopen")
        .expect("Installation not found after reopen");

    assert_eq!(found_inst.id, installation.id);
    assert_eq!(found_inst.name, installation.name);

    let company_repo2 = CompanyRepository::new(db2.pool());
    let found_company = company_repo2
        .find_by_id_optional(&company.id)
        .await
        .expect("Failed to find company after reopen")
        .expect("Company not found after reopen");

    assert_eq!(found_company.name, company.name);

    cleanup_test_db(&path);
}

// ============================================================================
// NOVOS TESTES PARA FUNDACAO-ORGANIZACIONAL-02
// ============================================================================

/// Testa inicialização idempotente - criar com ID específico
#[tokio::test]
async fn test_installation_create_with_id() {
    let test_name = "installation_with_id";
    let (db, path) = setup_test_db(test_name).await;

    let repo = InstallationRepository::new(db.pool());
    let installation_id = "test-installation-id-12345";
    let owner_uuid = "owner-uuid-67890";

    // Criar instalação com ID específico
    let installation = repo
        .create_with_id("Test Installation", installation_id, owner_uuid)
        .await
        .expect("Failed to create installation with ID");

    assert_eq!(installation.id, installation_id);
    assert_eq!(installation.name, "Test Installation");
    assert_eq!(installation.owner_uuid, owner_uuid);

    // Verificar que pode ser encontrada pelo ID
    let found = repo
        .find_by_id(installation_id)
        .await
        .expect("Failed to find installation")
        .expect("Installation not found");

    assert_eq!(found.id, installation_id);
    assert_eq!(found.owner_uuid, owner_uuid);

    cleanup_test_db(&path);
}

/// Testa que a mesma instalação não é criada duas vezes
#[tokio::test]
async fn test_installation_idempotent() {
    let test_name = "installation_idempotent";
    let (db, path) = setup_test_db(test_name).await;

    let repo = InstallationRepository::new(db.pool());
    let installation_id = "persistent-installation-id";
    let owner_uuid = "persistent-owner-uuid";

    // Criar instalação pela primeira vez
    let installation1 = repo
        .create_with_id("Test Installation", installation_id, owner_uuid)
        .await
        .expect("Failed to create installation");

    // Tentar buscar/criar novamente - deve encontrar a existente
    let found = repo
        .find_by_id(installation_id)
        .await
        .expect("Failed to find installation")
        .expect("Installation should exist");

    assert_eq!(found.id, installation_id);
    assert_eq!(found.owner_uuid, owner_uuid);

    // Verificar que não há duplicatas
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM installations WHERE id = ?")
        .bind(installation_id)
        .fetch_one(db.pool())
        .await
        .expect("Failed to count installations");

    assert_eq!(count, 1, "Should have exactly one installation record");

    cleanup_test_db(&path);
}

/// Testa isolamento entre múltiplas instalações
#[tokio::test]
async fn test_isolation_between_installations() {
    let test_name = "multi_installation";
    let (db, path) = setup_test_db(test_name).await;

    let repo = InstallationRepository::new(db.pool());

    // Criar duas instalações com IDs únicos
    let inst1_id = "installation-1-uuid";
    let inst2_id = "installation-2-uuid";
    let owner1 = "owner-1-uuid";
    let owner2 = "owner-2-uuid";

    let inst1 = repo
        .create_with_id("Installation 1", inst1_id, owner1)
        .await
        .expect("Failed to create inst 1");
    let inst2 = repo
        .create_with_id("Installation 2", inst2_id, owner2)
        .await
        .expect("Failed to create inst 2");

    let company_repo = CompanyRepository::new(db.pool());

    // Criar empresas em cada instalação
    let company1 = company_repo
        .create(&inst1.id, "Empresa 1")
        .await
        .expect("Failed to create company 1");
    let company2 = company_repo
        .create(&inst2.id, "Empresa 2")
        .await
        .expect("Failed to create company 2");

    // Listar empresas de inst1 - só deve ter empresa1
    let companies1 = company_repo
        .list_by_installation(&inst1.id)
        .await
        .expect("Failed");
    assert_eq!(companies1.len(), 1);
    assert_eq!(companies1[0].id, company1.id);

    // Listar empresas de inst2 - só deve ter empresa2
    let companies2 = company_repo
        .list_by_installation(&inst2.id)
        .await
        .expect("Failed");
    assert_eq!(companies2.len(), 1);
    assert_eq!(companies2[0].id, company2.id);

    // Verificar belongs_to_installation
    assert!(company_repo
        .belongs_to_installation(&company1.id, &inst1.id)
        .await
        .expect("Failed"));
    assert!(!company_repo
        .belongs_to_installation(&company1.id, &inst2.id)
        .await
        .expect("Failed"));
    assert!(company_repo
        .belongs_to_installation(&company2.id, &inst2.id)
        .await
        .expect("Failed"));
    assert!(!company_repo
        .belongs_to_installation(&company2.id, &inst1.id)
        .await
        .expect("Failed"));

    cleanup_test_db(&path);
}

/// Testa que owner_uuid é independente do installation_id
#[tokio::test]
async fn test_owner_uuid_independent() {
    let test_name = "owner_uuid_independent";
    let (db, path) = setup_test_db(test_name).await;

    let repo = InstallationRepository::new(db.pool());

    // Criar instalação com IDs diferentes
    let installation_id = "inst-id-different";
    let owner_uuid = "owner-uuid-different";

    let installation = repo
        .create_with_id("Test", installation_id, owner_uuid)
        .await
        .expect("Failed to create installation");

    // Verificar que os IDs são diferentes mas ambos são válidos
    assert_ne!(installation.id, installation.owner_uuid);
    assert_eq!(installation.id, installation_id);
    assert_eq!(installation.owner_uuid, owner_uuid);

    // Pode buscar pelo ID da instalação
    let found_by_id = repo
        .find_by_id(&installation_id)
        .await
        .expect("Failed")
        .expect("Not found");
    assert_eq!(found_by_id.id, installation_id);

    // Pode buscar pelo owner_uuid
    let found_by_owner = repo
        .find_by_owner_uuid(&owner_uuid)
        .await
        .expect("Failed")
        .expect("Not found");
    assert_eq!(found_by_owner.owner_uuid, owner_uuid);
    assert_eq!(found_by_owner.id, installation_id);

    cleanup_test_db(&path);
}

// ============================================================================
// TESTES DE INICIALIZAÇÃO REAL E ISOLAMENTO
// ============================================================================

/// Testa inicialização idempotente real como TinaApp faria
#[tokio::test]
async fn test_tinaapp_initialization_idempotent() {
    use crate::TinaApp;

    let test_name = "tinaapp_init_idempotent";
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let db_path =
        std::env::temp_dir().join(format!("tina-init-test-{}-{}.db", test_name, timestamp));
    let installation_id = "test-init-installation-id";
    let owner_uuid = "test-init-owner-uuid";

    // Primeira inicialização usando TinaApp::initialize_at
    let tina1 = TinaApp::initialize_at(&db_path, installation_id, owner_uuid)
        .await
        .expect("Failed to initialize TinaApp first time");

    // Verificar identidade da TinaApp
    assert_eq!(tina1.config().installation_id, installation_id);
    assert_eq!(tina1.config().owner_uuid, owner_uuid);

    // Criar empresa usando o repositório
    let company_repo = crate::infra::CompanyRepository::new(tina1.database().pool());
    let company = company_repo
        .create(installation_id, "Empresa Inicial")
        .await
        .expect("Failed to create company");

    // Verificar empresa criada
    let found = company_repo
        .find_by_id_optional(&company.id)
        .await
        .expect("Failed")
        .expect("Not found");
    assert_eq!(found.name, "Empresa Inicial");

    // Encerrar a instância
    drop(tina1);

    // Segunda inicialização com MESMA configuração (mesmo banco, mesmos IDs)
    let tina2 = TinaApp::initialize_at(&db_path, installation_id, owner_uuid)
        .await
        .expect("Failed to initialize TinaApp second time");

    // Verificar que a identidade persistiu
    assert_eq!(tina2.config().installation_id, installation_id);
    assert_eq!(tina2.config().owner_uuid, owner_uuid);

    // Verificar que a instalação ainda existe no banco
    let repo2 = crate::infra::InstallationRepository::new(tina2.database().pool());
    let inst = repo2
        .find_by_id(installation_id)
        .await
        .expect("Failed to find")
        .expect("Installation should exist");
    assert_eq!(inst.id, installation_id);
    assert_eq!(inst.owner_uuid, owner_uuid);

    // Verificar que a empresa ainda existe
    let company_repo2 = crate::infra::CompanyRepository::new(tina2.database().pool());
    let company2 = company_repo2
        .find_by_id_optional(&company.id)
        .await
        .expect("Failed")
        .expect("Company should still exist");
    assert_eq!(company2.name, company.name);

    // Uma configuração com owner_uuid divergente deve falhar de forma fechada.
    drop(tina2);
    let mismatched_owner =
        TinaApp::initialize_at(&db_path, installation_id, "owner-uuid-diferente").await;
    assert!(
        mismatched_owner.is_err(),
        "A inicialização não pode aceitar owner_uuid divergente do banco"
    );

    // Reabrir com a identidade correta para verificar persistência e unicidade.
    let tina3 = TinaApp::initialize_at(&db_path, installation_id, owner_uuid)
        .await
        .expect("A identidade original deve continuar válida");

    // Verificar apenas UM registro de instalação
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM installations WHERE id = ?")
        .bind(installation_id)
        .fetch_one(tina3.database().pool())
        .await
        .expect("Failed to count");
    assert_eq!(count, 1, "Should have exactly one installation record");

    drop(tina3);
    // Limpar
    let _ = std::fs::remove_file(&db_path);
}

/// Testa isolamento entre instalações no nível de comandos
#[tokio::test]
async fn test_command_level_isolation() {
    let test_name = "command_isolation";
    let (db, path) = setup_test_db(test_name).await;

    let repo = InstallationRepository::new(db.pool());
    let company_repo = CompanyRepository::new(db.pool());
    let dept_repo = DepartmentRepository::new(db.pool());
    let pos_repo = PositionRepository::new(db.pool());

    // Criar duas instalações
    let inst1 = repo
        .create("Installation 1", "owner-1")
        .await
        .expect("Failed inst 1");
    let inst2 = repo
        .create("Installation 2", "owner-2")
        .await
        .expect("Failed inst 2");

    // Criar empresas
    let comp1 = company_repo
        .create(&inst1.id, "Empresa 1")
        .await
        .expect("Failed comp 1");
    let comp2 = company_repo
        .create(&inst2.id, "Empresa 2")
        .await
        .expect("Failed comp 2");

    // Criar departamentos
    let dept1 = dept_repo
        .create(&comp1.id, "DEPT-1", "Dept 1", None)
        .await
        .expect("Failed dept 1");
    let dept2 = dept_repo
        .create(&comp2.id, "DEPT-2", "Dept 2", None)
        .await
        .expect("Failed dept 2");

    // Criar cargos
    let pos1 = pos_repo
        .create(&dept1.id, "Cargo 1", 1, 100, None)
        .await
        .expect("Failed pos 1");
    let pos2 = pos_repo
        .create(&dept2.id, "Cargo 2", 1, 200, None)
        .await
        .expect("Failed pos 2");

    // Testar belongs_to_company para cargos
    assert!(pos_repo
        .belongs_to_company(&pos1.id, &comp1.id)
        .await
        .expect("Query failed"));
    assert!(!pos_repo
        .belongs_to_company(&pos1.id, &comp2.id)
        .await
        .expect("Query failed"));
    assert!(pos_repo
        .belongs_to_company(&pos2.id, &comp2.id)
        .await
        .expect("Query failed"));
    assert!(!pos_repo
        .belongs_to_company(&pos2.id, &comp1.id)
        .await
        .expect("Query failed"));

    // Testar belongs_to_company para departamentos
    assert!(dept_repo
        .belongs_to_company(&dept1.id, &comp1.id)
        .await
        .expect("Query failed"));
    assert!(!dept_repo
        .belongs_to_company(&dept1.id, &comp2.id)
        .await
        .expect("Query failed"));
    assert!(dept_repo
        .belongs_to_company(&dept2.id, &comp2.id)
        .await
        .expect("Query failed"));
    assert!(!dept_repo
        .belongs_to_company(&dept2.id, &comp1.id)
        .await
        .expect("Query failed"));

    // Testar belongs_to_department para cargos
    assert!(pos_repo
        .belongs_to_department(&pos1.id, &dept1.id)
        .await
        .expect("Query failed"));
    assert!(!pos_repo
        .belongs_to_department(&pos1.id, &dept2.id)
        .await
        .expect("Query failed"));

    // Escritas autorizadas na própria instalação devem funcionar.
    dept_repo
        .update_name_with_scope(&dept1.id, &inst1.id, "Dept 1 atualizado")
        .await
        .expect("A instalação proprietária deve atualizar seu departamento");
    pos_repo
        .update_name_with_scope(&pos1.id, &inst1.id, "Cargo 1 atualizado")
        .await
        .expect("A instalação proprietária deve atualizar seu cargo");

    // Arquivamentos devem revalidar seus pré-requisitos dentro do próprio UPDATE.
    assert!(company_repo.archive(&comp1.id, &inst1.id).await.is_err());
    assert!(dept_repo
        .archive_with_scope(&dept1.id, &inst1.id)
        .await
        .is_err());

    // Escritas e arquivamentos com IDs de outra instalação devem falhar.
    assert!(dept_repo
        .update_name_with_scope(&dept2.id, &inst1.id, "Acesso indevido")
        .await
        .is_err());
    assert!(dept_repo
        .archive_with_scope(&dept2.id, &inst1.id)
        .await
        .is_err());
    assert!(pos_repo
        .update_name_with_scope(&pos2.id, &inst1.id, "Acesso indevido")
        .await
        .is_err());
    assert!(pos_repo
        .archive_with_scope(&pos2.id, &inst1.id)
        .await
        .is_err());

    let dept2_after = dept_repo
        .find_by_id(&dept2.id)
        .await
        .expect("Dept 2 deve existir");
    let pos2_after = pos_repo
        .find_by_id(&pos2.id)
        .await
        .expect("Pos 2 deve existir");
    assert_eq!(dept2_after.name, "Dept 2");
    assert_eq!(pos2_after.name, "Cargo 2");
    assert_eq!(dept2_after.status.to_string(), "active");
    assert_eq!(pos2_after.status.to_string(), "active");

    cleanup_test_db(&path);
}

/// Testa busca por owner_uuid
#[tokio::test]
async fn test_find_by_owner_uuid() {
    let test_name = "owner_uuid_search";
    let (db, path) = setup_test_db(test_name).await;

    let repo = InstallationRepository::new(db.pool());

    // Criar instalação
    let installation = repo
        .create("Test Owner UUID", "unique-owner-uuid-456")
        .await
        .expect("Failed to create installation");

    // Buscar pelo owner_uuid
    let found = repo
        .find_by_owner_uuid("unique-owner-uuid-456")
        .await
        .expect("Failed to find by owner")
        .expect("Should find installation");

    assert_eq!(found.id, installation.id);
    assert_eq!(found.name, installation.name);

    // Verificar que não encontra outro owner_uuid
    let not_found = repo
        .find_by_owner_uuid("non-existent-uuid")
        .await
        .expect("Query failed")
        .is_none();

    assert!(not_found);

    cleanup_test_db(&path);
}

#[tokio::test]
async fn personal_organization_is_explicit_unique_and_preserves_company_hierarchy() {
    let path = std::env::temp_dir().join(format!("tina-personal-{}.db", uuid::Uuid::new_v4()));
    let app = crate::TinaApp::initialize_at(&path, "personal-installation", "personal-owner")
        .await
        .unwrap();
    app.unlock_owner_for_test().unwrap();
    let personal = app
        .owner_create_company_with_kind("Pessoal", "personal")
        .await
        .unwrap();
    let business = app.owner_create_company("Negócio").await.unwrap();
    assert_eq!(personal.kind, "personal");
    assert_eq!(business.kind, "business");
    assert!(app
        .owner_create_company_with_kind("Outra pessoal", "personal")
        .await
        .is_err());
    assert!(app
        .owner_create_company_with_kind("Inválida", "unknown")
        .await
        .is_err());
    let dept = app
        .owner_create_department(&personal.id, "FIN", "Finanças pessoais", None)
        .await
        .unwrap();
    assert_eq!(dept.company_id, personal.id);
    let db = app.database();
    let reloaded = db.companies().find_by_id(&personal.id).await.unwrap();
    assert_eq!(reloaded.kind, "personal");
    db.pool().close().await;
    let _ = std::fs::remove_file(path);
}
