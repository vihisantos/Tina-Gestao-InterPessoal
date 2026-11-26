use sqlx::{Row, SqlitePool};

#[derive(Debug, serde::Serialize)]
pub(crate) struct ProfessionalRecord {
    pub id: String,
    pub category: String,
    pub description: String,
    pub evidence_ref: Option<String>,
    pub evidence_status: String,
    pub supersedes_id: Option<String>,
    pub recorded_by: String,
    pub recorded_at: String,
}

/// Internal repository: caller must authenticate and authorize the owner first.
pub(crate) async fn list_records(
    pool: &SqlitePool,
    installation_id: &str,
    company_id: &str,
    employee_id: &str,
) -> Result<Vec<ProfessionalRecord>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT r.* FROM employee_professional_records r
         JOIN employees e ON e.id=r.employee_id
         JOIN positions p ON p.id=e.position_id
         JOIN departments d ON d.id=p.department_id
         JOIN companies c ON c.id=d.company_id
         WHERE c.installation_id=? AND c.id=? AND e.id=?
         ORDER BY r.recorded_at DESC, r.id DESC",
    )
    .bind(installation_id)
    .bind(company_id)
    .bind(employee_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(|row| {
        Ok(ProfessionalRecord {
            id: row.try_get("id")?,
            category: row.try_get("category")?,
            description: row.try_get("description")?,
            evidence_ref: row.try_get("evidence_ref")?,
            evidence_status: row.try_get("evidence_status")?,
            supersedes_id: row.try_get("supersedes_id")?,
            recorded_by: row.try_get("recorded_by")?,
            recorded_at: row.try_get("recorded_at")?,
        })
    }).collect()
}

pub(crate) struct NewRecord<'a> {
    pub category: &'a str,
    pub description: &'a str,
    pub evidence_ref: Option<&'a str>,
    pub supersedes_id: Option<&'a str>,
}

/// New claims are unverified by default. Actor must come from authenticated context.
pub(crate) async fn append_record(
    pool: &SqlitePool,
    installation_id: &str,
    company_id: &str,
    employee_id: &str,
    actor: &str,
    record: NewRecord<'_>,
) -> Result<String, sqlx::Error> {
    if actor.trim().is_empty() {
        return Err(sqlx::Error::Protocol("missing authenticated actor".into()));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let result = sqlx::query(
        "INSERT INTO employee_professional_records
         (id,employee_id,category,description,evidence_ref,supersedes_id,recorded_by)
         SELECT ?,e.id,?,?,?,?,? FROM employees e
         JOIN positions p ON p.id=e.position_id
         JOIN departments d ON d.id=p.department_id
         JOIN companies c ON c.id=d.company_id
         WHERE c.installation_id=? AND c.id=? AND e.id=?",
    )
    .bind(&id)
    .bind(record.category)
    .bind(record.description)
    .bind(record.evidence_ref)
    .bind(record.supersedes_id)
    .bind(actor)
    .bind(installation_id)
    .bind(company_id)
    .bind(employee_id)
    .execute(pool)
    .await?;
    if result.rows_affected() != 1 {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(id)
}
