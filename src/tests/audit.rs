//! Contratos de auditoria organizacional: atomicidade, isolamento e imutabilidade.
use crate::infra::{
    CompanyRepository, DepartmentRepository, InstallationRepository, PositionRepository,
};
use crate::Database;

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
