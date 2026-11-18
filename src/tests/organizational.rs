//! Testes de integração para a fundação organizacional
//! 
//! Executar com: cargo test organizational_tests

use std::time::{SystemTime, UNIX_EPOCH};
use std::path::Path;
use crate::{Database};
use crate::infra::{InstallationRepository, CompanyRepository, DepartmentRepository, PositionRepository};

/// Helper para criar banco de teste único para cada teste
async fn setup_test_db(test_name: &str) -> (Database, String) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    
    let path = std::env::temp_dir().join(format!("tina-test-{}-{}.db", test_name, timestamp));
    let path_str = path.to_string_lossy().to_string();
    
    let db = Database::open(Path::new(&path)).await
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
    let installation = repo.create("Test Installation", "owner-test-uuid").await
        .expect("Failed to create installation");
    
    assert_eq!(installation.name, "Test Installation");
    assert_eq!(installation.owner_uuid, "owner-test-uuid");
    
    // Buscar pelo ID
    let found = repo.find_by_id(&installation.id).await
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
    let installation = repo.create("Test Company Installation", "owner-test-uuid").await
        .expect("Failed to create installation");
    
    // Criar empresa
    let company_repo = CompanyRepository::new(db.pool());
    let company = company_repo.create(&installation.id, "Empresa Teste").await
        .expect("Failed to create company");
    
    assert_eq!(company.name, "Empresa Teste");
    assert!(company.is_active());
    
    // Atualizar nome
    company_repo.update_name(&company.id, "Empresa Atualizada").await
        .expect("Failed to update company");
    
    let updated = company_repo.find_by_id(&company.id).await
        .expect("Failed to find updated company");
    assert_eq!(updated.name, "Empresa Atualizada");
    
    // Arquivar
    company_repo.archive(&company.id).await
        .expect("Failed to archive company");
    
    let archived = company_repo.find_by_id(&company.id).await
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
    let installation = repo.create("Test Department Installation", "owner-test-uuid").await
        .expect("Failed to create installation");
    
    let company_repo = CompanyRepository::new(db.pool());
    let company = company_repo.create(&installation.id, "Empresa Teste Dept").await
        .expect("Failed to create company");
    
    // Criar departamento
    let dept_repo = DepartmentRepository::new(db.pool());
    let dept = dept_repo.create(
        &company.id,
        "DEPT-001",
        "Recursos Humanos",
        Some("Departamento de RH"),
    ).await.expect("Failed to create department");
    
    assert_eq!(dept.code, "DEPT-001");
    assert_eq!(dept.name, "Recursos Humanos");
    assert!(dept_repo.belongs_to_company(&dept.id, &company.id).await
        .expect("Failed to check department ownership"));
    
    // Listar departamentos
    let depts = dept_repo.list_by_company(&company.id).await
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
    let installation = repo.create("Test Position Installation", "owner-test-uuid").await
        .expect("Failed to create installation");
    
    let company_repo = CompanyRepository::new(db.pool());
    let company = company_repo.create(&installation.id, "Empresa Teste Pos").await
        .expect("Failed to create company");
    
    let dept_repo = DepartmentRepository::new(db.pool());
    let dept = dept_repo.create(
        &company.id,
        "POS-001",
        "Tecnologia",
        Some("Departamento de TI"),
    ).await.expect("Failed to create department");
    
    // Criar cargo
    let pos_repo = PositionRepository::new(db.pool());
    let position = pos_repo.create(
        &dept.id,
        "Analista de Sistemas",
        2,
        500,
        Some("Desenvolve sistemas"),
    ).await.expect("Failed to create position");
    
    assert_eq!(position.name, "Analista de Sistemas");
    assert_eq!(position.level, 2);
    assert!(pos_repo.belongs_to_company(&position.id, &company.id).await
        .expect("Failed to check position ownership"));
    
    cleanup_test_db(&path);
}

#[tokio::test]
async fn test_cross_company_isolation() {
    let test_name = "isolation";
    let (db, path) = setup_test_db(test_name).await;
    let repo = InstallationRepository::new(db.pool());
    
    // Criar instalação única
    let installation = repo.create("Isolation Test Installation", "owner-isolation-test").await
        .expect("Failed to create installation");
    
    let company_repo = CompanyRepository::new(db.pool());
    
    // Criar duas empresas
    let company_a = company_repo.create(&installation.id, "Empresa A").await
        .expect("Failed to create company A");
    let company_b = company_repo.create(&installation.id, "Empresa B").await
        .expect("Failed to create company B");
    
    // Criar departamento em empresa A
    let dept_repo = DepartmentRepository::new(db.pool());
    let dept_a = dept_repo.create(
        &company_a.id,
        "DEPT-A",
        "Departamento A",
        None,
    ).await.expect("Failed to create department A");
    
    // Criar cargo em departamento A
    let pos_repo = PositionRepository::new(db.pool());
    let position_a = pos_repo.create(
        &dept_a.id,
        "Cargo A",
        1,
        500,
        None,
    ).await.expect("Failed to create position A");
    
    // Verificar isolamento:
    // 1. Departamento B NÃO deve existir
    let dept_b_exists = dept_repo.find_by_id_optional("non-existent-dept").await
        .expect("Failed to check department")
        .is_some();
    assert!(!dept_b_exists);
    
    // 2. Cargo deve pertencer a empresa A
    let belongs_to_a = pos_repo.belongs_to_company(&position_a.id, &company_a.id).await
        .expect("Failed to check position ownership");
    assert!(belongs_to_a);
    
    // 3. Cargo NÃO deve pertencer a empresa B
    let belongs_to_b = pos_repo.belongs_to_company(&position_a.id, &company_b.id).await
        .expect("Failed to check position ownership against B");
    assert!(!belongs_to_b);
    
    // 4. Departamento NÃO deve pertencer a empresa B
    let dept_belongs_to_b = dept_repo.belongs_to_company(&dept_a.id, &company_b.id).await
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
    let installation = repo.create("Persistence Test", "owner-persistence").await
        .expect("Failed to create installation");
    
    let company_repo = CompanyRepository::new(db.pool());
    let company = company_repo.create(&installation.id, "Empresa Persistente").await
        .expect("Failed to create company");
    
    // Simular "reinicialização" fechando e reabrindo o banco
    drop(db);
    
    // Reabrir banco
    let db2 = Database::open(Path::new(&path)).await
        .expect("Failed to reopen database");
    
    // Verificar dados persistentes
    let repo2 = InstallationRepository::new(db2.pool());
    let found_inst = repo2.find_by_id(&installation.id).await
        .expect("Failed to find installation after reopen")
        .expect("Installation not found after reopen");
    
    assert_eq!(found_inst.id, installation.id);
    assert_eq!(found_inst.name, installation.name);
    
    let company_repo2 = CompanyRepository::new(db2.pool());
    let found_company = company_repo2.find_by_id_optional(&company.id).await
        .expect("Failed to find company after reopen")
        .expect("Company not found after reopen");
    
    assert_eq!(found_company.name, company.name);
    
    cleanup_test_db(&path);
}