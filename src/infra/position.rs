//! Repositório de cargos (positions)

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::core::position::{Position, PositionStatus};
use crate::error::Result;

/// Representa uma linha de posição do banco
#[derive(sqlx::FromRow, Debug)]
struct PositionRow {
    id: String,
    department_id: String,
    name: String,
    level: i32,
    authority_level: i32,
    description: Option<String>,
    status: String,
    created_at: String,
}

impl From<PositionRow> for Position {
    fn from(row: PositionRow) -> Self {
        let status = match row.status.as_str() {
            "active" => PositionStatus::Active,
            "inactive" => PositionStatus::Inactive,
            "suspended" => PositionStatus::Suspended,
            _ => PositionStatus::Inactive,
        };
        
        Self {
            id: row.id,
            department_id: row.department_id,
            name: row.name,
            level: row.level,
            authority_level: row.authority_level,
            description: row.description,
            status,
            created_at: row.created_at,
        }
    }
}

/// Repositório para operações de cargo
pub struct PositionRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> PositionRepository<'a> {
    /// Cria um novo repositório de cargos
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Cria um novo cargo
    /// 
    /// # Argumentos
    /// * `department_id` - ID do departamento (validado pelo caller)
    /// * `name` - Nome do cargo
    /// * `level` - Nível hierárquico
    /// * `authority_level` - Nível de autoridade
    /// * `description` - Descrição opcional
    pub async fn create(
        &self,
        department_id: &str,
        name: &str,
        level: i32,
        authority_level: i32,
        description: Option<&str>,
    ) -> Result<Position> {
        let id = Uuid::new_v4().to_string();
        
        sqlx::query(
            r#"
            INSERT INTO positions (id, department_id, name, level, authority_level, description, status, created_at)
            VALUES (?, ?, ?, ?, ?, ?, 'active', datetime('now'))
            "#
        )
        .bind(&id)
        .bind(department_id)
        .bind(name)
        .bind(level)
        .bind(authority_level)
        .bind(description)
        .execute(self.pool)
        .await?;

        self.find_by_id(&id).await
    }

    /// Busca cargo pelo ID
    pub async fn find_by_id(&self, id: &str) -> Result<Position> {
        let row: PositionRow = sqlx::query_as(
            "SELECT id, department_id, name, level, authority_level, description, status, created_at FROM positions WHERE id = ?"
        )
        .bind(id)
        .fetch_one(self.pool)
        .await?;
        
        Ok(row.into())
    }

    /// Busca cargo pelo ID (opcional)
    pub async fn find_by_id_optional(&self, id: &str) -> Result<Option<Position>> {
        let row: Option<PositionRow> = sqlx::query_as(
            "SELECT id, department_id, name, level, authority_level, description, status, created_at FROM positions WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;
        
        Ok(row.map(|r| r.into()))
    }

    /// Lista cargos de um departamento
    pub async fn list_by_department(&self, department_id: &str) -> Result<Vec<Position>> {
        let rows: Vec<PositionRow> = sqlx::query_as(
            "SELECT id, department_id, name, level, authority_level, description, status, created_at FROM positions WHERE department_id = ? AND status = 'active' ORDER BY level, name"
        )
        .bind(department_id)
        .fetch_all(self.pool)
        .await?;
        
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Verifica se cargo pertence ao departamento
    pub async fn belongs_to_department(&self, position_id: &str, department_id: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM positions WHERE id = ? AND department_id = ? AND status = 'active'"
        )
        .bind(position_id)
        .bind(department_id)
        .fetch_one(self.pool)
        .await?;
        
        Ok(count > 0)
    }

    /// Verifica se cargo pertence a empresa (via cadeia)
    pub async fn belongs_to_company(&self, position_id: &str, company_id: &str) -> Result<bool> {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) = 1 
            FROM positions p
            JOIN departments d ON p.department_id = d.id
            WHERE p.id = ? AND d.company_id = ? AND p.status = 'active'
            "#
        )
        .bind(position_id)
        .bind(company_id)
        .fetch_one(self.pool)
        .await?;
        
        Ok(exists)
    }

    /// Verifica se cargo existe
    pub async fn exists(&self, position_id: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM positions WHERE id = ?"
        )
        .bind(position_id)
        .fetch_one(self.pool)
        .await?;
        
        Ok(count > 0)
    }

    /// Atualiza o nome do cargo
    pub async fn update_name(&self, position_id: &str, new_name: &str) -> Result<()> {
        sqlx::query(
            "UPDATE positions SET name = ? WHERE id = ?"
        )
        .bind(new_name)
        .bind(position_id)
        .execute(self.pool)
        .await?;
        
        Ok(())
    }

    /// Atualiza a descrição do cargo
    pub async fn update_description(&self, position_id: &str, new_description: Option<&str>) -> Result<()> {
        sqlx::query(
            "UPDATE positions SET description = ? WHERE id = ?"
        )
        .bind(new_description)
        .bind(position_id)
        .execute(self.pool)
        .await?;
        
        Ok(())
    }

    /// Arquiva um cargo
    pub async fn archive(&self, position_id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE positions SET status = 'inactive' WHERE id = ?"
        )
        .bind(position_id)
        .execute(self.pool)
        .await?;
        
        Ok(())
    }

    /// Verifica se cargo pode ser arquivado (sem funcionários ativos)
    pub async fn can_archive(&self, position_id: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM employees WHERE position_id = ? AND is_active = 1"
        )
        .bind(position_id)
        .fetch_one(self.pool)
        .await?;
        
        Ok(count == 0)
    }

    /// Busca cargo com cadeia de pertencimento (departamento → empresa)
    pub async fn find_with_company_validation(
        &self, 
        position_id: &str, 
        company_id: &str
    ) -> Result<Option<Position>> {
        let row: Option<PositionRow> = sqlx::query_as(
            r#"
            SELECT p.id, p.department_id, p.name, p.level, p.authority_level, p.description, p.status, p.created_at
            FROM positions p
            JOIN departments d ON p.department_id = d.id
            WHERE p.id = ? AND d.company_id = ? AND p.status = 'active'
            "#
        )
        .bind(position_id)
        .bind(company_id)
        .fetch_optional(self.pool)
        .await?;
        
        Ok(row.map(|r| r.into()))
    }
}