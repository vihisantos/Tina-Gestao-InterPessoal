//! Tipos de erro da plataforma Tina.

use thiserror::Error;

/// Erro principal das operações da Tina.
#[derive(Error, Debug)]
pub enum TinaError {
    /// Falha em operação do banco local.
    #[error("erro de banco de dados: {0}")]
    Database(#[from] sqlx::Error),

    /// Falha em migração do banco de dados SQLite.
    #[error("erro de migração do banco de dados: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    /// Falha de sistema de arquivos.
    #[error("erro de I/O: {0}")]
    Io(#[from] std::io::Error),

    /// Configuração inválida ou incompleta.
    #[error("configuração inválida: {0}")]
    Config(String),

    /// Falha ao interpretar `tina.toml`.
    #[error("erro ao ler TOML: {0}")]
    TomlParse(#[from] toml::de::Error),

    /// Falha ao gravar `tina.toml`.
    #[error("erro ao gravar TOML: {0}")]
    TomlWrite(#[from] toml::ser::Error),

    /// Falha de autenticação local.
    #[error("erro de autenticação: {0}")]
    Auth(String),

    /// Acesso negado por política local.
    #[error("acesso negado: {0}")]
    Unauthorized(String),

    /// Permissão insuficiente.
    #[error("permissão negada: {0}")]
    PermissionDenied(String),

    /// Falha em inferência com provedor de IA autorizado.
    #[error("erro de inferência: {0}")]
    Inference(String),

    /// Falha em execução de ferramenta.
    #[error("erro de ferramenta: {0}")]
    ToolError(String),

    /// Falha em delegação.
    #[error("erro de delegação: {0}")]
    Delegation(String),

    /// Falha em memória.
    #[error("erro de memória: {0}")]
    Memory(String),

    /// Falha no sistema de eventos.
    #[error("erro de evento: {0}")]
    Event(String),

    /// Resultado inválido.
    #[error("validação falhou: {0}")]
    Validation(String),

    /// Violação de regra de hierarquia organizacional.
    #[error("hierarquia inválida: {0}")]
    Hierarchy(String),

    /// Erro genérico.
    #[error("{0}")]
    Other(String),
}

/// Alias de resultado usado pelo núcleo da Tina.
pub type Result<T> = std::result::Result<T, TinaError>;
