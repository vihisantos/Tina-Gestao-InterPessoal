//! Repositório de instalações

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::core::Installation;
use crate::error::Result;

/// Representa uma linha de instalação do banco
#[derive(sqlx::FromRow, Debug)]
struct InstallationRow {
    id: String,
    name: String,
    root_path: String,
    version: String,
    owner_uuid: String,
    created_at: String,
}

/// Repositório para operações de instalação
pub struct InstallationRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> InstallationRepository<'a> {
    /// Cria um novo repositório de instalação
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Cria uma nova instalação
    pub async fn create(&self, name: &str, owner_uuid: &str) -> Result<Installation> {
        let id = Uuid::new_v4().to_string();

        self.create_with_id(name, &id, owner_uuid).await
    }

    /// Cria uma instalação com ID específico (para inicialização idempotente)
    pub async fn create_with_id(
        &self,
        name: &str,
        id: &str,
        owner_uuid: &str,
    ) -> Result<Installation> {
        sqlx::query(
            r#"
            INSERT INTO installations (id, name, root_path, version, owner_uuid, created_at)
            VALUES (?, ?, ?, ?, ?, datetime('now'))
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(id) // Use the ID as root_path to avoid conflicts when creating multiple installations in tests
        .bind(env!("CARGO_PKG_VERSION"))
        .bind(owner_uuid)
        .execute(self.pool)
        .await?;

        let row: InstallationRow = sqlx::query_as(
            "SELECT id, name, root_path, version, owner_uuid, created_at FROM installations WHERE id = ?"
        )
        .bind(id)
        .fetch_one(self.pool)
        .await?;

        Ok(Installation {
            id: row.id,
            name: row.name,
            root_path: row.root_path,
            version: row.version,
            owner_uuid: row.owner_uuid,
            created_at: row.created_at,
        })
    }

    /// Busca instalação pelo ID
    pub async fn find_by_id(&self, id: &str) -> Result<Option<Installation>> {
        let row: Option<InstallationRow> = sqlx::query_as(
            "SELECT id, name, root_path, version, owner_uuid, created_at FROM installations WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| Installation {
            id: row.id,
            name: row.name,
            root_path: row.root_path,
            version: row.version,
            owner_uuid: row.owner_uuid,
            created_at: row.created_at,
        }))
    }

    /// Busca instalação pelo UUID do proprietário
    pub async fn find_by_owner_uuid(&self, owner_uuid: &str) -> Result<Option<Installation>> {
        let row: Option<InstallationRow> = sqlx::query_as(
            "SELECT id, name, root_path, version, owner_uuid, created_at FROM installations WHERE owner_uuid = ?"
        )
        .bind(owner_uuid)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| Installation {
            id: row.id,
            name: row.name,
            root_path: row.root_path,
            version: row.version,
            owner_uuid: row.owner_uuid,
            created_at: row.created_at,
        }))
    }

    /// Verifica se instalação existe
    pub async fn exists(&self) -> Result<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM installations")
            .fetch_one(self.pool)
            .await?;
        Ok(count > 0)
    }

    /// Busca a instalação única (ou cria padrão)
    pub async fn get_or_create_default(&self) -> Result<Installation> {
        if let Some(inst) = self.find_by_id("default").await? {
            return Ok(inst);
        }

        // Criar instalação padrão
        let owner_uuid = Uuid::new_v4().to_string();
        self.create("Tina", &owner_uuid).await
    }

    /// Lista empresas por instalação (para validação de isolamento)
    pub async fn list_companies(&self, installation_id: &str) -> Result<Vec<crate::core::Company>> {
        use crate::core::company::{Company, CompanyStatus};

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

        let rows: Vec<CompanyRow> = sqlx::query_as(
            "SELECT id, installation_id, name, status, created_at FROM companies WHERE installation_id = ? AND status = 'active' ORDER BY name"
        )
        .bind(installation_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}
