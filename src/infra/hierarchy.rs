//! Hierarquia dinâmica entre funcionários: relação atual e histórico.
//!
//! # Fonte de verdade
//!
//! A relação **atual** vive em `employees.supervisor_id` (autorreferência).
//! O histórico é um log append-only em `employee_supervisor_history`; não é
//! uma segunda fonte de verdade do presente.
//!
//! # Consistência transacional
//!
//! Alterações usam `BEGIN IMMEDIATE`, que adquire o lock de escrita do SQLite
//! **antes** de ler a cadeia de superiores. Com isso, duas alterações
//! simultâneas são serializadas: a segunda só enxerga o estado já confirmado
//! pela primeira e a detecção de ciclos é feita sobre esse estado. A atualização
//! de `supervisor_id` e a inserção no histórico ocorrem na mesma transação:
//! se uma falhar, a outra é desfeita.
//!
//! # Ciclos
//!
//! A verificação percorre **toda** a cadeia de ancestrais do superior
//! proposto. Um limite defensivo de profundidade (10.000 níveis) existe apenas
//! para impedir laço infinito em dados legados já corrompidos; não é um limite
//! funcional da organização.

use sqlx::sqlite::SqlitePool;
use sqlx::SqliteConnection;
use uuid::Uuid;

use crate::core::employee::{Employee, EmployeeStatus};
use crate::core::hierarchy::{
    HierarchyNode, HierarchyOperation, SupervisorChange, SupervisorHistoryEntry,
};
use crate::error::{Result, TinaError};

/// Limite defensivo de profundidade para consultas recursivas.
///
/// Não restringe a hierarquia funcional: apenas evita laço infinito caso um
/// banco legado contenha ciclo (situação que as operações validadas não criam).
const RECURSION_GUARD: i64 = 10_000;

/// Erros tipados das regras de hierarquia.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HierarchyError {
    /// Funcionário inexistente, de outra empresa ou inativo.
    #[error("funcionário não encontrado, inativo ou fora do escopo da empresa")]
    EmployeeUnavailable,
    /// Funcionário não pode ser seu próprio superior.
    #[error("um funcionário não pode ser seu próprio superior")]
    SelfSupervision,
    /// Superior inexistente, inativo ou de outra empresa/instalação.
    #[error("superior não encontrado, inativo ou fora do escopo da empresa")]
    SupervisorUnavailable,
    /// A alteração criaria um ciclo hierárquico.
    #[error("a alteração criaria um ciclo hierárquico")]
    CycleDetected,
}

impl From<HierarchyError> for TinaError {
    fn from(error: HierarchyError) -> Self {
        TinaError::Hierarchy(error.to_string())
    }
}

/// Linha completa de funcionário usada nas consultas de hierarquia.
#[derive(sqlx::FromRow, Debug)]
struct EmployeeRecord {
    id: String,
    position_id: String,
    name: String,
    responsibilities: Option<String>,
    supervisor_id: Option<String>,
    status: String,
    created_at: String,
    updated_at: String,
    deactivated_at: Option<String>,
}

impl From<EmployeeRecord> for Employee {
    fn from(row: EmployeeRecord) -> Self {
        let status = match row.status.as_str() {
            "active" => EmployeeStatus::Active,
            _ => EmployeeStatus::Inactive,
        };

        Self {
            id: row.id,
            name: row.name,
            position_id: row.position_id,
            responsibilities: row.responsibilities,
            supervisor_id: row.supervisor_id,
            status,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deactivated_at: row.deactivated_at,
        }
    }
}

/// Linha da árvore com profundidade calculada.
#[derive(sqlx::FromRow, Debug)]
struct HierarchyRecord {
    id: String,
    position_id: String,
    name: String,
    responsibilities: Option<String>,
    supervisor_id: Option<String>,
    status: String,
    created_at: String,
    updated_at: String,
    deactivated_at: Option<String>,
    depth: Option<i64>,
}

impl From<HierarchyRecord> for HierarchyNode {
    fn from(row: HierarchyRecord) -> Self {
        let depth = row.depth;
        let employee = Employee::from(EmployeeRecord {
            id: row.id,
            position_id: row.position_id,
            name: row.name,
            responsibilities: row.responsibilities,
            supervisor_id: row.supervisor_id,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deactivated_at: row.deactivated_at,
        });

        Self { employee, depth }
    }
}

/// Linha do histórico hierárquico.
#[derive(sqlx::FromRow, Debug)]
struct HistoryRecord {
    id: String,
    employee_id: String,
    previous_supervisor_id: Option<String>,
    new_supervisor_id: Option<String>,
    operation: String,
    reason: Option<String>,
    author_id: Option<String>,
    created_at: String,
}

impl HistoryRecord {
    fn into_entry(self) -> SupervisorHistoryEntry {
        SupervisorHistoryEntry {
            id: self.id,
            employee_id: self.employee_id,
            previous_supervisor_id: self.previous_supervisor_id,
            new_supervisor_id: self.new_supervisor_id,
            operation: HierarchyOperation::from_db(&self.operation)
                .unwrap_or(HierarchyOperation::Removed),
            reason: self.reason,
            author_id: self.author_id,
            created_at: self.created_at,
        }
    }
}

/// Operações SQL da hierarquia.
///
/// Recebem uma conexão explícita para participar de transações controladas
/// pelo [`HierarchyService`].
pub struct HierarchyRepository;

impl HierarchyRepository {
    /// Busca funcionário ativo, validando a empresa via cargo → departamento.
    pub(crate) async fn find_active_in_company(
        conn: &mut SqliteConnection,
        employee_id: &str,
        company_id: &str,
    ) -> Result<Option<Employee>> {
        let row: Option<EmployeeRecord> = sqlx::query_as(
            r#"
            SELECT e.id, e.position_id, e.name, e.responsibilities, e.supervisor_id,
                   e.status, e.created_at, e.updated_at, e.deactivated_at
            FROM employees e
            JOIN positions p ON e.position_id = p.id
            JOIN departments d ON p.department_id = d.id
            WHERE e.id = ? AND d.company_id = ? AND e.status = 'active'
            "#,
        )
        .bind(employee_id)
        .bind(company_id)
        .fetch_optional(&mut *conn)
        .await?;

        Ok(row.map(Into::into))
    }

    /// Busca funcionário (qualquer status) dentro da empresa.
    pub(crate) async fn find_in_company(
        conn: &mut SqliteConnection,
        employee_id: &str,
        company_id: &str,
    ) -> Result<Option<Employee>> {
        let row: Option<EmployeeRecord> = sqlx::query_as(
            r#"
            SELECT e.id, e.position_id, e.name, e.responsibilities, e.supervisor_id,
                   e.status, e.created_at, e.updated_at, e.deactivated_at
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

        Ok(row.map(Into::into))
    }

    /// Verifica se `target_id` aparece entre os ancestrais de `start_id`.
    ///
    /// Percorre toda a cadeia de superiores; `true` significa que a relação
    /// proposta fecharia um ciclo.
    pub(crate) async fn is_in_supervisor_chain(
        conn: &mut SqliteConnection,
        start_id: &str,
        target_id: &str,
    ) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            r#"
            WITH RECURSIVE ancestors(id, supervisor_id, depth) AS (
                SELECT id, supervisor_id, 0
                FROM employees
                WHERE id = ?1
                UNION ALL
                SELECT e.id, e.supervisor_id, a.depth + 1
                FROM employees e
                JOIN ancestors a ON e.id = a.supervisor_id
                WHERE a.depth < ?3
            )
            SELECT COUNT(*) FROM ancestors WHERE id = ?2
            "#,
        )
        .bind(start_id)
        .bind(target_id)
        .bind(RECURSION_GUARD)
        .fetch_one(&mut *conn)
        .await?;

        Ok(count > 0)
    }

    /// Atualiza a relação atual. Retorna `false` se o funcionário não existir.
    pub(crate) async fn update_supervisor(
        conn: &mut SqliteConnection,
        employee_id: &str,
        supervisor_id: Option<&str>,
    ) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE employees SET supervisor_id = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(supervisor_id)
        .bind(employee_id)
        .execute(&mut *conn)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Insere um evento imutável no histórico.
    pub(crate) async fn insert_history(
        conn: &mut SqliteConnection,
        employee_id: &str,
        previous_supervisor_id: Option<&str>,
        new_supervisor_id: Option<&str>,
        operation: HierarchyOperation,
        reason: Option<&str>,
        author_id: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO employee_supervisor_history
                (id, employee_id, previous_supervisor_id, new_supervisor_id,
                 operation, reason, author_id, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(employee_id)
        .bind(previous_supervisor_id)
        .bind(new_supervisor_id)
        .bind(operation.as_str())
        .bind(reason)
        .bind(author_id)
        .execute(&mut *conn)
        .await?;

        Ok(())
    }
}

/// Serviço de hierarquia: valida regras de negócio e coordena relação + histórico.
pub struct HierarchyService<'a> {
    pool: &'a SqlitePool,
}

impl<'a> HierarchyService<'a> {
    /// Cria o serviço sobre um pool SQLite.
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Define (ou remove) o superior direto de um funcionário.
    ///
    /// `company_id` delimita o escopo: funcionário e superior precisam
    /// pertencer à empresa. `reason` e `author_id` são opcionais; sem
    /// identidade autenticada o chamador passa `None` para `author_id`, e o
    /// histórico registra a ausência de autor.
    ///
    /// Se a relação solicitada já for a atual, nada é gravado e
    /// [`SupervisorChange::operation`] retorna `None`.
    pub async fn set_supervisor(
        &self,
        employee_id: &str,
        company_id: &str,
        new_supervisor_id: Option<&str>,
        reason: Option<&str>,
        author_id: Option<&str>,
    ) -> Result<SupervisorChange> {
        let mut conn = self.pool.acquire().await?;
        sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;

        let outcome = self
            .apply_supervisor_change(
                &mut conn,
                employee_id,
                company_id,
                new_supervisor_id,
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

    async fn apply_supervisor_change(
        &self,
        conn: &mut SqliteConnection,
        employee_id: &str,
        company_id: &str,
        new_supervisor_id: Option<&str>,
        reason: Option<&str>,
        author_id: Option<&str>,
    ) -> Result<SupervisorChange> {
        let employee = HierarchyRepository::find_active_in_company(conn, employee_id, company_id)
            .await?
            .ok_or(HierarchyError::EmployeeUnavailable)?;

        let previous_supervisor_id = employee.supervisor_id.clone();

        if previous_supervisor_id.as_deref() == new_supervisor_id {
            return Ok(SupervisorChange {
                employee_id: employee.id,
                previous_supervisor_id: previous_supervisor_id.clone(),
                new_supervisor_id: previous_supervisor_id,
                operation: None,
            });
        }

        if let Some(supervisor_id) = new_supervisor_id {
            if supervisor_id == employee_id {
                return Err(HierarchyError::SelfSupervision.into());
            }

            let supervisor = HierarchyRepository::find_in_company(conn, supervisor_id, company_id)
                .await?
                .ok_or(HierarchyError::SupervisorUnavailable)?;

            if supervisor.status != EmployeeStatus::Active {
                return Err(HierarchyError::SupervisorUnavailable.into());
            }

            // O funcionário não pode passar a responder a alguém que já
            // depende dele, direta ou indiretamente.
            if HierarchyRepository::is_in_supervisor_chain(conn, supervisor_id, employee_id).await?
            {
                return Err(HierarchyError::CycleDetected.into());
            }
        }

        let operation = match (&previous_supervisor_id, new_supervisor_id) {
            (None, Some(_)) => HierarchyOperation::Assigned,
            (Some(_), None) => HierarchyOperation::Removed,
            (Some(_), Some(_)) => HierarchyOperation::Changed,
            // Relação inalterada já retornou acima.
            (None, None) => unreachable!("unchanged relation is returned before this point"),
        };

        let updated =
            HierarchyRepository::update_supervisor(conn, employee_id, new_supervisor_id).await?;

        if !updated {
            return Err(HierarchyError::EmployeeUnavailable.into());
        }

        HierarchyRepository::insert_history(
            conn,
            employee_id,
            previous_supervisor_id.as_deref(),
            new_supervisor_id,
            operation,
            reason,
            author_id,
        )
        .await?;

        Ok(SupervisorChange {
            employee_id: employee.id,
            previous_supervisor_id,
            new_supervisor_id: new_supervisor_id.map(str::to_string),
            operation: Some(operation),
        })
    }

    /// Retorna a cadeia de superiores de um funcionário, do direto à raiz.
    pub async fn supervisor_chain(
        &self,
        employee_id: &str,
        company_id: &str,
    ) -> Result<Vec<Employee>> {
        let rows: Vec<EmployeeRecord> = sqlx::query_as(
            r#"
            WITH RECURSIVE chain(id, supervisor_id, depth) AS (
                SELECT e.id, e.supervisor_id, 0
                FROM employees e
                JOIN positions p ON e.position_id = p.id
                JOIN departments d ON p.department_id = d.id
                WHERE e.id = ?1 AND d.company_id = ?2
                UNION ALL
                SELECT e.id, e.supervisor_id, c.depth + 1
                FROM employees e
                JOIN chain c ON e.id = c.supervisor_id
                WHERE c.depth < ?3
            )
            SELECT e.id, e.position_id, e.name, e.responsibilities, e.supervisor_id,
                   e.status, e.created_at, e.updated_at, e.deactivated_at
            FROM chain c
            JOIN employees e ON e.id = c.id
            WHERE c.depth > 0
            ORDER BY c.depth
            "#,
        )
        .bind(employee_id)
        .bind(company_id)
        .bind(RECURSION_GUARD)
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Lista os subordinados diretos de um superior dentro da empresa.
    pub async fn direct_subordinates(
        &self,
        supervisor_id: &str,
        company_id: &str,
        include_inactive: bool,
    ) -> Result<Vec<Employee>> {
        let status_filter = if include_inactive {
            ""
        } else {
            "AND e.status = 'active'"
        };

        let query = format!(
            r#"
            SELECT e.id, e.position_id, e.name, e.responsibilities, e.supervisor_id,
                   e.status, e.created_at, e.updated_at, e.deactivated_at
            FROM employees e
            JOIN positions p ON e.position_id = p.id
            JOIN departments d ON p.department_id = d.id
            WHERE e.supervisor_id = ? AND d.company_id = ? {status_filter}
            ORDER BY e.name
            "#
        );

        let rows: Vec<EmployeeRecord> = sqlx::query_as(&query)
            .bind(supervisor_id)
            .bind(company_id)
            .fetch_all(self.pool)
            .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Lista toda a descendência de um superior dentro da empresa.
    ///
    /// A profundidade começa em 1 (subordinados diretos). Não há limite
    /// funcional de níveis; o corte em [`RECURSION_GUARD`] é apenas defensivo.
    pub async fn descendants(
        &self,
        supervisor_id: &str,
        company_id: &str,
        include_inactive: bool,
    ) -> Result<Vec<HierarchyNode>> {
        let status_filter = if include_inactive {
            ""
        } else {
            "AND e.status = 'active'"
        };

        let query = format!(
            r#"
            WITH RECURSIVE subtree(id, depth) AS (
                SELECT id, 1 FROM employees WHERE supervisor_id = ?1
                UNION ALL
                SELECT e.id, s.depth + 1
                FROM employees e
                JOIN subtree s ON e.supervisor_id = s.id
                WHERE s.depth < ?3
            )
            SELECT e.id, e.position_id, e.name, e.responsibilities, e.supervisor_id,
                   e.status, e.created_at, e.updated_at, e.deactivated_at, s.depth
            FROM subtree s
            JOIN employees e ON e.id = s.id
            JOIN positions p ON e.position_id = p.id
            JOIN departments d ON p.department_id = d.id
            WHERE d.company_id = ?2 {status_filter}
            ORDER BY s.depth, e.name
            "#
        );

        let rows: Vec<HierarchyRecord> = sqlx::query_as(&query)
            .bind(supervisor_id)
            .bind(company_id)
            .bind(RECURSION_GUARD)
            .fetch_all(self.pool)
            .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Retorna a árvore da empresa com profundidade a partir das raízes.
    ///
    /// Funcionários inalcançáveis a partir de uma raiz (dados legados com
    /// ciclo) aparecem com `depth = None` em vez de serem omitidos.
    pub async fn company_tree(
        &self,
        company_id: &str,
        include_inactive: bool,
    ) -> Result<Vec<HierarchyNode>> {
        let status_filter = if include_inactive {
            ""
        } else {
            "AND e.status = 'active'"
        };

        let query = format!(
            r#"
            WITH RECURSIVE tree(id, depth) AS (
                SELECT e.id, 0
                FROM employees e
                JOIN positions p ON e.position_id = p.id
                JOIN departments d ON p.department_id = d.id
                WHERE d.company_id = ?1 AND e.supervisor_id IS NULL
                UNION ALL
                SELECT e.id, t.depth + 1
                FROM employees e
                JOIN tree t ON e.supervisor_id = t.id
                WHERE t.depth < ?2
            )
            SELECT e.id, e.position_id, e.name, e.responsibilities, e.supervisor_id,
                   e.status, e.created_at, e.updated_at, e.deactivated_at, t.depth
            FROM employees e
            JOIN positions p ON e.position_id = p.id
            JOIN departments d ON p.department_id = d.id
            LEFT JOIN tree t ON t.id = e.id
            WHERE d.company_id = ?1 {status_filter}
            ORDER BY (t.depth IS NULL), t.depth, e.name
            "#
        );

        let rows: Vec<HierarchyRecord> = sqlx::query_as(&query)
            .bind(company_id)
            .bind(RECURSION_GUARD)
            .fetch_all(self.pool)
            .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Recupera o histórico hierárquico de um funcionário, em ordem cronológica.
    pub async fn history(
        &self,
        employee_id: &str,
        company_id: &str,
    ) -> Result<Vec<SupervisorHistoryEntry>> {
        let rows: Vec<HistoryRecord> = sqlx::query_as(
            r#"
            SELECT h.id, h.employee_id, h.previous_supervisor_id, h.new_supervisor_id,
                   h.operation, h.reason, h.author_id, h.created_at
            FROM employee_supervisor_history h
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

        Ok(rows.into_iter().map(HistoryRecord::into_entry).collect())
    }
}
