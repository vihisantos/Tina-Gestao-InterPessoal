//! Repositório de empresas

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::core::company::{Company, CompanyStatus};
use crate::error::{Result, TinaError};
use crate::infra::audit::{begin_owner_write, commit_owner_write};

/// Representa uma linha de empresa do banco
#[derive(sqlx::FromRow, Debug)]
struct CompanyRow {
    id: String,
    installation_id: String,
    name: String,
    status: String,
    created_at: String,
}

impl From<CompanyRow> for Company {
    fn from(row: CompanyRow) -> Self {
        let status = match row.status.as_str() {
            "active" => CompanyStatus::Active,
            "inactive" => CompanyStatus::Inactive,
            "suspended" => CompanyStatus::Suspended,
            _ => CompanyStatus::Inactive,
        };

        Self {
            id: row.id,
            installation_id: row.installation_id,
            name: row.name,
            status,
            created_at: row.created_at,
        }
    }
}

/// Repositório para operações de empresa
pub struct CompanyRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> CompanyRepository<'a> {
    /// Cria um novo repositório de empresas
    pub(crate) fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Cria uma nova empresa
    ///
    /// # Argumentos
    /// * `installation_id` - ID da instalação (validado pelo caller)
    /// * `name` - Nome da empresa
    ///
    /// # Retorno
    /// A empresa criada com ID gerado automaticamente
    pub async fn create(&self, installation_id: &str, name: &str) -> Result<Company> {
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO companies (id, installation_id, name, status, created_at)
            VALUES (?, ?, ?, 'active', datetime('now'))
            "#,
        )
        .bind(&id)
        .bind(installation_id)
        .bind(name)
        .execute(self.pool)
        .await?;

        self.find_by_id(&id).await
    }

    /// Busca empresa pelo ID
    pub async fn find_by_id(&self, id: &str) -> Result<Company> {
        let row: CompanyRow = sqlx::query_as(
            "SELECT id, installation_id, name, status, created_at FROM companies WHERE id = ?",
        )
        .bind(id)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into())
    }

    /// Busca empresa pelo ID (opcional)
    pub async fn find_by_id_optional(&self, id: &str) -> Result<Option<Company>> {
        let row: Option<CompanyRow> = sqlx::query_as(
            "SELECT id, installation_id, name, status, created_at FROM companies WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }

    /// Lista todas as empresas de uma instalação
    ///
    /// # Argumentos
    /// * `installation_id` - ID da instalação
    pub async fn list_by_installation(&self, installation_id: &str) -> Result<Vec<Company>> {
        let rows: Vec<CompanyRow> = sqlx::query_as(
            "SELECT id, installation_id, name, status, created_at FROM companies WHERE installation_id = ? AND status = 'active' ORDER BY name"
        )
        .bind(installation_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Verifica se empresa pertence à instalação
    pub async fn belongs_to_installation(
        &self,
        company_id: &str,
        installation_id: &str,
    ) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM companies WHERE id = ? AND installation_id = ? AND status = 'active'"
        )
        .bind(company_id)
        .bind(installation_id)
        .fetch_one(self.pool)
        .await?;

        Ok(count > 0)
    }

    /// Atualiza o nome da empresa (apenas se pertencer à instalação)
    pub async fn update_name(
        &self,
        company_id: &str,
        installation_id: &str,
        new_name: &str,
    ) -> Result<()> {
        let result =
            sqlx::query("UPDATE companies SET name = ? WHERE id = ? AND installation_id = ?")
                .bind(new_name)
                .bind(company_id)
                .bind(installation_id)
                .execute(self.pool)
                .await?;

        if result.rows_affected() == 0 {
            return Err(TinaError::Other(
                "Company not found or doesn't belong to installation".to_string(),
            )
            .into());
        }

        Ok(())
    }

    /// Atualiza apenas o nome da empresa (sem verificação de instalação)
    /// Usado internamente quando a verificação já foi feita no nível do comando
    pub async fn update_name_unrestricted(&self, company_id: &str, new_name: &str) -> Result<()> {
        sqlx::query("UPDATE companies SET name = ? WHERE id = ?")
            .bind(new_name)
            .bind(company_id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// Arquiva uma empresa (não exclui fisicamente) - apenas se pertencer à instalação
    pub async fn archive(&self, company_id: &str, installation_id: &str) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE companies
            SET status = 'inactive'
            WHERE id = ?
              AND installation_id = ?
              AND status = 'active'
              AND NOT EXISTS (
                  SELECT 1
                  FROM departments d
                  WHERE d.company_id = companies.id
                    AND d.status = 'active'
              )
            "#,
        )
        .bind(company_id)
        .bind(installation_id)
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(TinaError::Other(
                "Company not found or doesn't belong to installation".to_string(),
            )
            .into());
        }

        Ok(())
    }

    /// Arquiva uma empresa sem verificação de instalação (usado quando já verificado)
    pub async fn archive_unrestricted(&self, company_id: &str) -> Result<()> {
        sqlx::query("UPDATE companies SET status = 'inactive' WHERE id = ?")
            .bind(company_id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// Verifica se empresa pode ser arquivada (sem departamentos ativos)
    pub async fn can_archive(&self, company_id: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM departments WHERE company_id = ? AND status = 'active'",
        )
        .bind(company_id)
        .fetch_one(self.pool)
        .await?;

        Ok(count == 0)
    }

    /// Verifica se empresa pode ser arquivada - apenas se pertencer à instalação
    pub async fn can_archive_for_installation(
        &self,
        company_id: &str,
        installation_id: &str,
    ) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM departments WHERE company_id = ? AND status = 'active'",
        )
        .bind(company_id)
        .fetch_one(self.pool)
        .await?;

        if count > 0 {
            return Ok(false);
        }

        // Verificar se a empresa pertence à instalação
        let company_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM companies WHERE id = ? AND installation_id = ?",
        )
        .bind(company_id)
        .bind(installation_id)
        .fetch_one(self.pool)
        .await?;

        Ok(company_count > 0)
    }

    /// Cria empresa com autoria verificada pelo serviço de aplicação.
    /// Garante atomicidade entre criação e auditoria.
    pub(crate) async fn create_as_owner(
        &self,
        installation_id: &str,
        owner_id: &str,
        name: &str,
    ) -> Result<Company> {
        let mut tx = begin_owner_write(self.pool, installation_id, owner_id).await?;
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO companies (id, installation_id, name, status, created_at)
            VALUES (?, ?, ?, 'active', datetime('now'))
            "#,
        )
        .bind(&id)
        .bind(installation_id)
        .bind(name)
        .execute(&mut *tx)
        .await?;

        let row: CompanyRow = sqlx::query_as(
            "SELECT id, installation_id, name, status, created_at FROM companies WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&mut *tx)
        .await?;
        commit_owner_write(tx, installation_id).await?;
        Ok(row.into())
    }

    /// Atualiza o nome da empresa com autoria no mesmo commit.
    pub(crate) async fn update_name_as_owner(
        &self,
        company_id: &str,
        installation_id: &str,
        owner_id: &str,
        new_name: &str,
    ) -> Result<()> {
        let mut tx = begin_owner_write(self.pool, installation_id, owner_id).await?;
        let result = sqlx::query(
            "UPDATE companies SET name = ? WHERE id = ? AND installation_id = ? AND status = 'active'",
        )
        .bind(new_name)
        .bind(company_id)
        .bind(installation_id)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            return Err(TinaError::Unauthorized(
                "empresa fora da instalação".into(),
            ));
        }
        commit_owner_write(tx, installation_id).await
    }

    /// Arquiva uma empresa com autoria e verificação de restrições.
    pub(crate) async fn archive_as_owner(
        &self,
        company_id: &str,
        installation_id: &str,
        owner_id: &str,
    ) -> Result<()> {
        let mut tx = begin_owner_write(self.pool, installation_id, owner_id).await?;
        let result = sqlx::query(
            r#"
            UPDATE companies
            SET status = 'inactive'
            WHERE id = ?
              AND installation_id = ?
              AND status = 'active'
              AND NOT EXISTS (
                  SELECT 1
                  FROM departments d
                  WHERE d.company_id = companies.id
                    AND d.status = 'active'
              )
            "#,
        )
        .bind(company_id)
        .bind(installation_id)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            return Err(TinaError::Other(
                "empresa indisponível ou com departamentos ativos".into(),
            ));
        }
        commit_owner_write(tx, installation_id).await
    }
}
