//! Repositório de departamentos

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::core::department::{Department, DepartmentStatus};
use crate::error::Result;

/// Representa uma linha de departamento do banco
#[derive(sqlx::FromRow, Debug)]
struct DepartmentRow {
    id: String,
    company_id: String,
    code: String,
    name: String,
    description: Option<String>,
    status: String,
    created_at: String,
}

impl From<DepartmentRow> for Department {
    fn from(row: DepartmentRow) -> Self {
        let status = match row.status.as_str() {
            "active" => DepartmentStatus::Active,
            "inactive" => DepartmentStatus::Inactive,
            "suspended" => DepartmentStatus::Suspended,
            _ => DepartmentStatus::Inactive,
        };
        
        Self {
            id: row.id,
            company_id: row.company_id,
            code: row.code,
            name: row.name,
            description: row.description,
            status,
            created_at: row.created_at,
        }
    }
}

/// Repositório para operações de departamento
pub struct DepartmentRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> DepartmentRepository<'a> {
    /// Cria um novo repositório de departamentos
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Cria um novo departamento
    /// 
    /// # Argumentos
    /// * `company_id` - ID da empresa (validado pelo caller)
    /// * `code` - Código único do departamento
    /// * `name` - Nome do departamento
    /// * `description` - Descrição opcional
    pub async fn create(
        &self,
        company_id: &str,
        code: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<Department> {
        let id = Uuid::new_v4().to_string();
        
        sqlx::query(
            r#"
            INSERT INTO departments (id, company_id, code, name, description, status, created_at)
            VALUES (?, ?, ?, ?, ?, 'active', datetime('now'))
            "#
        )
        .bind(&id)
        .bind(company_id)
        .bind(code)
        .bind(name)
        .bind(description)
        .execute(self.pool)
        .await?;

        self.find_by_id(&id).await
    }

    /// Busca departamento pelo ID
    pub async fn find_by_id(&self, id: &str) -> Result<Department> {
        let row: DepartmentRow = sqlx::query_as(
            "SELECT id, company_id, code, name, description, status, created_at FROM departments WHERE id = ?"
        )
        .bind(id)
        .fetch_one(self.pool)
        .await?;
        
        Ok(row.into())
    }

    /// Busca departamento pelo ID (opcional)
    pub async fn find_by_id_optional(&self, id: &str) -> Result<Option<Department>> {
        let row: Option<DepartmentRow> = sqlx::query_as(
            "SELECT id, company_id, code, name, description, status, created_at FROM departments WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;
        
        Ok(row.map(|r| r.into()))
    }

    /// Lista departamentos de uma empresa
    pub async fn list_by_company(&self, company_id: &str) -> Result<Vec<Department>> {
        let rows: Vec<DepartmentRow> = sqlx::query_as(
            "SELECT id, company_id, code, name, description, status, created_at FROM departments WHERE company_id = ? AND status = 'active' ORDER BY code, name"
        )
        .bind(company_id)
        .fetch_all(self.pool)
        .await?;
        
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Verifica se departamento pertence à empresa
    pub async fn belongs_to_company(&self, department_id: &str, company_id: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM departments WHERE id = ? AND company_id = ? AND status = 'active'"
        )
        .bind(department_id)
        .bind(company_id)
        .fetch_one(self.pool)
        .await?;
        
        Ok(count > 0)
    }

    /// Verifica se departamento existe
    pub async fn exists(&self, department_id: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM departments WHERE id = ?"
        )
        .bind(department_id)
        .fetch_one(self.pool)
        .await?;
        
        Ok(count > 0)
    }

    /// Atualiza o nome do departamento
    pub async fn update_name(&self, department_id: &str, new_name: &str) -> Result<()> {
        sqlx::query(
            "UPDATE departments SET name = ? WHERE id = ?"
        )
        .bind(new_name)
        .bind(department_id)
        .execute(self.pool)
        .await?;
        
        Ok(())
    }

    /// Atualiza a descrição do departamento
    pub async fn update_description(&self, department_id: &str, new_description: Option<&str>) -> Result<()> {
        sqlx::query(
            "UPDATE departments SET description = ? WHERE id = ?"
        )
        .bind(new_description)
        .bind(department_id)
        .execute(self.pool)
        .await?;
        
        Ok(())
    }

    /// Arquiva um departamento
    pub async fn archive(&self, department_id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE departments SET status = 'inactive' WHERE id = ?"
        )
        .bind(department_id)
        .execute(self.pool)
        .await?;
        
        Ok(())
    }

    /// Verifica se departamento pode ser arquivado (sem cargos ativos)
    pub async fn can_archive(&self, department_id: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM positions WHERE department_id = ? AND status = 'active'"
        )
        .bind(department_id)
        .fetch_one(self.pool)
        .await?;
        
        Ok(count == 0)
    }
}