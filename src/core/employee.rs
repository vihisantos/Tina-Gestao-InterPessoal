//! Entidade Employee - funcionário IA da Tina.
//!
//! Cada funcionário é uma entidade independente com identidade, cargo,
//! departamento, empresa, responsabilidades, estado operacional e
//! relação hierárquica. Não é o proprietário da instalação.

/// Status operacional do funcionário
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmployeeStatus {
    /// Funcionário ativo e operacional
    #[default]
    Active,
    /// Funcionário desativado (não excluído fisicamente)
    Inactive,
}

impl std::fmt::Display for EmployeeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Inactive => write!(f, "inactive"),
        }
    }
}

/// Representa um funcionário IA da Tina.
///
/// Pertence a uma empresa da instalação, associado a um departamento
/// e cargo válidos. Pode ter um superior direto (relacionamento próprio).
/// Não há profundidade máxima artificial na hierarquia.
#[derive(Debug, Clone)]
pub struct Employee {
    /// ID estável e persistente do funcionário
    pub id: String,
    /// Nome/identidade do funcionário
    pub name: String,
    /// ID do cargo ao qual está associado
    pub position_id: String,
    /// Responsabilidades do funcionário
    pub responsibilities: Option<String>,
    /// ID do superior direto (opcional, relacionamento próprio)
    pub supervisor_id: Option<String>,
    /// Estado operacional
    pub status: EmployeeStatus,
    /// Data de criação em RFC3339
    pub created_at: String,
    /// Data de última atualização em RFC3339
    pub updated_at: String,
    /// Data de desativação, quando aplicável
    pub deactivated_at: Option<String>,
}

impl Employee {
    /// Cria um novo funcionário ativo
    pub fn new(
        id: String,
        name: String,
        position_id: String,
        responsibilities: Option<String>,
        supervisor_id: Option<String>,
        created_at: String,
        updated_at: String,
    ) -> Self {
        Self {
            id,
            name,
            position_id,
            responsibilities,
            supervisor_id,
            status: EmployeeStatus::Active,
            created_at,
            updated_at,
            deactivated_at: None,
        }
    }

    /// Verifica se o funcionário está ativo
    pub fn is_active(&self) -> bool {
        self.status == EmployeeStatus::Active
    }

    /// Verifica se o funcionário está desativado
    pub fn is_inactive(&self) -> bool {
        self.status == EmployeeStatus::Inactive
    }

    /// Retorna o nome do funcionário
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Retorna o ID do cargo
    pub fn position_id(&self) -> &str {
        &self.position_id
    }

    /// Retorna o ID do superior direto, se houver
    pub fn supervisor_id(&self) -> Option<&str> {
        self.supervisor_id.as_deref()
    }

    /// Retorna as responsabilidades
    pub fn responsibilities(&self) -> Option<&str> {
        self.responsibilities.as_deref()
    }

    /// Retorna a data de desativação, se houver
    pub fn deactivated_at(&self) -> Option<&str> {
        self.deactivated_at.as_deref()
    }
}
