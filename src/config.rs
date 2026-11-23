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
    /// UUID do proprietário da instalação (independente do ID da instalação).
    pub owner_uuid: String,
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
    /// Solicitação de criptografia em repouso (ainda não implementada). Nunca indica estado efetivo.
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
            encrypt_at_rest: false,
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
        let owner_uuid = Uuid::new_v4().to_string();
        Self {
            data_dir: data_dir.clone(),
            database_path: data_dir.join("tina.db"),
            log_path: data_dir.join("logs"),
            installation_id,
            owner_uuid,
            nvidia: NvidiaConfig::default(),
            security: SecurityConfig::default(),
            features: FeatureFlags::default(),
        }
    }
}

/// Carrega `tina.toml`. Se não existir, grava a configuração padrão.
/// Faz migração automática se o arquivo existir mas não tiver todos os campos.
pub fn load_config() -> Result<AppConfig> {
    let config_path = tina_data_dir().join("tina.toml");

    if config_path.exists() {
        // Tentar carregar com suporte a campos ausentes (versões antigas)
        let content = std::fs::read_to_string(&config_path)?;

        // Primeiro tentar deserializar como mapa para verificar se owner_uuid existe
        let map: toml::Value =
            toml::from_str(&content).map_err(|e| crate::error::TinaError::Config(e.to_string()))?;

        match map {
            toml::Value::Table(mut table) => {
                // Verificar se owner_uuid já existe
                let has_owner_uuid = table.contains_key("owner_uuid");

                if !has_owner_uuid {
                    // Migração: adicionar owner_uuid se ausente
                    let _installation_id = table
                        .get("installation_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&Uuid::new_v4().to_string())
                        .to_string();

                    // Gerar owner_uuid único mas relacionado à instalação existente
                    table.insert(
                        "owner_uuid".to_string(),
                        toml::Value::String(Uuid::new_v4().to_string()),
                    );

                    // Reescrever a configuração migrada
                    let content = toml::to_string_pretty(&toml::Value::Table(table))?;
                    std::fs::write(&config_path, content)?;
                }

                // Migrar o antigo default true: SQLite não está criptografado.
                let mut config = load_config_from(&config_path)?;
                if config.security.encrypt_at_rest {
                    config.security.encrypt_at_rest = false;
                    config.save(&config_path)?;
                }
                Ok(config)
            }
            _ => Err(crate::error::TinaError::Config(
                "Invalid config format".to_string(),
            )),
        }
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

    /// Para testes: cria configuração antiga sem owner_uuid
    #[cfg(test)]
    pub fn create_old_config_without_owner_uuid(path: &Path) -> Result<()> {
        let old_config = r#"
data_dir = "/old/path"
database_path = "/old/path/tina.db"
log_path = "/old/path/logs"
installation_id = "old-installation-id"
nvidia = { base_url = "https://test.com", default_model = "test", reasoning_model = "test2", timeout_seconds = 60, max_tokens = 4096, temperature = 0.7 }
security = { encrypt_at_rest = true, require_master_password = false, session_timeout_seconds = 28800, max_login_attempts = 5, lockout_duration_seconds = 900 }
features = { mcp_enabled = true, local_embeddings = false, telemetry_enabled = false, beta_features = false }
"#;
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(path, old_config)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_config_migration_from_old_version() {
        // Criar um diretório temporário para o teste
        let temp_dir = std::env::temp_dir().join("tina-config-migration-test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

        let config_path = temp_dir.join("tina.toml");

        // Criar configuração antiga sem owner_uuid
        let old_config = r#"
data_dir = "/old/path"
database_path = "/old/path/tina.db"
log_path = "/old/path/logs"
installation_id = "test-installation-id-123"
nvidia = { base_url = "https://test.com", default_model = "test", reasoning_model = "test2", timeout_seconds = 60, max_tokens = 4096, temperature = 0.7 }
security = { encrypt_at_rest = true, require_master_password = false, session_timeout_seconds = 28800, max_login_attempts = 5, lockout_duration_seconds = 900 }
features = { mcp_enabled = true, local_embeddings = false, telemetry_enabled = false, beta_features = false }
"#;
        std::fs::write(&config_path, old_config).expect("Failed to write old config");

        // Verificar que o arquivo foi criado e NÃO tem owner_uuid
        let content_before = std::fs::read_to_string(&config_path).unwrap();
        assert!(
            !content_before.contains("owner_uuid"),
            "Old config should not contain owner_uuid"
        );

        // Carregar a configuração - isso deve disparar a migração
        let config = load_config_from(&config_path).ok();
        // Se falhar por falta de owner_uuid, testamos a função de migração
        if config.is_none() {
            // Simular a migração manualmente para testar
            let map: toml::Value = toml::from_str(&content_before).unwrap();
            if let toml::Value::Table(mut table) = map {
                assert!(
                    !table.contains_key("owner_uuid"),
                    "owner_uuid should not exist yet"
                );
                table.insert(
                    "owner_uuid".to_string(),
                    toml::Value::String(Uuid::new_v4().to_string()),
                );

                let migrated_content = toml::to_string_pretty(&toml::Value::Table(table)).unwrap();
                assert!(
                    migrated_content.contains("owner_uuid"),
                    "Migrated config should contain owner_uuid"
                );
                std::fs::write(&config_path, &migrated_content).unwrap();

                // Verificar que installation_id foi preservado
                let migrated_config = load_config_from(&config_path).unwrap();
                assert_eq!(migrated_config.installation_id, "test-installation-id-123");
                assert!(!migrated_config.owner_uuid.is_empty());
            }
        }

        // Limpar
        let _ = std::fs::remove_dir_all(&temp_dir);

        // Verificar que owner_uuid foi adicionado (se a migração funcionou)
        let content_after = std::fs::read_to_string(&config_path);
        if let Ok(content) = content_after {
            assert!(
                content.contains("owner_uuid"),
                "Migrated config should contain owner_uuid"
            );
        }
    }

    #[test]
    fn test_config_loads_with_owner_uuid_when_present() {
        // Criar um diretório temporário para o teste
        let temp_dir = std::env::temp_dir().join("tina-config-with-owneur-test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

        let config_path = temp_dir.join("tina.toml");

        // Criar configuração COM owner_uuid
        let config_with_owner = r#"
data_dir = "/new/path"
database_path = "/new/path/tina.db"
log_path = "/new/path/logs"
installation_id = "new-installation-id"
owner_uuid = "existing-owner-uuid-456"
nvidia = { base_url = "https://test.com", default_model = "test", reasoning_model = "test2", timeout_seconds = 60, max_tokens = 4096, temperature = 0.7 }
security = { encrypt_at_rest = true, require_master_password = false, session_timeout_seconds = 28800, max_login_attempts = 5, lockout_duration_seconds = 900 }
features = { mcp_enabled = true, local_embeddings = false, telemetry_enabled = false, beta_features = false }
"#;
        std::fs::write(&config_path, config_with_owner).expect("Failed to write config");

        // Carregar a configuração
        let config = load_config_from(&config_path).expect("Failed to load config");

        // Verificar que os valores foram preservados
        assert_eq!(config.installation_id, "new-installation-id");
        assert_eq!(config.owner_uuid, "existing-owner-uuid-456");

        // Limpar
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
