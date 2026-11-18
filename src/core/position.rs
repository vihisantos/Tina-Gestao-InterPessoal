//! Entidade Position - cargo dentro de um departamento

/// Status do cargo
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PositionStatus {
    /// Cargo ativo
    #[default]
    Active,
    /// Cargo inativo
    Inactive,
    /// Cargo suspenso
    Suspended,
}

impl std::fmt::Display for PositionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Inactive => write!(f, "inactive"),
            Self::Suspended => write!(f, "suspended"),
        }
    }
}

/// Representa um cargo dentro de um departamento
#[derive(Debug, Clone)]
pub struct Position {
    /// ID único do cargo
    pub id: String,
    /// ID do departamento proprietário
    pub department_id: String,
    /// Nome do cargo
    pub name: String,
    /// Nível hierárquico (0 = topo)
    pub level: i32,
    /// Nível de autoridade (para permissões)
    pub authority_level: i32,
    /// Descrição opcional
    pub description: Option<String>,
    /// Status do cargo
    pub status: PositionStatus,
    /// Data de criação em RFC3339
    pub created_at: String,
}

impl Position {
    /// Cria um novo cargo
    pub fn new(
        id: String,
        department_id: String,
        name: String,
        level: i32,
        authority_level: i32,
        description: Option<String>,
        status: PositionStatus,
        created_at: String,
    ) -> Self {
        Self {
            id,
            department_id,
            name,
            level,
            authority_level,
            description,
            status,
            created_at,
        }
    }

    /// Verifica se o cargo está ativo
    pub fn is_active(&self) -> bool {
        self.status == PositionStatus::Active
    }

    /// Retorna o nome do cargo
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Retorna o ID do departamento
    pub fn department_id(&self) -> &str {
        &self.department_id
    }

    /// Retorna o nível hierárquico
    pub fn level(&self) -> i32 {
        self.level
    }

    /// Retorna o nível de autoridade
    pub fn authority_level(&self) -> i32 {
        self.authority_level
    }
}