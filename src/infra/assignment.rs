//! Movimentação de cargo/departamento: validação, transação e histórico.
//!
//! A posição atual vive em `employees.position_id`. O departamento é derivado
//! dessa posição; por isso a mudança de cargo registra também os departamentos
//! anterior e novo, cobrindo transferências entre departamentos.
//!
//! A atualização do cargo e a inserção em `employee_assignment_history`
//! ocorrem na mesma transação (`BEGIN IMMEDIATE`), com `ROLLBACK` em falha.

use sqlx::sqlite::SqlitePool;
use sqlx::SqliteConnection;
use uuid::Uuid;

use crate::core::assignment::{AssignmentHistoryEntry, PositionChange};
use crate::error::{Result, TinaError};

/// Linha do histórico de movimentações.
#[derive(sqlx::FromRow, Debug)]
struct AssignmentHistoryRecord {
    id: String,
    employee_id: String,
    previous_position_id: String,
    new_position_id: String,
    previous_department_id: String,
    new_department_id: String,
    reason: Option<String>,
    author_id: Option<String>,
    created_at: String,
}

impl From<AssignmentHistoryRecord> for AssignmentHistoryEntry {
    fn from(row: AssignmentHistoryRecord) -> Self {
        Self {
            id: row.id,
            employee_id: row.employee_id,
            previous_position_id: row.previous_position_id,
            new_position_id: row.new_position_id,
            previous_department_id: row.previous_department_id,
            new_department_id: row.new_department_id,
            reason: row.reason,
            author_id: row.author_id,
            created_at: row.created_at,
        }
    }
}

/// Operações SQL de movimentação.
pub struct AssignmentRepository;

impl AssignmentRepository {
    /// Lê cargo e departamento atuais de um funcionário dentro da empresa.
    pub(crate) async fn current_assignment(
        conn: &mut SqliteConnection,
        employee_id: &str,
        company_id: &str,
    ) -> Result<Option<(String, String)>> {
        let row: Option<(String, String)> = sqlx::query_as(
            r#"
            SELECT e.position_id, p.department_id
            FROM employees e
            JOIN positions p ON e.position_id = p.id
            JOIN departments d ON p.department_id = d.id
            WHERE e.id = ? AND d.company_id = ?
            "#,
        )
        .bind(employee_id)
        .bind(company_id)
        .fetch_optional(&mut *conn)
        .await?;

        Ok(row)
    }

    /// Valida o novo cargo e retorna seu departamento.
    pub(crate) async fn active_position_department(
        conn: &mut SqliteConnection,
        position_id: &str,
        company_id: &str,
    ) -> Result<Option<String>> {
        let department_id: Option<String> = sqlx::query_scalar(
            r#"
            SELECT p.department_id
            FROM positions p
            JOIN departments d ON p.department_id = d.id
            WHERE p.id = ? AND p.status = 'active' AND d.status = 'active' AND d.company_id = ?
            "#,
        )
        .bind(position_id)
        .bind(company_id)
        .fetch_optional(&mut *conn)
        .await?;

        Ok(department_id)
    }

    /// Atualiza o cargo atual. Retorna `false` se o funcionário não existir.
    pub(crate) async fn update_position(
        conn: &mut SqliteConnection,
        employee_id: &str,
        new_position_id: &str,
    ) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE employees SET position_id = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(new_position_id)
        .bind(employee_id)
        .execute(&mut *conn)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Insere um evento imutável no histórico de movimentações.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn insert_history(
        conn: &mut SqliteConnection,
        employee_id: &str,
        previous_position_id: &str,
        new_position_id: &str,
        previous_department_id: &str,
        new_department_id: &str,
        reason: Option<&str>,
        author_id: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO employee_assignment_history
                (id, employee_id, previous_position_id, new_position_id,
                 previous_department_id, new_department_id,
                 reason, author_id, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(employee_id)
        .bind(previous_position_id)
        .bind(new_position_id)
        .bind(previous_department_id)
        .bind(new_department_id)
        .bind(reason)
        .bind(author_id)
        .execute(&mut *conn)
        .await?;

        Ok(())
    }
}

/// Serviço de movimentação de cargo/departamento.
pub struct AssignmentService<'a> {
    pool: &'a SqlitePool,
}

impl<'a> AssignmentService<'a> {
    /// Cria o serviço sobre um pool SQLite.
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Altera o cargo (e, por consequência, o departamento) de um funcionário.
    ///
    /// `company_id` delimita o escopo: o funcionário e o novo cargo precisam
    /// pertencer à empresa. `reason` e `author_id` são opcionais; sem
    /// identidade autenticada o chamador passa `None` para `author_id`, e o
    /// histórico registra a ausência de autor.
    ///
    /// Se o funcionário já ocupa o cargo solicitado, nada é gravado e
    /// [`PositionChange::recorded`] retorna `false`.
    pub async fn change_position(
        &self,
        employee_id: &str,
        company_id: &str,
        new_position_id: &str,
        reason: Option<&str>,
        author_id: Option<&str>,
    ) -> Result<PositionChange> {
        let mut conn = self.pool.acquire().await?;
        sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;

        let outcome = self
            .apply_change(
                &mut conn,
                employee_id,
                company_id,
                new_position_id,
                reason,
                author_id,
            )
            .await;

        match outcome {
            Ok(change) => {
                sqlx::query("COMMIT").execute(&mut *conn).await?;
                Ok(change)
            }
            Err(error) => {
                let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
                Err(error)
            }
        }
    }

    async fn apply_change(
        &self,
        conn: &mut SqliteConnection,
        employee_id: &str,
        company_id: &str,
        new_position_id: &str,
        reason: Option<&str>,
        author_id: Option<&str>,
    ) -> Result<PositionChange> {
        let (previous_position_id, previous_department_id) =
            AssignmentRepository::current_assignment(conn, employee_id, company_id)
                .await?
                .ok_or_else(|| {
                    TinaError::Other("Employee not found or doesn't belong to company".to_string())
                })?;

        let new_department_id =
            AssignmentRepository::active_position_department(conn, new_position_id, company_id)
                .await?
                .ok_or_else(|| {
                    TinaError::Other(
                        "Position not found, inactive, or doesn't belong to the same company"
                            .to_string(),
                    )
                })?;

        if previous_position_id == new_position_id {
            return Ok(PositionChange {
                employee_id: employee_id.to_string(),
                previous_position_id,
                new_position_id: new_position_id.to_string(),
                previous_department_id,
                new_department_id,
                recorded: false,
            });
        }

        let updated =
            AssignmentRepository::update_position(conn, employee_id, new_position_id).await?;

        if !updated {
            return Err(TinaError::Other("Employee not found".to_string()));
        }

        AssignmentRepository::insert_history(
            conn,
            employee_id,
            &previous_position_id,
            new_position_id,
            &previous_department_id,
            &new_department_id,
            reason,
            author_id,
        )
        .await?;

        Ok(PositionChange {
            employee_id: employee_id.to_string(),
            previous_position_id,
            new_position_id: new_position_id.to_string(),
            previous_department_id,
            new_department_id,
            recorded: true,
        })
    }

    /// Recupera o histórico de movimentações de um funcionário, em ordem
    /// cronológica, dentro do escopo da empresa.
    pub async fn history(
        &self,
        employee_id: &str,
        company_id: &str,
    ) -> Result<Vec<AssignmentHistoryEntry>> {
        let rows: Vec<AssignmentHistoryRecord> = sqlx::query_as(
            r#"
            SELECT h.id, h.employee_id, h.previous_position_id, h.new_position_id,
                   h.previous_department_id, h.new_department_id,
                   h.reason, h.author_id, h.created_at
            FROM employee_assignment_history h
            JOIN employees e ON e.id = h.employee_id
            JOIN positions p ON e.position_id = p.id
            JOIN departments d ON p.department_id = d.id
            WHERE h.employee_id = ? AND d.company_id = ?
            ORDER BY h.created_at, h.rowid
            "#,
        )
        .bind(employee_id)
        .bind(company_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Altera o cargo com autoria transacional.
    ///
    /// Similar a [`Self::change_position`] mas insere/remover o contexto de
    /// auditoria antes/depois da operação, permitindo que os triggers capturem
    /// o actor correto. O `owner_uuid` deve ser o UUID do proprietário da
    /// instalação.
    ///
    /// Usa uma única transação SQLx para garantir atomicidade entre:
    /// inserção do contexto de autoria, validação de escopo, alteração do
    /// cargo, registro do histórico, remoção do contexto e confirmação.
    pub async fn change_position_with_owner(
        &self,
        employee_id: &str,
        company_id: &str,
        installation_id: &str,
        owner_uuid: &str,
        new_position_id: &str,
        reason: Option<&str>,
    ) -> Result<PositionChange> {
        let mut tx = self.pool.begin().await?;

        // Passo 1: Inserir contexto de autoria - falha se owner_uuid inválido
        let inserted = sqlx::query(
            "INSERT INTO organizational_audit_actor_context (installation_id, actor_id) \
             SELECT id, owner_uuid FROM installations WHERE id = ? AND owner_uuid = ?",
        )
        .bind(installation_id)
        .bind(owner_uuid)
        .execute(&mut *tx)
        .await?;

        if inserted.rows_affected() == 0 {
            let _ = tx.rollback().await;
            return Err(TinaError::Unauthorized(
                "identidade de auditoria inválida".into(),
            ));
        }

        // Passo 2: Executar a mudança de cargo dentro da transação
        let outcome = self
            .apply_change_in_transaction(
                &mut tx,
                employee_id,
                company_id,
                new_position_id,
                reason,
                Some(owner_uuid),
            )
            .await;

        match outcome {
            Ok(change) => {
                // Passo 3: Remover contexto antes do COMMIT
                sqlx::query(
                    "DELETE FROM organizational_audit_actor_context WHERE installation_id = ?",
                )
                .bind(installation_id)
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
                Ok(change)
            }
            Err(error) => {
                // Passo 4: Remover contexto antes do ROLLBACK
                let _ = sqlx::query(
                    "DELETE FROM organizational_audit_actor_context WHERE installation_id = ?",
                )
                .bind(installation_id)
                .execute(&mut *tx)
                .await;
                let _ = tx.rollback().await;
                Err(error)
            }
        }
    }

    /// Versão de apply_change que usa conexão transacional existente
    /// sem gerenciar sua própria transação.
    async fn apply_change_in_transaction(
        &self,
        conn: &mut SqliteConnection,
        employee_id: &str,
        company_id: &str,
        new_position_id: &str,
        reason: Option<&str>,
        author_id: Option<&str>,
    ) -> Result<PositionChange> {
        let (previous_position_id, previous_department_id) =
            AssignmentRepository::current_assignment(conn, employee_id, company_id)
                .await?
                .ok_or_else(|| {
                    TinaError::Other("Employee not found or doesn't belong to company".to_string())
                })?;

        let new_department_id =
            AssignmentRepository::active_position_department(conn, new_position_id, company_id)
                .await?
                .ok_or_else(|| {
                    TinaError::Other(
                        "Position not found, inactive, or doesn't belong to the same company"
                            .to_string(),
                    )
                })?;

        if previous_position_id == new_position_id {
            return Ok(PositionChange {
                employee_id: employee_id.to_string(),
                previous_position_id,
                new_position_id: new_position_id.to_string(),
                previous_department_id,
                new_department_id,
                recorded: false,
            });
        }

        let updated =
            AssignmentRepository::update_position(conn, employee_id, new_position_id).await?;

        if !updated {
            return Err(TinaError::Other("Employee not found".to_string()));
        }

        AssignmentRepository::insert_history(
            conn,
            employee_id,
            &previous_position_id,
            new_position_id,
            &previous_department_id,
            &new_department_id,
            reason,
            author_id,
        )
        .await?;

        Ok(PositionChange {
            employee_id: employee_id.to_string(),
            previous_position_id,
            new_position_id: new_position_id.to_string(),
            previous_department_id,
            new_department_id,
            recorded: true,
        })
    }
}
