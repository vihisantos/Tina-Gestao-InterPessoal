//! Repositório de cargos (positions)

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::core::position::{Position, PositionStatus};
use crate::error::{Result, TinaError};
use crate::infra::audit::{begin_owner_write, commit_owner_write};

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
    pub(crate) fn new(pool: &'a SqlitePool) -> Self {
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
    pub async fn belongs_to_department(
        &self,
        position_id: &str,
        department_id: &str,
    ) -> Result<bool> {
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
            "#,
        )
        .bind(position_id)
        .bind(company_id)
        .fetch_one(self.pool)
        .await?;

        Ok(exists)
    }

    /// Verifica se cargo existe
    pub async fn exists(&self, position_id: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM positions WHERE id = ?")
            .bind(position_id)
            .fetch_one(self.pool)
            .await?;

        Ok(count > 0)
    }

    /// Atualiza o nome do cargo
    pub async fn update_name(&self, position_id: &str, new_name: &str) -> Result<()> {
        sqlx::query("UPDATE positions SET name = ? WHERE id = ?")
            .bind(new_name)
            .bind(position_id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// Atualiza a descrição do cargo
    pub async fn update_description(
        &self,
        position_id: &str,
        new_description: Option<&str>,
    ) -> Result<()> {
        sqlx::query("UPDATE positions SET description = ? WHERE id = ?")
            .bind(new_description)
            .bind(position_id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// Arquiva um cargo
    pub async fn archive(&self, position_id: &str) -> Result<()> {
        sqlx::query("UPDATE positions SET status = 'inactive' WHERE id = ?")
            .bind(position_id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// Verifica se cargo pode ser arquivado (sem funcionários ativos)
    pub async fn can_archive(&self, position_id: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM employees WHERE position_id = ? AND status = 'active'",
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
        company_id: &str,
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

    /// Atualiza o nome do cargo com escopo aplicado atomicamente no SQL.
    pub async fn update_name_with_scope(
        &self,
        position_id: &str,
        installation_id: &str,
        new_name: &str,
    ) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE positions
            SET name = ?
            WHERE id = ?
              AND status = 'active'
              AND EXISTS (
                  SELECT 1
                  FROM departments d
                  JOIN companies c ON c.id = d.company_id
                  WHERE d.id = positions.department_id
                    AND d.status = 'active'
                    AND c.installation_id = ?
                    AND c.status = 'active'
              )
            "#,
        )
        .bind(new_name)
        .bind(position_id)
        .bind(installation_id)
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(TinaError::Other(
                "Position not found or doesn't belong to installation".to_string(),
            )
            .into());
        }
        Ok(())
    }

    /// Atualiza a descrição do cargo com escopo aplicado atomicamente no SQL.
    pub async fn update_description_with_scope(
        &self,
        position_id: &str,
        installation_id: &str,
        new_description: Option<&str>,
    ) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE positions
            SET description = ?
            WHERE id = ?
              AND status = 'active'
              AND EXISTS (
                  SELECT 1
                  FROM departments d
                  JOIN companies c ON c.id = d.company_id
                  WHERE d.id = positions.department_id
                    AND d.status = 'active'
                    AND c.installation_id = ?
                    AND c.status = 'active'
              )
            "#,
        )
        .bind(new_description)
        .bind(position_id)
        .bind(installation_id)
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(TinaError::Other(
                "Position not found or doesn't belong to installation".to_string(),
            )
            .into());
        }
        Ok(())
    }

    /// Arquiva um cargo com escopo aplicado atomicamente no SQL.
    pub async fn archive_with_scope(&self, position_id: &str, installation_id: &str) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE positions
            SET status = 'inactive'
            WHERE id = ?
              AND status = 'active'
              AND NOT EXISTS (
                  SELECT 1
                  FROM employees e
                  WHERE e.position_id = positions.id
                    AND e.status = 'active'
              )
              AND EXISTS (
                  SELECT 1
                  FROM departments d
                  JOIN companies c ON c.id = d.company_id
                  WHERE d.id = positions.department_id
                    AND d.status = 'active'
                    AND c.installation_id = ?
                    AND c.status = 'active'
              )
            "#,
        )
        .bind(position_id)
        .bind(installation_id)
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(TinaError::Other(
                "Position not found or doesn't belong to installation".to_string(),
            )
            .into());
        }
        Ok(())
    }

    /// Criação com autoria verificada pelo serviço de aplicação.
    pub(crate) async fn create_as_owner(
        &self,
        installation_id: &str,
        owner_id: &str,
        department_id: &str,
        name: &str,
        level: i32,
        authority_level: i32,
        description: Option<&str>,
    ) -> Result<Position> {
        let mut tx = begin_owner_write(self.pool, installation_id, owner_id).await?;
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO positions (id, department_id, name, level, authority_level, description, status, created_at)
            VALUES (?, ?, ?, ?, ?, ?, 'active', datetime('now'))
            "#,
        )
        .bind(&id)
        .bind(department_id)
        .bind(name)
        .bind(level)
        .bind(authority_level)
        .bind(description)
        .execute(&mut *tx)
        .await?;

        let row: PositionRow = sqlx::query_as(
            "SELECT id, department_id, name, level, authority_level, description, status, created_at FROM positions WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&mut *tx)
        .await?;
        commit_owner_write(tx, installation_id).await?;
        Ok(row.into())
    }

    /// Atualiza o nome do cargo com autoria no mesmo commit.
    pub(crate) async fn update_name_as_owner(
        &self,
        position_id: &str,
        installation_id: &str,
        owner_id: &str,
        new_name: &str,
    ) -> Result<()> {
        let mut tx = begin_owner_write(self.pool, installation_id, owner_id).await?;
        let result = sqlx::query(
            r#"
            UPDATE positions
            SET name = ?
            WHERE id = ?
              AND status = 'active'
              AND EXISTS (
                  SELECT 1
                  FROM departments d
                  JOIN companies c ON c.id = d.company_id
                  WHERE d.id = positions.department_id
                    AND d.status = 'active'
                    AND c.installation_id = ?
                    AND c.status = 'active'
              )
            "#,
        )
        .bind(new_name)
        .bind(position_id)
        .bind(installation_id)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            return Err(TinaError::Unauthorized("cargo fora da instalação".into()));
        }
        commit_owner_write(tx, installation_id).await
    }

    /// Atualiza a descrição do cargo com autoria no mesmo commit.
    pub(crate) async fn update_description_as_owner(
        &self,
        position_id: &str,
        installation_id: &str,
        owner_id: &str,
        new_description: Option<&str>,
    ) -> Result<()> {
        let mut tx = begin_owner_write(self.pool, installation_id, owner_id).await?;
        let result = sqlx::query(
            r#"
            UPDATE positions
            SET description = ?
            WHERE id = ?
              AND status = 'active'
              AND EXISTS (
                  SELECT 1
                  FROM departments d
                  JOIN companies c ON c.id = d.company_id
                  WHERE d.id = positions.department_id
                    AND d.status = 'active'
                    AND c.installation_id = ?
                    AND c.status = 'active'
              )
            "#,
        )
        .bind(new_description)
        .bind(position_id)
        .bind(installation_id)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            return Err(TinaError::Unauthorized("cargo fora da instalação".into()));
        }
        commit_owner_write(tx, installation_id).await
    }

    /// Arquiva um cargo com autoria e verificação de restrições.
    pub(crate) async fn archive_as_owner(
        &self,
        position_id: &str,
        installation_id: &str,
        owner_id: &str,
    ) -> Result<()> {
        let mut tx = begin_owner_write(self.pool, installation_id, owner_id).await?;
        let result = sqlx::query(
            r#"
            UPDATE positions
            SET status = 'inactive'
            WHERE id = ?
              AND status = 'active'
              AND NOT EXISTS (
                  SELECT 1
                  FROM employees e
                  WHERE e.position_id = positions.id
                  AND e.status = 'active'
              )
              AND EXISTS (
                  SELECT 1
                  FROM departments d
                  JOIN companies c ON c.id = d.company_id
                  WHERE d.id = positions.department_id
                    AND d.status = 'active'
                    AND c.installation_id = ?
                    AND c.status = 'active'
              )
            "#,
        )
        .bind(position_id)
        .bind(installation_id)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            return Err(TinaError::Other(
                "cargo indisponível ou com funcionários ativos".into(),
            ));
        }
        commit_owner_write(tx, installation_id).await
    }
}
