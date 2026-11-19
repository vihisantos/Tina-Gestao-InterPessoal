//! Repositório de funcionários (employees)

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::core::employee::{Employee, EmployeeStatus};
use crate::error::{Result, TinaError};

/// Representa uma linha de funcionário do banco
#[derive(sqlx::FromRow, Debug)]
struct EmployeeRow {
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

impl From<EmployeeRow> for Employee {
    fn from(row: EmployeeRow) -> Self {
        let status = match row.status.as_str() {
            "active" => EmployeeStatus::Active,
            "inactive" => EmployeeStatus::Inactive,
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

/// Repositório para operações de funcionário
pub struct EmployeeRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> EmployeeRepository<'a> {
    /// Cria um novo repositório de funcionários
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Cria um novo funcionário associado a um cargo existente.
    ///
    /// # Argumentos
    /// * `position_id` - ID do cargo (deve existir e estar ativo)
    /// * `name` - Nome do funcionário
    /// * `responsibilities` - Responsabilidades opcionais
    /// * `supervisor_id` - ID do superior direto (opcional, deve ser funcionário da mesma empresa)
    ///
    /// # Retorno
    /// O funcionário criado com ID gerado automaticamente
    pub async fn create(
        &self,
        position_id: &str,
        name: &str,
        responsibilities: Option<&str>,
        supervisor_id: Option<&str>,
    ) -> Result<Employee> {
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO employees (id, position_id, name, responsibilities, supervisor_id, model_config, status, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, 'nvidia/nemotron-3.5-bolt-e4b', 'active', datetime('now'), datetime('now'))
            "#,
        )
        .bind(&id)
        .bind(position_id)
        .bind(name)
        .bind(responsibilities)
        .bind(supervisor_id)
        .execute(self.pool)
        .await?;

        self.find_by_id(&id).await
    }

    /// Busca funcionário pelo ID
    pub async fn find_by_id(&self, id: &str) -> Result<Employee> {
        let row: EmployeeRow = sqlx::query_as(
            "SELECT id, position_id, name, responsibilities, supervisor_id, status, created_at, updated_at, deactivated_at FROM employees WHERE id = ?",
        )
        .bind(id)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into())
    }

    /// Busca funcionário pelo ID (opcional)
    pub async fn find_by_id_optional(&self, id: &str) -> Result<Option<Employee>> {
        let row: Option<EmployeeRow> = sqlx::query_as(
            "SELECT id, position_id, name, responsibilities, supervisor_id, status, created_at, updated_at, deactivated_at FROM employees WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }

    /// Lista todos os funcionários de uma empresa (via cadeia posição → departamento → empresa)
    ///
    /// # Argumentos
    /// * `company_id` - ID da empresa
    /// * `include_inactive` - se deve incluir funcionários inativos
    pub async fn list_by_company(
        &self,
        company_id: &str,
        include_inactive: bool,
    ) -> Result<Vec<Employee>> {
        let query = if include_inactive {
            r#"
            SELECT e.id, e.position_id, e.name, e.responsibilities, e.supervisor_id, e.status, e.created_at, e.updated_at, e.deactivated_at
            FROM employees e
            JOIN positions p ON e.position_id = p.id
            JOIN departments d ON p.department_id = d.id
            WHERE d.company_id = ?
            ORDER BY e.name
            "#
        } else {
            r#"
            SELECT e.id, e.position_id, e.name, e.responsibilities, e.supervisor_id, e.status, e.created_at, e.updated_at, e.deactivated_at
            FROM employees e
            JOIN positions p ON e.position_id = p.id
            JOIN departments d ON p.department_id = d.id
            WHERE d.company_id = ? AND e.status = 'active' AND p.status = 'active' AND d.status = 'active'
            ORDER BY e.name
            "#
        };

        let rows: Vec<EmployeeRow> = sqlx::query_as(query)
            .bind(company_id)
            .fetch_all(self.pool)
            .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Lista todos os funcionários de um departamento (via cargo → departamento)
    ///
    /// # Argumentos
    /// * `department_id` - ID do departamento
    /// * `include_inactive` - se deve incluir funcionários inativos
    pub async fn list_by_department(
        &self,
        department_id: &str,
        include_inactive: bool,
    ) -> Result<Vec<Employee>> {
        let query = if include_inactive {
            r#"
            SELECT e.id, e.position_id, e.name, e.responsibilities, e.supervisor_id, e.status, e.created_at, e.updated_at, e.deactivated_at
            FROM employees e
            JOIN positions p ON e.position_id = p.id
            WHERE p.department_id = ?
            ORDER BY e.name
            "#
        } else {
            r#"
            SELECT e.id, e.position_id, e.name, e.responsibilities, e.supervisor_id, e.status, e.created_at, e.updated_at, e.deactivated_at
            FROM employees e
            JOIN positions p ON e.position_id = p.id
            WHERE p.department_id = ? AND e.status = 'active' AND p.status = 'active'
            ORDER BY e.name
            "#
        };

        let rows: Vec<EmployeeRow> = sqlx::query_as(query)
            .bind(department_id)
            .fetch_all(self.pool)
            .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Verifica se o funcionário pertence à empresa informada (via cadeia completa)
    pub async fn belongs_to_company(
        &self,
        employee_id: &str,
        company_id: &str,
    ) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM employees e
            JOIN positions p ON e.position_id = p.id
            JOIN departments d ON p.department_id = d.id
            WHERE e.id = ? AND d.company_id = ?
            "#,
        )
        .bind(employee_id)
        .bind(company_id)
        .fetch_one(self.pool)
        .await?;

        Ok(count > 0)
    }

    /// Verifica se o funcionário pertence à instalação informada (via cadeia completa)
    pub async fn belongs_to_installation(
        &self,
        employee_id: &str,
        installation_id: &str,
    ) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM employees e
            JOIN positions p ON e.position_id = p.id
            JOIN departments d ON p.department_id = d.id
            JOIN companies c ON d.company_id = c.id
            WHERE e.id = ? AND c.installation_id = ?
            "#,
        )
        .bind(employee_id)
        .bind(installation_id)
        .fetch_one(self.pool)
        .await?;

        Ok(count > 0)
    }

    /// Verifica se o cargo existe e está ativo
    pub async fn position_exists_and_active(&self, position_id: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM positions WHERE id = ? AND status = 'active'",
        )
        .bind(position_id)
        .fetch_one(self.pool)
        .await?;

        Ok(count > 0)
    }

    /// Verifica se o departamento existe e está ativo
    pub async fn department_exists_and_active(&self, department_id: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM departments WHERE id = ? AND status = 'active'",
        )
        .bind(department_id)
        .fetch_one(self.pool)
        .await?;

        Ok(count > 0)
    }

    /// Verifica se o funcionário existe (qualquer status)
    pub async fn exists(&self, employee_id: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM employees WHERE id = ?")
            .bind(employee_id)
            .fetch_one(self.pool)
            .await?;

        Ok(count > 0)
    }

    /// Atualiza nome e responsabilidades do funcionário (apenas se ativo e pertencer à empresa)
    pub async fn update_info(
        &self,
        employee_id: &str,
        company_id: &str,
        name: &str,
        responsibilities: Option<&str>,
    ) -> Result<()> {
        // Verificar pertencimento à empresa antes de atualizar
        if !self.belongs_to_company(employee_id, company_id).await? {
            return Err(TinaError::Other(
                "Employee not found or doesn't belong to company".to_string(),
            )
            .into());
        }

        let result = sqlx::query(
            "UPDATE employees SET name = ?, responsibilities = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(name)
        .bind(responsibilities)
        .bind(employee_id)
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(TinaError::Other(
                "Employee not found".to_string(),
            )
            .into());
        }

        Ok(())
    }

    /// Alterar departamento e cargo do funcionário, respeitando consistência organizacional.
    ///
    /// O novo cargo deve pertencer ao novo departamento, e o departamento deve pertencer à mesma empresa.
    pub async fn change_position(
        &self,
        employee_id: &str,
        company_id: &str,
        new_position_id: &str,
    ) -> Result<()> {
        // Verificar que o funcionário pertence à empresa
        if !self.belongs_to_company(employee_id, company_id).await? {
            return Err(TinaError::Other(
                "Employee not found or doesn't belong to company".to_string(),
            )
            .into());
        }

        // Verificar que o novo cargo existe, está ativo e pertence a um departamento da mesma empresa
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM positions p
            JOIN departments d ON p.department_id = d.id
            WHERE p.id = ? AND p.status = 'active' AND d.status = 'active' AND d.company_id = ?
            "#,
        )
        .bind(new_position_id)
        .bind(company_id)
        .fetch_one(self.pool)
        .await?;

        if count == 0 {
            return Err(TinaError::Other(
                "Position not found, inactive, or doesn't belong to the same company".to_string(),
            )
            .into());
        }

        let result = sqlx::query(
            "UPDATE employees SET position_id = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(new_position_id)
        .bind(employee_id)
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(TinaError::Other("Employee not found".to_string()).into());
        }

        Ok(())
    }

    /// Associar superior direto (relacionamento próprio).
    /// O superior deve pertencer à mesma empresa.
    pub async fn assign_supervisor(
        &self,
        employee_id: &str,
        company_id: &str,
        supervisor_id: Option<&str>,
    ) -> Result<()> {
        // Verificar que o funcionário pertence à empresa
        if !self.belongs_to_company(employee_id, company_id).await? {
            return Err(TinaError::Other(
                "Employee not found or doesn't belong to company".to_string(),
            )
            .into());
        }

        // Se houver supervisor, verificar que ele existe, está ativo e pertence à mesma empresa
        if let Some(sup_id) = supervisor_id {
            if sup_id == employee_id {
                return Err(TinaError::Other(
                    "Employee cannot be their own supervisor".to_string(),
                )
                .into());
            }

            let sup_count: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*)
                FROM employees e
                JOIN positions p ON e.position_id = p.id
                JOIN departments d ON p.department_id = d.id
                WHERE e.id = ? AND e.status = 'active' AND d.company_id = ?
                "#,
            )
            .bind(sup_id)
            .bind(company_id)
            .fetch_one(self.pool)
            .await?;

            if sup_count == 0 {
                return Err(TinaError::Other(
                    "Supervisor not found, inactive, or doesn't belong to the same company"
                        .to_string(),
                )
                .into());
            }
        }

        let result = sqlx::query(
            "UPDATE employees SET supervisor_id = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(supervisor_id)
        .bind(employee_id)
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(TinaError::Other("Employee not found".to_string()).into());
        }

        Ok(())
    }

    /// Desativar funcionário (não exclui fisicamente, preservando histórico)
    pub async fn deactivate(&self, employee_id: &str, company_id: &str) -> Result<()> {
        // Verificar pertencimento à empresa
        if !self.belongs_to_company(employee_id, company_id).await? {
            return Err(TinaError::Other(
                "Employee not found or doesn't belong to company".to_string(),
            )
            .into());
        }

        let result = sqlx::query(
            "UPDATE employees SET status = 'inactive', deactivated_at = datetime('now'), updated_at = datetime('now') WHERE id = ? AND status = 'active'",
        )
        .bind(employee_id)
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(TinaError::Other(
                "Employee not found, already inactive, or doesn't belong to company".to_string(),
            )
            .into());
        }

        Ok(())
    }

    /// Reativar funcionário previamente desativado
    pub async fn reactivate(&self, employee_id: &str, company_id: &str) -> Result<()> {
        // Verificar pertencimento à empresa
        if !self.belongs_to_company(employee_id, company_id).await? {
            return Err(TinaError::Other(
                "Employee not found or doesn't belong to company".to_string(),
            )
            .into());
        }

        let result = sqlx::query(
            "UPDATE employees SET status = 'active', deactivated_at = NULL, updated_at = datetime('now') WHERE id = ? AND status = 'inactive'",
        )
        .bind(employee_id)
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(TinaError::Other(
                "Employee not found, already active, or doesn't belong to company".to_string(),
            )
            .into());
        }

        Ok(())
    }

    /// Verifica se o funcionário pode ser desativado (não é o proprietário da instalação)
    /// O proprietário não é um funcionário, então esta verificação é no nível de comando.
    pub async fn count_subordinates(&self, supervisor_id: &str) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM employees WHERE supervisor_id = ?",
        )
        .bind(supervisor_id)
        .fetch_one(self.pool)
        .await?;

        Ok(count)
    }

    /// Busca funcionários que reportam diretamente a um supervisor
    pub async fn list_subordinates(&self, supervisor_id: &str) -> Result<Vec<Employee>> {
        let rows: Vec<EmployeeRow> = sqlx::query_as(
            "SELECT id, position_id, name, responsibilities, supervisor_id, status, created_at, updated_at, deactivated_at FROM employees WHERE supervisor_id = ? ORDER BY name",
        )
        .bind(supervisor_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Atualiza apenas as responsabilidades do funcionário
    pub async fn update_responsibilities(
        &self,
        employee_id: &str,
        company_id: &str,
        responsibilities: Option<&str>,
    ) -> Result<()> {
        if !self.belongs_to_company(employee_id, company_id).await? {
            return Err(TinaError::Other(
                "Employee not found or doesn't belong to company".to_string(),
            )
            .into());
        }

        let result = sqlx::query(
            "UPDATE employees SET responsibilities = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(responsibilities)
        .bind(employee_id)
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(TinaError::Other("Employee not found".to_string()).into());
        }

        Ok(())
    }
}


