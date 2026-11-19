//! Entidade Company - representa uma organização dentro da instalação

/// Status da empresa
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompanyStatus {
    /// Empresa ativa
    #[default]
    Active,
    /// Empresa inativa
    Inactive,
    /// Empresa suspenso
    Suspended,
}

impl std::fmt::Display for CompanyStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Inactive => write!(f, "inactive"),
            Self::Suspended => write!(f, "suspended"),
        }
    }
}

/// Representa uma empresa dentro de uma instalação.
#[derive(Debug, Clone)]
pub struct Company {
    /// ID único da empresa
    pub id: String,
    /// ID da instalação proprietária
    pub installation_id: String,
    /// Nome da empresa
    pub name: String,
    /// Status da empresa
    pub status: CompanyStatus,
    /// Data de criação em RFC3339
    pub created_at: String,
}

impl Company {
    /// Cria uma nova empresa
    pub fn new(
        id: String,
        installation_id: String,
        name: String,
        status: CompanyStatus,
        created_at: String,
    ) -> Self {
        Self {
            id,
            installation_id,
            name,
            status,
            created_at,
        }
    }

    /// Verifica se a empresa está ativa
    pub fn is_active(&self) -> bool {
        self.status == CompanyStatus::Active
    }

    /// Retorna o nome da empresa
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Retorna o ID da instalação
    pub fn installation_id(&self) -> &str {
        &self.installation_id
    }
}
