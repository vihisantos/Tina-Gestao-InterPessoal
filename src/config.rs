//! Configuração local da plataforma Tina.
//!
//! No Windows, `dirs::config_dir()` aponta para `%APPDATA%`. Assim, os arquivos
//! padrão ficam em `%APPDATA%\tina\`:
//! - `tina.toml` — configuração;
//! - `tina.db` — banco SQLite;
//! - `logs/` — diretório de logs.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Result;

/// Diretório de dados da instalação Tina.
fn tina_data_dir() -> PathBuf {
    dirs::config_dir()
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tina")
}

/// Configuração da aplicação.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Diretório raiz de dados da instalação.
    pub data_dir: PathBuf,
    /// Caminho completo do arquivo SQLite.
    pub database_path: PathBuf,
    /// Diretório de logs locais.
    pub log_path: PathBuf,
    /// ID da instalação atual (do registro de instalações).
    pub installation_id: String,
    /// Configuração do provedor autorizado (NVIDIA NIM).
    pub nvidia: NvidiaConfig,
    /// Políticas de segurança locais.
    pub security: SecurityConfig,
    /// Habilitação de funcionalidades opcionais.
    pub features: FeatureFlags,
}

/// Configuração do provedor NVIDIA NIM (único autorizado).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NvidiaConfig {
    /// URL base da API NIM.
    pub base_url: String,
    /// Modelo padrão para conversação.
    pub default_model: String,
    /// Modelo padrão para raciocínio.
    pub reasoning_model: String,
    /// Timeout das requisições, em segundos.
    pub timeout_seconds: u64,
    /// Limite de tokens por resposta.
    pub max_tokens: u32,
    /// Temperatura de amostragem.
    pub temperature: f32,
}

impl Default for NvidiaConfig {
    fn default() -> Self {
        Self {
            base_url: "https://integrate.api.nvidia.com/v1".to_string(),
            default_model: "nvidia/nemotron-3.5-bolt-e4b".to_string(),
            reasoning_model: "nvidia/nemotron-3.5-lightning-30b-a3b".to_string(),
            timeout_seconds: 60,
            max_tokens: 4096,
            temperature: 0.7,
        }
    }
}

/// Configuração de segurança local.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Habilita criptografia em repouso quando implementada.
    pub encrypt_at_rest: bool,
    /// Exige senha mestra para operações sensíveis quando implementado.
    pub require_master_password: bool,
    /// Duração da sessão, em segundos (0 = nunca expira).
    pub session_timeout_seconds: u64,
    /// Tentativas de login antes do bloqueio.
    pub max_login_attempts: u32,
    /// Duração do bloqueio, em segundos.
    pub lockout_duration_seconds: u64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            encrypt_at_rest: true,
            require_master_password: false,
            session_timeout_seconds: 8 * 3600,
            max_login_attempts: 5,
            lockout_duration_seconds: 900,
        }
    }
}

/// Flags de funcionalidades opcionais.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Integração MCP (futura).
    pub mcp_enabled: bool,
    /// Embeddings locais (futuro).
    pub local_embeddings: bool,
    /// Telemetria estritamente local (futuro).
    pub telemetry_enabled: bool,
    /// Funcionalidades experimentais.
    pub beta_features: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            mcp_enabled: true,
            local_embeddings: false,
            telemetry_enabled: false,
            beta_features: false,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        let data_dir = tina_data_dir();
        let installation_id = Uuid::new_v4().to_string();
        Self {
            data_dir: data_dir.clone(),
            database_path: data_dir.join("tina.db"),
            log_path: data_dir.join("logs"),
            installation_id,
            nvidia: NvidiaConfig::default(),
            security: SecurityConfig::default(),
            features: FeatureFlags::default(),
        }
    }
}

/// Carrega `tina.toml`. Se não existir, grava a configuração padrão.
pub fn load_config() -> Result<AppConfig> {
    let config_path = tina_data_dir().join("tina.toml");

    if config_path.exists() {
        load_config_from(&config_path)
    } else {
        let config = AppConfig::default();
        config.save(&config_path)?;
        Ok(config)
    }
}

/// Lê uma configuração a partir de um caminho explícito.
pub fn load_config_from(path: &Path) -> Result<AppConfig> {
    let content = std::fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

impl AppConfig {
    /// Grava a configuração em formato TOML.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Cria os diretórios necessários antes de abrir o banco.
    pub fn ensure_directories(&self) -> Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        if let Some(parent) = self.database_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::create_dir_all(&self.log_path)?;
        Ok(())
    }
}
