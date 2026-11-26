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
/// Migra configurações legadas de forma segura, idempotente e com backup.
pub async fn load_config() -> Result<AppConfig> {
    load_config_at(&tina_data_dir().join("tina.toml")).await
}

/// Carrega/migra configuração em um caminho explícito.
pub async fn load_config_at(config_path: &Path) -> Result<AppConfig> {
    if !config_path.exists() {
        let config = AppConfig::default();
        config.save(config_path)?;
        return Ok(config);
    }

    let content = std::fs::read_to_string(config_path)?;
    let mut table = parse_config_table(&content, config_path)?;

    let missing_owner_uuid = !table.contains_key("owner_uuid");
    let missing_installation_id = !table.contains_key("installation_id");

    if missing_owner_uuid || missing_installation_id {
        let mut migrated = table.clone();
        migrate_legacy_fields(&mut migrated, missing_owner_uuid, missing_installation_id).await?;
        // Validação completa antes de qualquer escrita: garante que não criamos backup
        // nem persistimos migração parcial quando há conflito/ambiguidade.
        let _ = config_from_table(&migrated, config_path)?;
        backup_config_before_migration(config_path, &content)?;
        write_config_table_atomically(config_path, &migrated)?;
        table = migrated;
    }

    let mut config = config_from_table(&table, config_path)?;
    if config.security.encrypt_at_rest {
        config.security.encrypt_at_rest = false;
        config.save(config_path)?;
    }
    Ok(config)
}

/// Lê uma configuração completa a partir de um caminho explícito.
/// Não executa migração de campos ausentes.
pub fn load_config_from(path: &Path) -> Result<AppConfig> {
    let content = std::fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

fn parse_config_table(content: &str, path: &Path) -> Result<toml::map::Map<String, toml::Value>> {
    let value: toml::Value = toml::from_str(content).map_err(|e| {
        crate::error::TinaError::Config(format!(
            "TOML inválido em {}: {e}. A inicialização foi interrompida sem alterar a configuração.",
            path.display()
        ))
    })?;
    match value {
        toml::Value::Table(table) => Ok(table),
        _ => Err(crate::error::TinaError::Config(format!(
            "formato inválido em {}: esperado tabela TOML raiz",
            path.display()
        ))),
    }
}

fn config_from_table(
    table: &toml::map::Map<String, toml::Value>,
    path: &Path,
) -> Result<AppConfig> {
    let value = toml::Value::Table(table.clone());
    value.try_into().map_err(|e: toml::de::Error| {
        crate::error::TinaError::Config(format!("configuração inválida em {}: {e}", path.display()))
    })
}

fn backup_config_before_migration(path: &Path, content: &str) -> Result<PathBuf> {
    let parent = path.parent().ok_or_else(|| {
        crate::error::TinaError::Config("caminho de configuração inválido".into())
    })?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("tina.toml");
    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S%3f");
    let mut backup = parent.join(format!("{file_name}.bak.{timestamp}"));
    let mut counter = 0u32;
    while backup.exists() {
        counter += 1;
        backup = parent.join(format!("{file_name}.bak.{timestamp}.{counter}"));
    }
    write_file_atomically(&backup, content.as_bytes())?;
    Ok(backup)
}

fn write_config_table_atomically(
    path: &Path,
    table: &toml::map::Map<String, toml::Value>,
) -> Result<()> {
    let content = toml::to_string_pretty(&toml::Value::Table(table.clone()))?;
    write_file_atomically(path, content.as_bytes())
}

fn write_file_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let parent = path.parent().ok_or_else(|| {
        crate::error::TinaError::Config("caminho de configuração inválido".into())
    })?;
    std::fs::create_dir_all(parent)?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("config");
    let temp_path = parent.join(format!(
        ".{file_name}.tmp-{}-{}",
        std::process::id(),
        Uuid::new_v4()
    ));
    let mut file = std::fs::File::create(&temp_path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    std::fs::rename(&temp_path, path)?;
    Ok(())
}

async fn migrate_legacy_fields(
    table: &mut toml::map::Map<String, toml::Value>,
    missing_owner_uuid: bool,
    missing_installation_id: bool,
) -> Result<()> {
    let existing_owner_uuid = table
        .get("owner_uuid")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let existing_installation_id = table
        .get("installation_id")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    if missing_installation_id {
        let installation_id =
            resolve_installation_id_for_migration(table, existing_owner_uuid.as_deref()).await?;
        table.insert(
            "installation_id".to_string(),
            toml::Value::String(installation_id),
        );
    }

    if missing_owner_uuid {
        let owner_uuid =
            resolve_owner_uuid_for_migration(table, existing_installation_id.as_deref()).await?;
        table.insert("owner_uuid".to_string(), toml::Value::String(owner_uuid));
    }

    // Garante que campos presentes mas inválidos/vazios não passem silenciosamente.
    if let Some(id) = table.get("installation_id").and_then(|v| v.as_str()) {
        if id.trim().is_empty() {
            return Err(crate::error::TinaError::Config(
                "installation_id vazio na configuração; migração interrompida".into(),
            ));
        }
    }
    if let Some(id) = table.get("owner_uuid").and_then(|v| v.as_str()) {
        if id.trim().is_empty() {
            return Err(crate::error::TinaError::Config(
                "owner_uuid vazio na configuração; migração interrompida".into(),
            ));
        }
    }

    Ok(())
}

async fn resolve_owner_uuid_for_migration(
    table: &toml::map::Map<String, toml::Value>,
    existing_installation_id: Option<&str>,
) -> Result<String> {
    if let Some(owner) = table.get("owner_uuid").and_then(|v| v.as_str()) {
        if !owner.trim().is_empty() {
            return Ok(owner.to_string());
        }
    }

    let db_path = configured_database_path(table);
    if db_path.exists() {
        let snapshot = inspect_database_identity(&db_path).await?;
        if let Some(installation_id) = existing_installation_id.or_else(|| {
            table
                .get("installation_id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
        }) {
            if let Some(inst) = snapshot
                .installations
                .iter()
                .find(|i| i.id == installation_id)
            {
                return Ok(inst.owner_uuid.clone());
            }
            if snapshot.installations.len() == 1 {
                let inst = &snapshot.installations[0];
                // Sem dados vinculados, a instalação solitária no banco é recuperável
                // se não houver conflito com owner_uuid já conhecido.
                if !snapshot.has_linked_data() {
                    if let Some(config_owner) = table.get("owner_uuid").and_then(|v| v.as_str()) {
                        if !config_owner.trim().is_empty() && config_owner != inst.owner_uuid {
                            return Err(crate::error::TinaError::Config(format!(
                                "conflito de identidade: owner_uuid da configuração difere do proprietário da instalação no banco (instalação {})",
                                inst.id
                            )));
                        }
                    }
                    return Ok(inst.owner_uuid.clone());
                }
                return Err(crate::error::TinaError::Config(format!(
                    "conflito de identidade: installation_id da configuração não corresponde ao banco (config={}, banco={}); owner_uuid não recuperado",
                    installation_id, inst.id
                )));
            }
            if snapshot.installations.len() > 1 {
                return Err(crate::error::TinaError::Config(
                    "ambiguidade de identidade: múltiplas instalações no banco impedem recuperar owner_uuid com segurança".into(),
                ));
            }
        } else if snapshot.installations.len() == 1 {
            return Ok(snapshot.installations[0].owner_uuid.clone());
        } else if snapshot.installations.len() > 1 {
            return Err(crate::error::TinaError::Config(
                "ambiguidade de identidade: múltiplas instalações no banco impedem recuperar owner_uuid com segurança".into(),
            ));
        }
    }

    Ok(Uuid::new_v4().to_string())
}

#[derive(Debug, Clone)]
struct DbInstallationIdentity {
    id: String,
    owner_uuid: String,
}

#[derive(Debug, Default)]
struct DbIdentitySnapshot {
    installations: Vec<DbInstallationIdentity>,
    linked_data_counts: Vec<(String, i64)>,
}

impl DbIdentitySnapshot {
    fn has_linked_data(&self) -> bool {
        self.linked_data_counts.iter().any(|(_, c)| *c > 0)
    }

    fn linked_data_summary(&self) -> String {
        self.linked_data_counts
            .iter()
            .filter(|(_, c)| *c > 0)
            .map(|(t, c)| format!("{t}={c}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

async fn resolve_installation_id_for_migration(
    table: &toml::map::Map<String, toml::Value>,
    config_owner_uuid: Option<&str>,
) -> Result<String> {
    let db_path = configured_database_path(table);
    if !db_path.exists() {
        // Banco ainda não existe: gerar novo ID é seguro e idempotente após persistir config.
        return Ok(Uuid::new_v4().to_string());
    }

    let snapshot = inspect_database_identity(&db_path).await?;

    match snapshot.installations.len() {
        0 => {
            if snapshot.has_linked_data() {
                return Err(crate::error::TinaError::Config(format!(
                    "conflito de identidade: configuração sem installation_id, mas o banco contém dados vinculados sem registro de instalação ({}). Inicialização interrompida sem sobrescrever registros.",
                    snapshot.linked_data_summary()
                )));
            }
            Ok(Uuid::new_v4().to_string())
        }
        1 => {
            let inst = &snapshot.installations[0];
            if let Some(owner_uuid) = config_owner_uuid {
                if !owner_uuid.trim().is_empty() && inst.owner_uuid != owner_uuid {
                    return Err(crate::error::TinaError::Config(format!(
                        "conflito de identidade: owner_uuid da configuração difere do proprietário da instalação no banco (instalação {}). Inicialização interrompida sem sobrescrever registros.",
                        inst.id
                    )));
                }
            }
            Ok(inst.id.clone())
        }
        _ => Err(crate::error::TinaError::Config(format!(
            "ambiguidade de identidade: existem {} instalações no banco e a configuração não informa installation_id. Inicialização interrompida sem sobrescrever registros.",
            snapshot.installations.len()
        ))),
    }
}

fn configured_database_path(table: &toml::map::Map<String, toml::Value>) -> PathBuf {
    if let Some(path) = table.get("database_path").and_then(toml_value_to_pathbuf) {
        return path;
    }
    if let Some(data_dir) = table.get("data_dir").and_then(toml_value_to_pathbuf) {
        return data_dir.join("tina.db");
    }
    tina_data_dir().join("tina.db")
}

fn toml_value_to_pathbuf(value: &toml::Value) -> Option<PathBuf> {
    match value {
        toml::Value::String(s) if !s.trim().is_empty() => Some(PathBuf::from(s)),
        _ => None,
    }
}

async fn inspect_database_identity(db_path: &Path) -> Result<DbIdentitySnapshot> {
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(false)
        .foreign_keys(true);
    let pool = match sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
    {
        Ok(pool) => pool,
        Err(e) => {
            return Err(crate::error::TinaError::Config(format!(
                "não foi possível abrir o banco SQLite para migração de configuração ({}): {e}",
                db_path.display()
            )));
        }
    };

    let installations = match sqlx::query_as::<_, (String, String)>(
        "SELECT id, owner_uuid FROM installations ORDER BY created_at ASC, id ASC",
    )
    .fetch_all(&pool)
    .await
    {
        Ok(rows) => rows
            .into_iter()
            .map(|(id, owner_uuid)| DbInstallationIdentity { id, owner_uuid })
            .collect(),
        Err(e) => {
            pool.close().await;
            return Err(crate::error::TinaError::Config(format!(
                "não foi possível ler instalações do banco para migração de configuração: {e}"
            )));
        }
    };

    let mut linked_data_counts = Vec::new();
    for table in linked_data_tables() {
        let exists: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name = ? LIMIT 1",
        )
        .bind(table)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);
        if exists.is_none() {
            continue;
        }
        let query = format!("SELECT COUNT(*) FROM {table}");
        let count: i64 = sqlx::query_scalar(&query)
            .fetch_one(&pool)
            .await
            .unwrap_or(0);
        linked_data_counts.push((table.to_string(), count));
    }

    pool.close().await;
    Ok(DbIdentitySnapshot {
        installations,
        linked_data_counts,
    })
}

fn linked_data_tables() -> &'static [&'static str] {
    &[
        "owner_accounts",
        "owner_key_recovery",
        "companies",
        "departments",
        "positions",
        "employees",
        "delegations",
        "memories",
        "events",
        "tools",
        "sessions",
        "messages",
        "config",
        "chat_sessions",
        "chat_messages",
    ]
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
    use sqlx::sqlite::SqlitePoolOptions;
    use std::path::{Path, PathBuf};

    fn unique_temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tina-config-{name}-{}-{}",
            std::process::id(),
            Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn legacy_config_toml(
        data_dir: &Path,
        db_path: &Path,
        include_installation_id: bool,
        include_owner_uuid: bool,
    ) -> String {
        let mut out = String::new();
        out.push_str(&format!("data_dir = {}\n", toml_string(data_dir)));
        out.push_str(&format!("database_path = {}\n", toml_string(db_path)));
        out.push_str(&format!(
            "log_path = {}\n",
            toml_string(&data_dir.join("logs"))
        ));
        if include_installation_id {
            out.push_str("installation_id = \"legacy-installation-id\"\n");
        }
        if include_owner_uuid {
            out.push_str("owner_uuid = \"legacy-owner-uuid\"\n");
        }
        out.push_str(
            r#"
[nvidia]
base_url = "https://custom.nvidia.example/v1"
default_model = "custom/default"
reasoning_model = "custom/reasoning"
timeout_seconds = 77
max_tokens = 1234
temperature = 0.25

[security]
encrypt_at_rest = true
require_master_password = false
session_timeout_seconds = 12345
max_login_attempts = 7
lockout_duration_seconds = 321

[features]
mcp_enabled = false
local_embeddings = true
telemetry_enabled = false
beta_features = true
"#,
        );
        out
    }

    fn toml_string(path: &Path) -> String {
        toml::Value::String(path.to_string_lossy().to_string()).to_string()
    }

    async fn create_test_db(db_path: &Path) -> sqlx::SqlitePool {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).expect("create db parent");
        }
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("connect sqlite");
        sqlx::migrate!("./migration")
            .run(&pool)
            .await
            .expect("run migrations");
        pool
    }

    async fn insert_installation(pool: &sqlx::SqlitePool, id: &str, owner_uuid: &str) {
        sqlx::query(
            "INSERT INTO installations (id, name, root_path, version, owner_uuid, created_at) VALUES (?, 'Tina', ?, '0.1.0', ?, datetime('now'))",
        )
        .bind(id)
        .bind(id)
        .bind(owner_uuid)
        .execute(pool)
        .await
        .expect("insert installation");
    }

    fn assert_preferences_preserved(config: &AppConfig, data_dir: &Path, db_path: &Path) {
        assert_eq!(config.data_dir, data_dir);
        assert_eq!(config.database_path, db_path);
        assert_eq!(config.log_path, data_dir.join("logs"));
        assert_eq!(config.nvidia.base_url, "https://custom.nvidia.example/v1");
        assert_eq!(config.nvidia.default_model, "custom/default");
        assert_eq!(config.nvidia.reasoning_model, "custom/reasoning");
        assert_eq!(config.nvidia.timeout_seconds, 77);
        assert_eq!(config.nvidia.max_tokens, 1234);
        assert_eq!(config.nvidia.temperature, 0.25);
        assert!(!config.security.encrypt_at_rest); // migração normaliza default antigo
        assert!(!config.security.require_master_password);
        assert_eq!(config.security.session_timeout_seconds, 12345);
        assert_eq!(config.security.max_login_attempts, 7);
        assert_eq!(config.security.lockout_duration_seconds, 321);
        assert!(!config.features.mcp_enabled);
        assert!(config.features.local_embeddings);
        assert!(!config.features.telemetry_enabled);
        assert!(config.features.beta_features);
    }

    fn backup_files(config_path: &Path) -> Vec<PathBuf> {
        let parent = config_path.parent().expect("parent");
        let name = config_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        std::fs::read_dir(parent)
            .expect("read dir")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with(&format!("{name}.bak.")))
            })
            .collect()
    }

    #[tokio::test]
    async fn migrates_legacy_config_without_installation_id_using_real_flow() {
        let dir = unique_temp_dir("missing-installation");
        let db_path = dir.join("tina.db");
        let pool = create_test_db(&db_path).await;
        insert_installation(&pool, "inst-from-db", "legacy-owner-uuid").await;
        pool.close().await;

        let config_path = dir.join("tina.toml");
        let original = legacy_config_toml(&dir, &db_path, false, true);
        std::fs::write(&config_path, &original).expect("write legacy config");

        let migrated = load_config_at(&config_path).await.expect("migrate config");
        assert_eq!(migrated.installation_id, "inst-from-db");
        assert_eq!(migrated.owner_uuid, "legacy-owner-uuid");
        assert_preferences_preserved(&migrated, &dir, &db_path);

        let backups = backup_files(&config_path);
        assert_eq!(backups.len(), 1, "should create one backup");
        let backup_content = std::fs::read_to_string(&backups[0]).expect("read backup");
        assert_eq!(backup_content, original);

        let on_disk = std::fs::read_to_string(&config_path).expect("read migrated");
        assert!(on_disk.contains("installation_id = \"inst-from-db\""));
        assert!(on_disk.contains("owner_uuid = \"legacy-owner-uuid\""));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn migrates_legacy_config_without_owner_uuid_using_real_flow() {
        let dir = unique_temp_dir("missing-owner");
        let db_path = dir.join("tina.db");
        let pool = create_test_db(&db_path).await;
        insert_installation(&pool, "legacy-installation-id", "owner-from-db").await;
        pool.close().await;

        let config_path = dir.join("tina.toml");
        let original = legacy_config_toml(&dir, &db_path, true, false);
        std::fs::write(&config_path, &original).expect("write config");

        let migrated = load_config_at(&config_path).await.expect("migrate config");
        assert_eq!(migrated.installation_id, "legacy-installation-id");
        assert_eq!(migrated.owner_uuid, "owner-from-db");
        assert_preferences_preserved(&migrated, &dir, &db_path);

        let backups = backup_files(&config_path);
        assert_eq!(backups.len(), 1);
        assert_eq!(
            std::fs::read_to_string(&backups[0]).expect("read backup"),
            original
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn recovers_existing_identity_from_sqlite() {
        let dir = unique_temp_dir("recover-sqlite");
        let db_path = dir.join("tina.db");
        let pool = create_test_db(&db_path).await;
        insert_installation(&pool, "db-installation", "db-owner").await;
        pool.close().await;

        let config_path = dir.join("tina.toml");
        // Sem installation_id e sem owner_uuid, mas com DB contendo identidade.
        std::fs::write(
            &config_path,
            legacy_config_toml(&dir, &db_path, false, false),
        )
        .expect("write config");

        let migrated = load_config_at(&config_path).await.expect("migrate config");
        assert_eq!(migrated.installation_id, "db-installation");
        assert_eq!(migrated.owner_uuid, "db-owner");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn generates_installation_id_when_db_has_no_installations_and_no_linked_data() {
        let dir = unique_temp_dir("empty-db");
        let db_path = dir.join("tina.db");
        let pool = create_test_db(&db_path).await;
        pool.close().await; // schema existe, mas sem instalações/dados vinculados

        let config_path = dir.join("tina.toml");
        std::fs::write(
            &config_path,
            legacy_config_toml(&dir, &db_path, false, true),
        )
        .expect("write config");

        let migrated = load_config_at(&config_path).await.expect("migrate config");
        assert!(!migrated.installation_id.trim().is_empty());
        assert_ne!(migrated.installation_id, "legacy-installation-id");
        assert_eq!(migrated.owner_uuid, "legacy-owner-uuid");
        assert_preferences_preserved(&migrated, &dir, &db_path);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn fails_on_identity_conflict_without_overwriting_records() {
        let dir = unique_temp_dir("conflict");
        let db_path = dir.join("tina.db");
        let pool = create_test_db(&db_path).await;
        insert_installation(&pool, "inst-db", "owner-db").await;
        pool.close().await;

        let config_path = dir.join("tina.toml");
        let mut original = legacy_config_toml(&dir, &db_path, false, true);
        original = original.replace(
            "owner_uuid = \"legacy-owner-uuid\"",
            "owner_uuid = \"other-owner\"",
        );
        std::fs::write(&config_path, &original).expect("write config");

        let err = load_config_at(&config_path)
            .await
            .expect_err("should conflict");
        let msg = err.to_string();
        assert!(
            msg.contains("conflito de identidade"),
            "unexpected error: {msg}"
        );

        let after = std::fs::read_to_string(&config_path).expect("read config");
        assert_eq!(
            after, original,
            "config must not be overwritten on conflict"
        );
        assert!(
            backup_files(&config_path).is_empty(),
            "no migration backup on pre-validation conflict"
        );

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(&db_path)
                    .create_if_missing(false),
            )
            .await
            .expect("open db");
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM installations")
            .fetch_one(&pool)
            .await
            .expect("count installations");
        assert_eq!(count, 1);
        pool.close().await;

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn fails_on_invalid_toml_without_changes() {
        let dir = unique_temp_dir("invalid-toml");
        let config_path = dir.join("tina.toml");
        let original = "data_dir = \"C:\\\\bad\"\ninstallation_id = \"x\"\n[broken\n";
        std::fs::write(&config_path, original).expect("write invalid config");

        let err = load_config_at(&config_path).await.expect_err("should fail");
        let msg = err.to_string();
        assert!(msg.contains("TOML inválido"), "unexpected error: {msg}");
        let after = std::fs::read_to_string(&config_path).expect("read config");
        assert_eq!(after, original);
        assert!(backup_files(&config_path).is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn restart_after_migration_is_idempotent() {
        let dir = unique_temp_dir("idempotent");
        let db_path = dir.join("tina.db");
        let pool = create_test_db(&db_path).await;
        insert_installation(&pool, "inst-stable", "owner-stable").await;
        pool.close().await;

        let config_path = dir.join("tina.toml");
        std::fs::write(
            &config_path,
            legacy_config_toml(&dir, &db_path, false, false),
        )
        .expect("write config");

        let first = load_config_at(&config_path).await.expect("first load");
        let after_first = std::fs::read_to_string(&config_path).expect("read");
        let backups_after_first = backup_files(&config_path).len();

        let second = load_config_at(&config_path).await.expect("second load");
        let after_second = std::fs::read_to_string(&config_path).expect("read");
        let backups_after_second = backup_files(&config_path).len();

        assert_eq!(first.installation_id, second.installation_id);
        assert_eq!(first.owner_uuid, second.owner_uuid);
        assert_eq!(second.installation_id, "inst-stable");
        assert_eq!(second.owner_uuid, "owner-stable");
        assert_eq!(after_first, after_second, "config file must be stable");
        assert_eq!(backups_after_first, 1);
        assert_eq!(
            backups_after_second, 1,
            "no new backup on idempotent restart"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn preserves_existing_settings_during_migration() {
        let dir = unique_temp_dir("preserve-settings");
        let db_path = dir.join("tina.db");
        let pool = create_test_db(&db_path).await;
        insert_installation(&pool, "inst-preserve", "legacy-owner-uuid").await;
        pool.close().await;

        let config_path = dir.join("tina.toml");
        std::fs::write(
            &config_path,
            legacy_config_toml(&dir, &db_path, false, true),
        )
        .expect("write config");

        let migrated = load_config_at(&config_path).await.expect("migrate");
        assert_preferences_preserved(&migrated, &dir, &db_path);
        assert_eq!(migrated.owner_uuid, "legacy-owner-uuid");
        assert_eq!(migrated.installation_id, "inst-preserve");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn migration_with_missing_owner_uuid_preserves_installation_id() {
        let dir = unique_temp_dir("preserve-installation-id");
        let db_path = dir.join("tina.db");
        let pool = create_test_db(&db_path).await;
        insert_installation(&pool, "inst-keep", "owner-db").await;
        pool.close().await;

        let config_path = dir.join("tina.toml");
        let original = legacy_config_toml(&dir, &db_path, true, false).replace(
            "installation_id = \"legacy-installation-id\"",
            "installation_id = \"inst-keep\"",
        );
        std::fs::write(&config_path, &original).expect("write config");

        let migrated = load_config_at(&config_path).await.expect("migrate");
        assert_eq!(migrated.installation_id, "inst-keep");
        assert_eq!(migrated.owner_uuid, "owner-db");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
