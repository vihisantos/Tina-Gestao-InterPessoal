//! Instalação Tina - identidade única da instalação e seu proprietário

/// Representa uma instalação Tina única no computador do proprietário.
#[derive(Debug, Clone)]
pub struct Installation {
    /// ID único da instalação (UUID)
    pub id: String,
    /// Nome da instalação
    pub name: String,
    /// Caminho raiz da instalação
    pub root_path: String,
    /// Versão da instalação
    pub version: String,
    /// UUID do proprietário (autoridade máxima)
    pub owner_uuid: String,
    /// Data de criação em RFC3339
    pub created_at: String,
}

impl Installation {
    /// Cria uma nova instalação com os dados fornecidos
    pub fn new(
        id: String,
        name: String,
        root_path: String,
        version: String,
        owner_uuid: String,
        created_at: String,
    ) -> Self {
        Self {
            id,
            name,
            root_path,
            version,
            owner_uuid,
            created_at,
        }
    }

    /// Retorna o UUID do proprietário
    pub fn owner_uuid(&self) -> &str {
        &self.owner_uuid
    }

    /// Retorna o ID da instalação
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Retorna o nome da instalação
    pub fn name(&self) -> &str {
        &self.name
    }
}
