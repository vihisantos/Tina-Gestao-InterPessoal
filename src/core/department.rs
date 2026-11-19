//! Entidade Department - departamento dentro de uma empresa

/// Status do departamento
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DepartmentStatus {
    /// Departamento ativo
    #[default]
    Active,
    /// Departamento inativo
    Inactive,
    /// Departamento suspenso
    Suspended,
}

impl std::fmt::Display for DepartmentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Inactive => write!(f, "inactive"),
            Self::Suspended => write!(f, "suspended"),
        }
    }
}

/// Representa um departamento dentro de uma empresa
#[derive(Debug, Clone)]
pub struct Department {
    /// ID único do departamento
    pub id: String,
    /// ID da empresa proprietária
    pub company_id: String,
    /// Código único dentro da empresa
    pub code: String,
    /// Nome do departamento
    pub name: String,
    /// Descrição opcional
    pub description: Option<String>,
    /// Status do departamento
    pub status: DepartmentStatus,
    /// Data de criação em RFC3339
    pub created_at: String,
}

impl Department {
    /// Cria um novo departamento
    pub fn new(
        id: String,
        company_id: String,
        code: String,
        name: String,
        description: Option<String>,
        status: DepartmentStatus,
        created_at: String,
    ) -> Self {
        Self {
            id,
            company_id,
            code,
            name,
            description,
            status,
            created_at,
        }
    }

    /// Verifica se o departamento está ativo
    pub fn is_active(&self) -> bool {
        self.status == DepartmentStatus::Active
    }

    /// Retorna o nome do departamento
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Retorna o código do departamento
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Retorna o ID da empresa
    pub fn company_id(&self) -> &str {
        &self.company_id
    }
}
