//! Tina Platform Ã¢â‚¬â€ inteligÃƒÂªncia pessoal e operacional.
//!
//! Este crate ÃƒÂ© o **nÃƒÂºcleo** da aplicaÃƒÂ§ÃƒÂ£o desktop. Ele nÃƒÂ£o contÃƒÂ©m entry point:
//! o executÃƒÂ¡vel ÃƒÂ© `src-tauri/src/main.rs`, que o utiliza como biblioteca.
//!
//! Nesta fase do projeto os mÃƒÂ³dulos necessÃƒÂ¡rios ao bootstrap desktop
//! sÃƒÂ£o declarados: configuraÃƒÂ§ÃƒÂ£o, erros, banco SQLite e instalaÃƒÂ§ÃƒÂ£o.

#![deny(missing_docs)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};

use tokio::sync::watch;

use crate::config::AppConfig;
use crate::core::authorization::{
    Actor, ActorContext, AuthorizationPolicy, Operation, ResourceScope,
};
use crate::core::owner_auth::{OwnerAccess, OwnerAccountStatus};
use crate::error::Result;
use crate::orchestration::plan::PlanCard;

pub mod config;
pub mod core;
pub mod error;

pub mod infra;
/// Motor central de planejamento, autorização e execução de ferramentas.
pub mod orchestration;

#[cfg(test)]
mod tests;

pub use error::{Result as TinaResult, TinaError};
pub use infra::Database;

/// VersÃƒÂ£o da plataforma, obtida do manifesto Cargo.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Resultado serializado de uma decisão sobre plano de ferramentas.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanDecision {
    /// Identificador imutável do plano decidido.
    pub plan_id: String,
    /// Estado persistido após a decisão ou execução.
    pub state: String,
    /// Cartão ainda ativo, quando houver.
    pub card: Option<PlanCard>,
    /// Etapas concluídas nesta chamada.
    pub executed: usize,
    /// Identificadores das etapas que falharam.
    pub failed: Vec<String>,
    /// Identificadores das etapas bloqueadas por dependência.
    pub skipped: Vec<String>,
    /// Mensagem assimilada ao histórico visível do chat.
    pub message: infra::chat::ChatMessage,
}

/// Estado inicial da Tina, compartilhado com os comandos Tauri via `State`.
#[derive(Debug)]
pub struct TinaApp {
    config: Arc<AppConfig>,
    database: Database,
    owner_access: Arc<OwnerAccess>,
    chat_generation: Arc<tokio::sync::Mutex<()>>,
    /// Sinais de cancelamento por plano, compartilhados com os comandos Tauri.
    plan_cancellation: Arc<StdMutex<HashMap<String, watch::Sender<bool>>>>,
    #[cfg(test)]
    test_nvidia_key: StdMutex<Option<String>>,
}

/// Acesso temporÃƒÂ¡rio aos repositÃƒÂ³rios da instalaÃƒÂ§ÃƒÂ£o, entregue somente apÃƒÂ³s
/// autenticaÃƒÂ§ÃƒÂ£o do proprietÃƒÂ¡rio. NÃƒÂ£o armazene este escopo entre operaÃƒÂ§ÃƒÂµes.
#[derive(Debug)]
pub struct AuthorizedOwnerDatabase<'a> {
    database: &'a Database,
}

impl<'a> AuthorizedOwnerDatabase<'a> {
    /// Pool SQLite da instalaÃƒÂ§ÃƒÂ£o para uma operaÃƒÂ§ÃƒÂ£o previamente autorizada.
    /// Os repositÃƒÂ³rios nÃƒÂ£o devem receber pools diretamente de entradas IPC.
    pub fn pool(&self) -> &'a sqlx::SqlitePool {
        self.database.pool()
    }

    /// RepositÃƒÂ³rio de empresas disponÃƒÂ­vel somente apÃƒÂ³s a autorizaÃƒÂ§ÃƒÂ£o do escopo.
    pub fn companies(&self) -> infra::CompanyRepository<'a> {
        infra::CompanyRepository::new(self.database.pool())
    }

    /// RepositÃƒÂ³rio de departamentos disponÃƒÂ­vel somente apÃƒÂ³s a autorizaÃƒÂ§ÃƒÂ£o do escopo.
    pub fn departments(&self) -> infra::DepartmentRepository<'a> {
        infra::DepartmentRepository::new(self.database.pool())
    }

    /// RepositÃƒÂ³rio de cargos disponÃƒÂ­vel somente apÃƒÂ³s a autorizaÃƒÂ§ÃƒÂ£o do escopo.
    pub fn positions(&self) -> infra::PositionRepository<'a> {
        infra::PositionRepository::new(self.database.pool())
    }

    /// RepositÃƒÂ³rio de funcionÃƒÂ¡rios disponÃƒÂ­vel somente apÃƒÂ³s a autorizaÃƒÂ§ÃƒÂ£o do escopo.
    pub fn employees(&self) -> infra::EmployeeRepository<'a> {
        infra::EmployeeRepository::new(self.database.pool())
    }
}

impl TinaApp {
    /// Carrega configuraÃƒÂ§ÃƒÂ£o, garante os diretÃƒÂ³rios locais, abre/cria o SQLite
    /// aplica as migrations e garante que a instalaÃƒÂ§ÃƒÂ£o exista no banco.
    pub async fn initialize() -> Result<Self> {
        let config = Arc::new(config::load_config().await?);
        config.ensure_directories()?;

        let database = Database::open(&config.database_path).await?;

        // Garantir que a instalaÃƒÂ§ÃƒÂ£o exista no banco (inicializaÃƒÂ§ÃƒÂ£o idempotente)
        let repo = infra::InstallationRepository::new(database.pool());

        // Verificar se a instalaÃƒÂ§ÃƒÂ£o do config existe no banco
        // O ID da instalaÃƒÂ§ÃƒÂ£o ÃƒÂ© o principal-chave (identificador da instalaÃƒÂ§ÃƒÂ£o como um todo)
        // O owner_uuid ÃƒÂ© o ID do proprietÃƒÂ¡rio que criou a instalaÃƒÂ§ÃƒÂ£o
        if let Some(existing_installation) = repo.find_by_id(&config.installation_id).await? {
            if existing_installation.owner_uuid != config.owner_uuid {
                return Err(crate::error::TinaError::Other(
                    "A identidade do proprietÃƒÂ¡rio na configuraÃƒÂ§ÃƒÂ£o nÃƒÂ£o corresponde ao registro persistido da instalaÃƒÂ§ÃƒÂ£o. A inicializaÃƒÂ§ÃƒÂ£o foi interrompida para proteger a identidade local.".to_string(),
                )
                .into());
            }
        } else {
            // InstalaÃƒÂ§ÃƒÂ£o nÃƒÂ£o existe - criar com o ID e proprietÃƒÂ¡rio da configuraÃƒÂ§ÃƒÂ£o.
            repo.create_with_id("Tina", &config.installation_id, &config.owner_uuid)
                .await?;
        }

        Ok(Self {
            config,
            database,
            owner_access: Arc::new(OwnerAccess::default()),
            chat_generation: Arc::new(tokio::sync::Mutex::new(())),
            plan_cancellation: Arc::new(StdMutex::new(HashMap::new())),
            #[cfg(test)]
            test_nvidia_key: StdMutex::new(None),
        })
    }

    /// Initialize TinaApp at a specific path (for testing).
    /// This allows isolated testing with temporary databases.
    pub async fn initialize_at(
        database_path: &std::path::Path,
        installation_id: &str,
        owner_uuid: &str,
    ) -> Result<Self> {
        let data_dir = database_path
            .parent()
            .expect("Database path must have parent directory")
            .to_path_buf();

        let config = Arc::new(AppConfig {
            data_dir: data_dir.clone(),
            database_path: database_path.to_path_buf(),
            log_path: data_dir.join("logs"),
            installation_id: installation_id.to_string(),
            owner_uuid: owner_uuid.to_string(),
            nvidia: config::NvidiaConfig::default(),
            security: config::SecurityConfig::default(),
            features: config::FeatureFlags::default(),
        });

        config.ensure_directories()?;

        let database = Database::open(&config.database_path).await?;

        // Garantir que a instalaÃƒÂ§ÃƒÂ£o exista no banco (inicializaÃƒÂ§ÃƒÂ£o idempotente)
        let repo = infra::InstallationRepository::new(database.pool());

        // Verificar se a instalaÃƒÂ§ÃƒÂ£o do config existe no banco
        if let Some(existing_installation) = repo.find_by_id(&config.installation_id).await? {
            if existing_installation.owner_uuid != config.owner_uuid {
                return Err(crate::error::TinaError::Other(
                    "A identidade do proprietÃƒÂ¡rio na configuraÃƒÂ§ÃƒÂ£o nÃƒÂ£o corresponde ao registro persistido da instalaÃƒÂ§ÃƒÂ£o. A inicializaÃƒÂ§ÃƒÂ£o foi interrompida para proteger a identidade local.".to_string(),
                )
                .into());
            }
        } else {
            // InstalaÃƒÂ§ÃƒÂ£o nÃƒÂ£o existe - criar com o ID e proprietÃƒÂ¡rio da configuraÃƒÂ§ÃƒÂ£o.
            repo.create_with_id("Tina", &config.installation_id, &config.owner_uuid)
                .await?;
        }

        Ok(Self {
            config,
            database,
            owner_access: Arc::new(OwnerAccess::default()),
            chat_generation: Arc::new(tokio::sync::Mutex::new(())),
            plan_cancellation: Arc::new(StdMutex::new(HashMap::new())),
            #[cfg(test)]
            test_nvidia_key: StdMutex::new(None),
        })
    }

    /// Consulta se existe cadastro inicial e se a sessão está desbloqueada.
    pub async fn owner_status(&self) -> Result<OwnerAccountStatus> {
        self.owner_access
            .status(self.database.pool(), &self.config.installation_id)
            .await
    }

    /// Cadastra conta inicial e gera a chave no cofre do Windows.
    pub async fn create_owner_account(&self, name: &str, password: &str) -> Result<()> {
        self.owner_access
            .create_account(
                self.database.pool(),
                &self.config.installation_id,
                &self.config.owner_uuid,
                name,
                password,
            )
            .await
    }

    /// Autentica o proprietÃƒÂ¡rio por senha e verifica a chave protegida no vault.
    pub async fn sign_in_owner(&self, password: &str) -> Result<()> {
        self.owner_access
            .sign_in(
                self.database.pool(),
                &self.config.installation_id,
                password,
                self.config.security.session_timeout_seconds,
                self.config.security.max_login_attempts,
                self.config.security.lockout_duration_seconds,
            )
            .await
    }

    /// Encerra a sessÃƒÂ£o autenticada.
    pub fn lock_owner(&self) -> Result<()> {
        self.owner_access.lock()
    }

    /// Verifica a sessÃƒÂ£o de proprietÃƒÂ¡rio, sem expor o segredo.
    pub fn owner_unlocked(&self) -> Result<bool> {
        self.owner_access.is_unlocked()
    }

    /// Exige sessÃƒÂ£o de proprietÃƒÂ¡rio para toda aÃƒÂ§ÃƒÂ£o administrativa.
    pub fn require_owner(&self, operation: Operation) -> Result<()> {
        if !self.owner_access.is_unlocked()? {
            return Err(TinaError::Unauthorized(
                "sessÃƒÂ£o do proprietÃƒÂ¡rio bloqueada".into(),
            ));
        }
        let context = ActorContext::verified(self.config.installation_id.clone(), Actor::Owner);
        AuthorizationPolicy::authorize(
            &context,
            operation,
            ResourceScope {
                installation_id: &self.config.installation_id,
                company_id: None,
            },
        )
    }

    /// Obtém (ou cria) o sinal de cancelamento persistente de um plano.
    fn plan_cancel_signal(&self, plan_id: &str) -> watch::Sender<bool> {
        let mut signals = self.plan_cancellation.lock().unwrap();
        signals
            .entry(plan_id.to_owned())
            .or_insert_with(|| watch::channel(false).0)
            .clone()
    }

    /// Configuração carregada da instalação.
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    /// Abre acesso ao banco da instalaÃƒÂ§ÃƒÂ£o somente apÃƒÂ³s autorizaÃƒÂ§ÃƒÂ£o do
    /// proprietÃƒÂ¡rio no backend. Os comandos Tauri usam apenas esta API.
    pub fn owner_database(&self, operation: Operation) -> Result<AuthorizedOwnerDatabase<'_>> {
        self.require_owner(operation)?;
        Ok(AuthorizedOwnerDatabase {
            database: &self.database,
        })
    }

    /// Consulta o diretório de pessoal da instalação autenticada, somente leitura.
    pub async fn owner_employee_directory(&self) -> Result<Vec<infra::EmployeeDirectoryEntry>> {
        let db = self.owner_database(Operation::Read)?;
        let entries = db
            .employees()
            .list_directory(&self.config.installation_id)
            .await?;
        self.require_owner(Operation::Read)?;
        Ok(entries)
    }

    /// Lista aplicativos e ferramentas registradas, sem anunciar implementações futuras.
    pub fn owner_tool_catalog(&self) -> Result<Vec<infra::tool_access::ToolEntry>> {
        self.require_owner(Operation::Read)?;
        Ok(infra::tool_access::catalog())
    }

    /// Consulta as atribuições de ferramentas desta instalação.
    pub async fn owner_tool_grants(&self) -> Result<Vec<infra::tool_access::ToolGrant>> {
        self.require_owner(Operation::Read)?;
        let rows =
            infra::tool_access::list(self.database.pool(), &self.config.installation_id).await?;
        self.require_owner(Operation::Read)?;
        Ok(rows)
    }

    /// Concede ou revoga uma ferramenta ou um aplicativo a um funcionário autenticado na instalação.
    pub async fn owner_change_tool_access(
        &self,
        employee_id: &str,
        action: &str,
        key: &str,
    ) -> Result<()> {
        self.require_owner(Operation::ManagePermissions)?;
        infra::tool_access::change(
            self.database.pool(),
            &self.config.installation_id,
            &self.config.owner_uuid,
            employee_id,
            action,
            key,
        )
        .await
    }

    /// Armazena a chave NVIDIA no cofre nativo apÃƒÂ³s autorizaÃƒÂ§ÃƒÂ£o.
    pub fn owner_set_nvidia_key(&self, key: &str) -> Result<()> {
        self.require_owner(Operation::Write)?;
        if key.is_empty() || key.len() > 4096 || key.chars().any(char::is_whitespace) {
            return Err(TinaError::Other("chave NVIDIA invÃƒÂ¡lida".into()));
        }
        #[cfg(windows)]
        {
            let entry = keyring::Entry::new("Tina.NVIDIA", &self.config.installation_id)
                .map_err(|_| TinaError::Other("cofre NVIDIA indisponÃƒÂ­vel".into()))?;
            entry
                .set_password(key)
                .map_err(|_| TinaError::Other("falha ao salvar chave NVIDIA".into()))
        }
        #[cfg(not(windows))]
        Err(TinaError::Other("cofre NVIDIA requer Windows".into()))
    }

    /// Verifica a existÃƒÂªncia da credencial sem retornar seu conteÃƒÂºdo.
    pub fn owner_has_nvidia_key(&self) -> Result<bool> {
        self.require_owner(Operation::Read)?;
        #[cfg(windows)]
        {
            let entry = keyring::Entry::new("Tina.NVIDIA", &self.config.installation_id)
                .map_err(|_| TinaError::Other("cofre NVIDIA indisponÃƒÂ­vel".into()))?;
            match entry.get_password() {
                Ok(key) => Ok(!key.is_empty()),
                Err(keyring::Error::NoEntry) => Ok(false),
                Err(_) => Err(TinaError::Other("falha ao consultar cofre NVIDIA".into())),
            }
        }
        #[cfg(not(windows))]
        Err(TinaError::Other("cofre NVIDIA requer Windows".into()))
    }

    /// Remove a credencial NVIDIA da instalaÃƒÂ§ÃƒÂ£o autenticada.
    pub fn owner_delete_nvidia_key(&self) -> Result<()> {
        self.require_owner(Operation::Write)?;
        #[cfg(windows)]
        {
            let entry = keyring::Entry::new("Tina.NVIDIA", &self.config.installation_id)
                .map_err(|_| TinaError::Other("cofre NVIDIA indisponÃƒÂ­vel".into()))?;
            match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(_) => Err(TinaError::Other("falha ao remover chave NVIDIA".into())),
            }
        }
        #[cfg(not(windows))]
        Err(TinaError::Other("cofre NVIDIA requer Windows".into()))
    }

    /// Cria uma conversa principal apÃƒÂ³s validar a sessÃƒÂ£o do proprietÃƒÂ¡rio.
    pub async fn owner_create_chat(&self, title: &str) -> Result<infra::chat::ChatSession> {
        self.require_owner(Operation::Write)?;
        infra::chat::create_session(self.database.pool(), &self.config.installation_id, title).await
    }

    /// Cria ou recupera uma conversa com identificador estÃƒÂ¡vel de tentativa.
    pub async fn owner_create_chat_with_id(
        &self,
        id: &str,
        title: &str,
    ) -> Result<infra::chat::ChatSession> {
        self.require_owner(Operation::Write)?;
        infra::chat::create_session_with_id(
            self.database.pool(),
            &self.config.installation_id,
            id,
            title,
        )
        .await
    }

    /// Renomeia uma conversa da instalaÃƒÂ§ÃƒÂ£o apÃƒÂ³s autorizaÃƒÂ§ÃƒÂ£o do proprietÃƒÂ¡rio.
    pub async fn owner_rename_chat(
        &self,
        session_id: &str,
        title: &str,
    ) -> Result<infra::chat::ChatSession> {
        self.require_owner(Operation::Write)?;
        infra::chat::rename_session(
            self.database.pool(),
            &self.config.installation_id,
            session_id,
            title,
        )
        .await
    }

    /// Lista conversas apenas da instalaÃƒÂ§ÃƒÂ£o autenticada.
    pub async fn owner_list_chats(&self) -> Result<Vec<infra::chat::ChatSession>> {
        self.require_owner(Operation::Read)?;
        infra::chat::list_sessions(self.database.pool(), &self.config.installation_id).await
    }

    /// Consulta mensagens de uma conversa apÃƒÂ³s validaÃƒÂ§ÃƒÂ£o de escopo.
    pub async fn owner_chat_messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<infra::chat::ChatMessage>> {
        self.require_owner(Operation::Read)?;
        infra::chat::list_messages(
            self.database.pool(),
            &self.config.installation_id,
            session_id,
        )
        .await
    }

    /// Persiste mensagem textual apÃƒÂ³s validar sessÃƒÂ£o e instalaÃƒÂ§ÃƒÂ£o.
    /// O backend de geraÃƒÂ§ÃƒÂ£o NVIDIA deverÃƒÂ¡ usar uma API interna distinta.
    pub async fn owner_append_chat_message(
        &self,
        session_id: &str,
        content: &str,
    ) -> Result<infra::chat::ChatMessage> {
        self.require_owner(Operation::Write)?;
        infra::chat::append_message(
            self.database.pool(),
            &self.config.installation_id,
            session_id,
            "user",
            content,
        )
        .await
    }

    /// Persiste uma mensagem de usuÃƒÂ¡rio idempotente por identificador de tentativa.
    pub async fn owner_append_chat_with_id(
        &self,
        session_id: &str,
        message_id: &str,
        content: &str,
    ) -> Result<infra::chat::ChatMessage> {
        self.require_owner(Operation::Write)?;
        let _guard = self
            .chat_generation
            .try_lock()
            .map_err(|_| TinaError::Other("jÃƒÂ¡ existe uma geraÃƒÂ§ÃƒÂ£o ativa".into()))?;
        infra::chat::append_message_with_id(
            self.database.pool(),
            &self.config.installation_id,
            session_id,
            message_id,
            "user",
            content,
        )
        .await
    }

    /// Limites de chat expostos ao frontend para evitar diverg?ncia.
    pub fn chat_limits(&self) -> infra::chat::ChatLimits {
        infra::chat::ChatLimits {
            max_user_message_bytes: infra::chat::MAX_USER_MESSAGE_BYTES,
        }
    }

    /// Gera resposta no backend, usando exclusivamente o modelo configurado da Tina.
    /// Uma gera??o por instala??o; respostas incompletas n?o entram no hist?rico.
    pub async fn owner_generate_chat(
        &self,
        session_id: &str,
        cancelled: tokio::sync::watch::Receiver<bool>,
        on_delta: impl FnMut(&str) -> Result<()>,
    ) -> Result<infra::chat::ChatMessage> {
        self.owner_generate_chat_with_base(
            session_id,
            &self.config.nvidia.base_url,
            std::time::Duration::from_secs(180),
            cancelled,
            on_delta,
        )
        .await
    }

    /// Variante interna para testes com transporte mock.
    #[cfg(test)]
    pub(crate) async fn owner_generate_chat_for_test(
        &self,
        session_id: &str,
        base_url: &str,
        response_timeout: std::time::Duration,
        cancelled: tokio::sync::watch::Receiver<bool>,
        on_delta: impl FnMut(&str) -> Result<()>,
    ) -> Result<infra::chat::ChatMessage> {
        self.owner_generate_chat_with_base(
            session_id,
            base_url,
            response_timeout,
            cancelled,
            on_delta,
        )
        .await
    }

    async fn owner_generate_chat_with_base(
        &self,
        session_id: &str,
        base_url: &str,
        response_timeout: std::time::Duration,
        cancelled: tokio::sync::watch::Receiver<bool>,
        mut on_delta: impl FnMut(&str) -> Result<()>,
    ) -> Result<infra::chat::ChatMessage> {
        self.require_owner(Operation::Write)?;
        let _guard = self
            .chat_generation
            .try_lock()
            .map_err(|_| TinaError::Other("jÃƒÂ¡ existe uma geraÃƒÂ§ÃƒÂ£o ativa".into()))?;
        self.require_owner(Operation::Write)?;
        #[cfg(not(windows))]
        {
            let _ = (session_id, cancelled, &mut on_delta);
            Err(TinaError::Other("cofre NVIDIA requer Windows".into()))
        }
        #[cfg(windows)]
        {
            let session_marker = self
                .owner_access
                .session_marker()?
                .ok_or_else(|| TinaError::Unauthorized("sessÃƒÂ£o bloqueada".into()))?;
            let check_session = || {
                if self.owner_access.session_marker()? != Some(session_marker) {
                    return Err(TinaError::Unauthorized(
                        "sessÃƒÂ£o alterada ou expirada".into(),
                    ));
                }
                Ok(())
            };
            let settings = &self.config.nvidia;
            infra::chat::nvidia_endpoint(base_url)?;
            if settings.max_tokens == 0 || settings.max_tokens > 4096 {
                return Err(TinaError::Other(
                    "limite de saÃƒÂ­da ainda nÃƒÂ£o validado para o modelo".into(),
                ));
            }
            let messages = self.owner_chat_messages(session_id).await?;
            let history_budget = infra::chat::CHAT_CONTEXT_BYTES
                .saturating_sub(infra::chat::SYSTEM_PROMPT_RESERVE_BYTES);
            let context = infra::chat::context_window(&messages, history_budget);
            if context.last().map(|m| m.role.as_str()) != Some("user") {
                return Err(TinaError::Other(
                    "nenhuma mensagem de usuÃƒÂ¡rio disponÃƒÂ­vel no contexto".into(),
                ));
            }
            let key = {
                #[cfg(test)]
                {
                    if let Some(k) = self.test_nvidia_key.lock().unwrap().clone() {
                        k
                    } else {
                        let entry =
                            keyring::Entry::new("Tina.NVIDIA", &self.config.installation_id)
                                .map_err(|_| {
                                    TinaError::Other("cofre NVIDIA indisponÃ­vel".into())
                                })?;
                        entry
                            .get_password()
                            .map_err(|_| TinaError::Other("configure a chave NVIDIA".into()))?
                    }
                }
                #[cfg(not(test))]
                {
                    let entry = keyring::Entry::new("Tina.NVIDIA", &self.config.installation_id)
                        .map_err(|_| TinaError::Other("cofre NVIDIA indisponÃ­vel".into()))?;
                    entry
                        .get_password()
                        .map_err(|_| TinaError::Other("configure a chave NVIDIA".into()))?
                }
            };
            self.require_owner(Operation::Write)?;
            check_session()?;
            let answer = tokio::select! {
                result = infra::chat::stream_nvidia_turn(
                    base_url,
                    &key,
                    &settings.default_model,
                    context,
                    settings.max_tokens,
                    response_timeout,
                    cancelled.clone(),
                    true,
                    |delta| {
                        check_session()?;
                        on_delta(delta)
                    },
                ) => result?,
                result = async {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                        if let Err(error) = check_session() {
                            break Err::<infra::chat::NvidiaTurn, _>(error);
                        }
                    }
                } => result?,
            };
            check_session()?;
            self.require_owner(Operation::Write)?;
            if *cancelled.borrow() {
                return Err(TinaError::Other("geraÃƒÂ§ÃƒÂ£o cancelada".into()));
            }
            match answer {
                infra::chat::NvidiaTurn::Text(answer) => {
                    infra::chat::append_message(
                        self.database.pool(),
                        &self.config.installation_id,
                        session_id,
                        "assistant",
                        &answer,
                    )
                    .await
                }
                infra::chat::NvidiaTurn::Tools(calls) => {
                    let active = orchestration::store::load_active_plan(
                        self.database.pool(),
                        &self.config.installation_id,
                        session_id,
                    )
                    .await?;
                    if active.is_some() {
                        return Err(TinaError::ToolError(
                            "já existe um plano ativo; aprove, recuse, cancele ou substitua o plano atual antes de continuar".into(),
                        ));
                    }
                    let user = context
                        .last()
                        .ok_or_else(|| TinaError::ToolError("mensagem ausente".into()))?;
                    if calls.len() == 1 {
                        let call = &calls[0];
                        return infra::chat_tools::propose(
                            self.database.pool(),
                            &self.config.installation_id,
                            session_id,
                            &user.id,
                            &call.name,
                            &call.arguments,
                        )
                        .await;
                    }
                    let prepared = orchestration::plan::prepare(
                        self.database.pool(),
                        &self.config.installation_id,
                        &calls,
                        orchestration::plan::PlanLimits::default(),
                    )
                    .await?;
                    let plan = orchestration::store::create_plan(
                        self.database.pool(),
                        &self.config.installation_id,
                        session_id,
                        &user.id,
                        &prepared,
                        orchestration::plan::PlanLimits::default(),
                    )
                    .await?;
                    orchestration::history::append_tool_call_message(
                        self.database.pool(),
                        &self.config.installation_id,
                        session_id,
                        &calls,
                    )
                    .await?;
                    let summary = format!(
                        "Organizei {} ações dependentes em um plano. Revise as etapas, justificativas e efeitos antes de autorizar.",
                        plan.steps_count
                    );
                    infra::chat::append_message(
                        self.database.pool(),
                        &self.config.installation_id,
                        session_id,
                        "assistant",
                        &summary,
                    )
                    .await
                }
            }
        }
    }

    /// Consulta a proposta ativa da sessão, independente da última mensagem.
    pub async fn owner_pending_department(
        &self,
        session_id: &str,
    ) -> Result<Option<infra::chat_tools::DepartmentProposal>> {
        self.require_owner(Operation::Read)?;
        infra::chat_tools::pending(
            self.database.pool(),
            &self.config.installation_id,
            session_id,
        )
        .await
    }

    /// Aprova ou recusa uma proposta persistida, com a identidade autenticada do proprietário.
    pub async fn owner_decide_department(
        &self,
        session_id: &str,
        proposal_id: &str,
        approved: bool,
    ) -> Result<infra::chat::ChatMessage> {
        self.require_owner(Operation::Write)?;
        let _guard = self
            .chat_generation
            .try_lock()
            .map_err(|_| TinaError::Other("há uma geração ativa".into()))?;
        self.require_owner(Operation::Write)?;
        infra::chat_tools::decide(
            self.database.pool(),
            &self.config.installation_id,
            &self.config.owner_uuid,
            session_id,
            proposal_id,
            approved,
        )
        .await
    }

    /// Cancela uma proposta ativa sem criar nada; idempotente para o mesmo id.
    pub async fn owner_cancel_proposal(
        &self,
        session_id: &str,
        proposal_id: &str,
    ) -> Result<infra::chat::ChatMessage> {
        self.require_owner(Operation::Write)?;
        let _guard = self
            .chat_generation
            .try_lock()
            .map_err(|_| TinaError::Other("há uma geração ativa".into()))?;
        self.require_owner(Operation::Write)?;
        infra::chat_tools::cancel(
            self.database.pool(),
            &self.config.installation_id,
            &self.config.owner_uuid,
            session_id,
            proposal_id,
        )
        .await
    }

    /// Consulta o cartão do plano ativo da sessão, com ações e dependências.
    pub async fn owner_pending_plan(&self, session_id: &str) -> Result<Option<PlanCard>> {
        self.require_owner(Operation::Read)?;
        orchestration::store::load_card(
            self.database.pool(),
            &self.config.installation_id,
            session_id,
        )
        .await
    }

    /// Aprova ou recusa a versão imutável de um plano de ferramentas.
    pub async fn owner_decide_plan(
        &self,
        session_id: &str,
        plan_id: &str,
        fingerprint: &str,
        approved: bool,
    ) -> Result<PlanDecision> {
        self.require_owner(Operation::Write)?;
        let _guard = self
            .chat_generation
            .try_lock()
            .map_err(|_| TinaError::Other("há uma geração ativa".into()))?;
        self.require_owner(Operation::Write)?;
        let marker = self
            .owner_access
            .session_marker()?
            .ok_or_else(|| TinaError::Unauthorized("sessão bloqueada".into()))?;

        let plan = orchestration::store::authorize_plan(
            self.database.pool(),
            &self.config.installation_id,
            &self.config.owner_uuid,
            session_id,
            plan_id,
            fingerprint,
            approved,
        )
        .await?;
        self.assert_plan_session(marker)?;

        if !approved {
            let message = infra::chat::append_message(
                self.database.pool(),
                &self.config.installation_id,
                session_id,
                "assistant",
                "Plano recusado. Nenhuma ação foi executada e nenhum efeito foi gravado.",
            )
            .await?;
            return Ok(PlanDecision {
                plan_id: plan.id,
                state: plan.state,
                card: None,
                executed: 0,
                failed: Vec::new(),
                skipped: Vec::new(),
                message,
            });
        }

        let authorized = plan
            .authorized_fingerprint
            .as_deref()
            .unwrap_or(&plan.fingerprint)
            .to_owned();
        let cancelled = self.plan_cancel_signal(plan_id).subscribe();
        let summary = self
            .execute_plan(session_id, plan_id, &authorized, marker, cancelled)
            .await?;
        let message = self.append_plan_status(session_id, &summary).await?;
        Ok(PlanDecision {
            plan_id: summary.plan.id,
            state: summary.plan.state,
            card: None,
            executed: summary.executed,
            failed: summary.failed,
            skipped: summary.skipped,
            message,
        })
    }

    /// Retoma uma aprovação interrompida sem pedir nova autorização.
    pub async fn owner_resume_plan(&self, session_id: &str, plan_id: &str) -> Result<PlanDecision> {
        self.require_owner(Operation::Write)?;
        let _guard = self
            .chat_generation
            .try_lock()
            .map_err(|_| TinaError::Other("há uma geração ativa".into()))?;
        self.require_owner(Operation::Write)?;
        let plan = orchestration::store::load_plan(
            self.database.pool(),
            &self.config.installation_id,
            session_id,
            plan_id,
        )
        .await?
        .ok_or_else(|| TinaError::Unauthorized("plano inexistente".into()))?;
        if !matches!(plan.state.as_str(), "authorized" | "running") {
            return Err(TinaError::ToolError(format!(
                "plano não pode ser retomado no estado {}",
                plan.state
            )));
        }
        let marker = self
            .owner_access
            .session_marker()?
            .ok_or_else(|| TinaError::Unauthorized("sessão bloqueada".into()))?;
        let authorized = plan
            .authorized_fingerprint
            .as_deref()
            .unwrap_or(&plan.fingerprint)
            .to_owned();
        let cancelled = self.plan_cancel_signal(plan_id).subscribe();
        let summary = self
            .execute_plan(session_id, plan_id, &authorized, marker, cancelled)
            .await?;
        let message = self.append_plan_status(session_id, &summary).await?;
        Ok(PlanDecision {
            plan_id: summary.plan.id,
            state: summary.plan.state,
            card: None,
            executed: summary.executed,
            failed: summary.failed,
            skipped: summary.skipped,
            message,
        })
    }

    /// Cancela um plano ativo, preservando etapas já concluídas.
    pub async fn owner_cancel_plan(
        &self,
        session_id: &str,
        plan_id: &str,
        reason: &str,
    ) -> Result<PlanDecision> {
        self.require_owner(Operation::Write)?;
        let existing = orchestration::store::load_plan(
            self.database.pool(),
            &self.config.installation_id,
            session_id,
            plan_id,
        )
        .await?
        .ok_or_else(|| TinaError::Unauthorized("plano inexistente".into()))?;
        if existing.state == "cancelled" {
            return Ok(PlanDecision {
                plan_id: existing.id,
                state: existing.state,
                card: None,
                executed: 0,
                failed: Vec::new(),
                skipped: Vec::new(),
                message: self.last_message_or_synthetic(session_id).await?,
            });
        }
        let _ = self.plan_cancel_signal(plan_id).send(true);
        let plan = orchestration::store::cancel_plan(
            self.database.pool(),
            &self.config.installation_id,
            &self.config.owner_uuid,
            session_id,
            plan_id,
            reason,
        )
        .await?;
        let message = infra::chat::append_message(
            self.database.pool(),
            &self.config.installation_id,
            session_id,
            "assistant",
            "Plano cancelado. Etapas concluídas foram preservadas; nenhuma foi desfeita.",
        )
        .await?;
        Ok(PlanDecision {
            plan_id: plan.id,
            state: plan.state,
            card: None,
            executed: 0,
            failed: Vec::new(),
            skipped: Vec::new(),
            message,
        })
    }

    /// Substitui explicitamente um plano ativo, registrando quem decidiu e por quê.
    pub async fn owner_supersede_plan(
        &self,
        session_id: &str,
        plan_id: &str,
        reason: &str,
    ) -> Result<PlanDecision> {
        self.require_owner(Operation::Write)?;
        let _ = self.plan_cancel_signal(plan_id).send(true);
        let plan = orchestration::store::supersede_plan(
            self.database.pool(),
            &self.config.installation_id,
            &self.config.owner_uuid,
            session_id,
            plan_id,
            reason,
        )
        .await?;
        let message = infra::chat::append_message(
            self.database.pool(),
            &self.config.installation_id,
            session_id,
            "assistant",
            "Plano substituído explicitamente. O novo plano exigirá autorização própria.",
        )
        .await?;
        Ok(PlanDecision {
            plan_id: plan.id,
            state: plan.state,
            card: None,
            executed: 0,
            failed: Vec::new(),
            skipped: Vec::new(),
            message,
        })
    }

    /// Executa as etapas autorizadas de um plano com retomada textual sem ferramentas.
    async fn execute_plan(
        &self,
        session_id: &str,
        plan_id: &str,
        fingerprint: &str,
        marker: std::time::Instant,
        cancelled: watch::Receiver<bool>,
    ) -> Result<orchestration::executor::ExecutionSummary> {
        let app = self;
        let resume_cancel = cancelled.clone();
        orchestration::executor::execute_authorized(
            self.database.pool(),
            &self.config.installation_id,
            &self.config.owner_uuid,
            session_id,
            plan_id,
            fingerprint,
            cancelled,
            move |history, budget| {
                let signal = resume_cancel.clone();
                async move { app.resume_plan_text(history, budget, signal).await }
            },
            move || app.assert_plan_session(marker),
        )
        .await
    }

    /// Continuação da conversa após as chamadas: inferência textual, sem novas ferramentas.
    async fn resume_plan_text(
        &self,
        history: Vec<infra::chat::ChatMessage>,
        budget: i64,
        cancelled: watch::Receiver<bool>,
    ) -> Result<String> {
        self.require_owner(Operation::Write)?;
        let marker = self
            .owner_access
            .session_marker()?
            .ok_or_else(|| TinaError::Unauthorized("sessão bloqueada".into()))?;
        let check_session = || {
            if self.owner_access.session_marker()? != Some(marker) {
                return Err(TinaError::Unauthorized(
                    "sessão alterada ou expirada".into(),
                ));
            }
            Ok(())
        };
        let settings = &self.config.nvidia;
        infra::chat::nvidia_endpoint(&settings.base_url)?;
        if settings.max_tokens == 0 || settings.max_tokens > 4096 {
            return Err(TinaError::Other(
                "limite de saída ainda não validado para o modelo".into(),
            ));
        }
        let key = {
            #[cfg(test)]
            {
                if let Some(k) = self.test_nvidia_key.lock().unwrap().clone() {
                    k
                } else {
                    let entry = keyring::Entry::new("Tina.NVIDIA", &self.config.installation_id)
                        .map_err(|_| TinaError::Other("cofre NVIDIA indisponível".into()))?;
                    entry
                        .get_password()
                        .map_err(|_| TinaError::Other("configure a chave NVIDIA".into()))?
                }
            }
            #[cfg(not(test))]
            {
                let entry = keyring::Entry::new("Tina.NVIDIA", &self.config.installation_id)
                    .map_err(|_| TinaError::Other("cofre NVIDIA indisponível".into()))?;
                entry
                    .get_password()
                    .map_err(|_| TinaError::Other("configure a chave NVIDIA".into()))?
            }
        };
        let context_budget = infra::chat::CHAT_CONTEXT_BYTES
            .saturating_sub(infra::chat::SYSTEM_PROMPT_RESERVE_BYTES);
        let context = infra::chat::context_window(&history, context_budget);
        let max_tokens = settings.max_tokens.min(budget.clamp(1, 4096) as u32);
        let turn = tokio::select! {
            result = infra::chat::stream_nvidia_turn(
                &settings.base_url,
                &key,
                &settings.default_model,
                context,
                max_tokens,
                std::time::Duration::from_secs(180),
                cancelled.clone(),
                false,
                |_delta| {
                    check_session()?;
                    Ok(())
                },
            ) => result?,
            result = async {
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                    check_session()?;
                    if *cancelled.borrow() {
                        break Err::<infra::chat::NvidiaTurn, _>(TinaError::Other(
                            "geração cancelada".into()
                        ));
                    }
                }
            } => result?,
        };
        check_session()?;
        match turn {
            infra::chat::NvidiaTurn::Text(text) => Ok(text),
            infra::chat::NvidiaTurn::Tools(_) => Err(TinaError::ToolError(
                "a retomada não pode ampliar o plano autorizado".into(),
            )),
        }
    }

    /// Revalida o marcador de sessão capturado antes da autorização.
    fn assert_plan_session(&self, marker: std::time::Instant) -> Result<()> {
        if self.owner_access.session_marker()? != Some(marker) {
            return Err(TinaError::Unauthorized(
                "sessão alterada ou expirada".into(),
            ));
        }
        Ok(())
    }

    /// Grava um resumo legível do resultado sem prometer rollback de efeitos.
    async fn append_plan_status(
        &self,
        session_id: &str,
        summary: &orchestration::executor::ExecutionSummary,
    ) -> Result<infra::chat::ChatMessage> {
        let text = match summary.plan.state.as_str() {
            "cancelled" => format!(
                "Plano cancelado durante a execução. {} etapa(s) concluídas foram preservadas.",
                summary.executed
            ),
            "failed" => format!(
                "Plano interrompido: {} etapa(s) concluídas, {} falharam, {} foram bloqueadas. Nenhum efeito já gravado foi desfeito.",
                summary.executed,
                summary.failed.len(),
                summary.skipped.len()
            ),
            _ => format!(
                "Plano concluído: {} etapa(s) executadas com sucesso.",
                summary.executed
            ),
        };
        infra::chat::append_message(
            self.database.pool(),
            &self.config.installation_id,
            session_id,
            "assistant",
            &text,
        )
        .await
    }

    /// Devolve a última mensagem persistida sem gravar uma nova na decisão idempotente.
    async fn last_message_or_synthetic(
        &self,
        session_id: &str,
    ) -> Result<infra::chat::ChatMessage> {
        let messages = infra::chat::list_messages(
            self.database.pool(),
            &self.config.installation_id,
            session_id,
        )
        .await?;
        Ok(messages
            .last()
            .cloned()
            .unwrap_or(infra::chat::ChatMessage {
                id: String::new(),
                role: "assistant".into(),
                content: "Plano já estava cancelado.".into(),
                created_at: String::new(),
                tool_call_id: None,
                tool_calls_json: None,
            }))
    }

    /// Define chave NVIDIA em memória para testes isolados.
    #[cfg(test)]
    pub fn set_test_nvidia_key_for_test(&self, key: &str) {
        *self.test_nvidia_key.lock().unwrap() = Some(key.to_string());
    }

    /// Acesso direto ao banco exclusivamente para testes internos.
    #[cfg(test)]
    pub(crate) fn database(&self) -> &Database {
        &self.database
    }

    /// ForÃƒÂ§a expiraÃƒÂ§ÃƒÂ£o da sessÃƒÂ£o para teste isolado de geraÃƒÂ§ÃƒÂ£o.
    #[cfg(test)]
    pub fn expire_owner_session_for_test(&self) -> Result<()> {
        self.owner_access.expire_session_for_test()
    }

    /// Autentica sessÃƒÂ£o em teste isolado, sem depender de cofre Windows.
    #[cfg(test)]
    pub fn unlock_owner_for_test(&self) -> Result<()> {
        self.owner_access
            .unlock_for_test(self.config.security.session_timeout_seconds)
    }

    /// Cria empresa com auditoria de autoria transacional
    pub async fn owner_create_company(&self, name: &str) -> Result<crate::core::Company> {
        self.require_owner(Operation::Write)?;
        let pool = self.database.pool();
        let installation_id = &self.config.installation_id;
        let owner_id = &self.config.owner_uuid;

        infra::CompanyRepository::new(pool)
            .create_as_owner(installation_id, owner_id, name)
            .await
    }

    /// Cria uma organização com contexto explícito, sem alterar a hierarquia existente.
    pub async fn owner_create_company_with_kind(
        &self,
        name: &str,
        kind: &str,
    ) -> Result<crate::core::Company> {
        self.require_owner(Operation::Write)?;
        infra::CompanyRepository::new(self.database.pool())
            .create_as_owner_with_kind(
                &self.config.installation_id,
                &self.config.owner_uuid,
                name,
                kind,
            )
            .await
    }

    /// Atualiza nome da empresa com verificação de autoria transacional
    pub async fn owner_update_company_name(&self, company_id: &str, new_name: &str) -> Result<()> {
        self.require_owner(Operation::Write)?;
        let installation_id = &self.config.installation_id;
        let owner_id = &self.config.owner_uuid;

        infra::CompanyRepository::new(self.database.pool())
            .update_name_as_owner(company_id, installation_id, owner_id, new_name)
            .await
    }

    /// Arquiva empresa com verificação de autoria transacional
    pub async fn owner_archive_company(&self, company_id: &str) -> Result<()> {
        self.require_owner(Operation::Write)?;
        let installation_id = &self.config.installation_id;
        let owner_id = &self.config.owner_uuid;

        infra::CompanyRepository::new(self.database.pool())
            .archive_as_owner(company_id, installation_id, owner_id)
            .await
    }

    /// Cria departamento com auditoria de autoria transacional
    pub async fn owner_create_department(
        &self,
        company_id: &str,
        code: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<crate::core::Department> {
        self.require_owner(Operation::Write)?;
        let installation_id = &self.config.installation_id;
        let owner_id = &self.config.owner_uuid;

        infra::DepartmentRepository::new(self.database.pool())
            .create_as_owner(
                installation_id,
                owner_id,
                company_id,
                code,
                name,
                description,
            )
            .await
    }

    /// Atualiza nome do departamento com auditoria de autoria transacional
    pub async fn owner_update_department_name(
        &self,
        department_id: &str,
        new_name: &str,
    ) -> Result<()> {
        self.require_owner(Operation::Write)?;
        let installation_id = &self.config.installation_id;
        let owner_id = &self.config.owner_uuid;

        infra::DepartmentRepository::new(self.database.pool())
            .update_name_as_owner(department_id, installation_id, owner_id, new_name)
            .await
    }

    /// Arquiva departamento com auditoria de autoria transacional
    pub async fn owner_archive_department(&self, department_id: &str) -> Result<()> {
        self.require_owner(Operation::Write)?;
        let installation_id = &self.config.installation_id;
        let owner_id = &self.config.owner_uuid;

        infra::DepartmentRepository::new(self.database.pool())
            .archive_as_owner(department_id, installation_id, owner_id)
            .await
    }

    /// Cria cargo com auditoria de autoria transacional
    pub async fn owner_create_position(
        &self,
        department_id: &str,
        name: &str,
        level: i32,
        authority_level: i32,
        description: Option<&str>,
    ) -> Result<crate::core::Position> {
        self.require_owner(Operation::Write)?;
        let installation_id = &self.config.installation_id;
        let owner_id = &self.config.owner_uuid;

        infra::PositionRepository::new(self.database.pool())
            .create_as_owner(
                installation_id,
                owner_id,
                department_id,
                name,
                level,
                authority_level,
                description,
            )
            .await
    }

    /// Atualiza nome do cargo com auditoria de autoria transacional
    pub async fn owner_update_position_name(
        &self,
        position_id: &str,
        new_name: &str,
    ) -> Result<()> {
        self.require_owner(Operation::Write)?;
        let installation_id = &self.config.installation_id;
        let owner_id = &self.config.owner_uuid;

        infra::PositionRepository::new(self.database.pool())
            .update_name_as_owner(position_id, installation_id, owner_id, new_name)
            .await
    }

    /// Arquiva cargo com auditoria de autoria transacional
    pub async fn owner_archive_position(&self, position_id: &str) -> Result<()> {
        self.require_owner(Operation::Write)?;
        let installation_id = &self.config.installation_id;
        let owner_id = &self.config.owner_uuid;

        infra::PositionRepository::new(self.database.pool())
            .archive_as_owner(position_id, installation_id, owner_id)
            .await
    }

    /// Atualiza descrição do cargo com auditoria de autoria transacional
    pub async fn owner_update_position_description(
        &self,
        position_id: &str,
        description: Option<&str>,
    ) -> Result<()> {
        self.require_owner(Operation::Write)?;
        let installation_id = &self.config.installation_id;
        let owner_id = &self.config.owner_uuid;

        infra::PositionRepository::new(self.database.pool())
            .update_description_as_owner(position_id, installation_id, owner_id, description)
            .await
    }

    /// Atualiza nome e descrição do cargo em uma única transação atômica
    pub async fn owner_update_position_details(
        &self,
        position_id: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<()> {
        self.require_owner(Operation::Write)?;
        let installation_id = &self.config.installation_id;
        let owner_id = &self.config.owner_uuid;

        infra::PositionRepository::new(self.database.pool())
            .update_details_as_owner(position_id, installation_id, owner_id, name, description)
            .await
    }

    /// Atualiza descrição do departamento com auditoria de autoria transacional
    pub async fn owner_update_department_description(
        &self,
        department_id: &str,
        description: Option<&str>,
    ) -> Result<()> {
        self.require_owner(Operation::Write)?;
        let installation_id = &self.config.installation_id;
        let owner_id = &self.config.owner_uuid;

        infra::DepartmentRepository::new(self.database.pool())
            .update_description_as_owner(department_id, installation_id, owner_id, description)
            .await
    }

    /// Atualiza nome e descrição do departamento em uma única transação atômica
    pub async fn owner_update_department_details(
        &self,
        department_id: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<()> {
        self.require_owner(Operation::Write)?;
        let installation_id = &self.config.installation_id;
        let owner_id = &self.config.owner_uuid;

        infra::DepartmentRepository::new(self.database.pool())
            .update_details_as_owner(department_id, installation_id, owner_id, name, description)
            .await
    }

    /// Atualiza supervisor do funcionário com autorização transacional
    pub async fn owner_update_employee_supervisor(
        &self,
        employee_id: &str,
        supervisor_id: Option<&str>,
    ) -> Result<()> {
        self.require_owner(Operation::Write)?;
        let installation_id = &self.config.installation_id;
        let owner_id = &self.config.owner_uuid;

        // Obter company_id
        let company_id: Option<String> = sqlx::query_scalar(
            r#"
            SELECT c.id
            FROM employees e
            JOIN positions p ON e.position_id = p.id
            JOIN departments d ON p.department_id = d.id
            JOIN companies c ON d.company_id = c.id
            WHERE e.id = ? AND c.status = 'active'
            "#,
        )
        .bind(employee_id)
        .fetch_optional(self.database.pool())
        .await?;

        let company_id =
            company_id.ok_or_else(|| TinaError::Other("Funcionário não encontrado".to_string()))?;

        infra::EmployeeRepository::new(self.database.pool())
            .update_supervisor_as_owner(
                employee_id,
                installation_id,
                owner_id,
                &company_id,
                supervisor_id,
            )
            .await
    }

    /// Cria funcionário com auditoria de autoria transacional
    pub async fn owner_create_employee(
        &self,
        position_id: &str,
        name: &str,
        responsibilities: Option<&str>,
        supervisor_id: Option<&str>,
    ) -> Result<crate::core::Employee> {
        self.require_owner(Operation::Write)?;
        let installation_id = &self.config.installation_id;
        let owner_id = &self.config.owner_uuid;

        infra::EmployeeRepository::new(self.database.pool())
            .create_as_owner(
                installation_id,
                owner_id,
                position_id,
                name,
                responsibilities,
                supervisor_id,
            )
            .await
    }

    /// Atualiza informações do funcionário com auditoria transacional
    pub async fn owner_update_employee_info(
        &self,
        employee_id: &str,
        name: &str,
        responsibilities: Option<&str>,
    ) -> Result<()> {
        self.require_owner(Operation::Write)?;
        let installation_id = &self.config.installation_id;
        let owner_id = &self.config.owner_uuid;

        // Obter company_id
        let company_id: Option<String> = sqlx::query_scalar(
            r#"
            SELECT c.id
            FROM employees e
            JOIN positions p ON e.position_id = p.id
            JOIN departments d ON p.department_id = d.id
            JOIN companies c ON d.company_id = c.id
            WHERE e.id = ? AND c.status = 'active'
            "#,
        )
        .bind(employee_id)
        .fetch_optional(self.database.pool())
        .await?;

        let company_id =
            company_id.ok_or_else(|| TinaError::Other("Funcionário não encontrado".to_string()))?;

        infra::EmployeeRepository::new(self.database.pool())
            .update_info_as_owner(
                employee_id,
                installation_id,
                owner_id,
                &company_id,
                name,
                responsibilities,
            )
            .await
    }

    /// Atualiza responsabilidades do funcionário com auditoria transacional
    pub async fn owner_update_employee_responsibilities(
        &self,
        employee_id: &str,
        responsibilities: Option<&str>,
    ) -> Result<()> {
        self.require_owner(Operation::Write)?;
        let installation_id = &self.config.installation_id;
        let owner_id = &self.config.owner_uuid;

        // Obter company_id
        let company_id: Option<String> = sqlx::query_scalar(
            r#"
            SELECT c.id
            FROM employees e
            JOIN positions p ON e.position_id = p.id
            JOIN departments d ON p.department_id = d.id
            JOIN companies c ON d.company_id = c.id
            WHERE e.id = ? AND c.status = 'active'
            "#,
        )
        .bind(employee_id)
        .fetch_optional(self.database.pool())
        .await?;

        let company_id =
            company_id.ok_or_else(|| TinaError::Other("Funcionário não encontrado".to_string()))?;

        infra::EmployeeRepository::new(self.database.pool())
            .update_responsibilities_as_owner(
                employee_id,
                installation_id,
                owner_id,
                &company_id,
                responsibilities,
            )
            .await
    }

    /// Desativa funcionário com auditoria transacional
    pub async fn owner_deactivate_employee(&self, employee_id: &str) -> Result<()> {
        self.require_owner(Operation::Write)?;
        let installation_id = &self.config.installation_id;
        let owner_id = &self.config.owner_uuid;

        // Obter company_id
        let company_id: Option<String> = sqlx::query_scalar(
            r#"
            SELECT c.id
            FROM employees e
            JOIN positions p ON e.position_id = p.id
            JOIN departments d ON p.department_id = d.id
            JOIN companies c ON d.company_id = c.id
            WHERE e.id = ? AND c.status = 'active'
            "#,
        )
        .bind(employee_id)
        .fetch_optional(self.database.pool())
        .await?;

        let company_id =
            company_id.ok_or_else(|| TinaError::Other("Funcionário não encontrado".to_string()))?;

        infra::EmployeeRepository::new(self.database.pool())
            .deactivate_as_owner(employee_id, installation_id, owner_id, &company_id)
            .await
    }

    /// Alterar cargo do funcionário com autoria transacional
    pub async fn owner_change_employee_position(
        &self,
        employee_id: &str,
        new_position_id: &str,
        reason: Option<&str>,
    ) -> Result<crate::core::assignment::PositionChange> {
        let pool = self.database.pool();
        let installation_id = self.config.installation_id.clone();

        // Verificar sessão antes de qualquer operação
        if !self.owner_access.is_unlocked()? {
            // Registrar tentativa de escrita negada
            let _ = infra::audit::record_denied_owner_audit_write(
                pool,
                &installation_id,
                "employee",
                employee_id,
                "position_change",
            )
            .await;
            return Err(TinaError::Unauthorized(
                "sessão do proprietário bloqueada".into(),
            ));
        }

        let owner_id = &self.config.owner_uuid;

        // Obter company_id
        let company_id: Option<String> = sqlx::query_scalar(
            r#"
            SELECT c.id
            FROM employees e
            JOIN positions p ON e.position_id = p.id
            JOIN departments d ON p.department_id = d.id
            JOIN companies c ON d.company_id = c.id
            WHERE e.id = ? AND c.status = 'active'
            "#,
        )
        .bind(employee_id)
        .fetch_optional(self.database.pool())
        .await?;

        let company_id =
            company_id.ok_or_else(|| TinaError::Other("Funcionário não encontrado".to_string()))?;

        infra::EmployeeRepository::new(self.database.pool())
            .change_position_with_owner_as_owner(
                employee_id,
                &installation_id,
                owner_id,
                &company_id,
                new_position_id,
                reason,
            )
            .await
    }

    /// Reativa funcionário com auditoria transacional
    pub async fn owner_reactivate_employee(&self, employee_id: &str) -> Result<()> {
        self.require_owner(Operation::Write)?;
        let installation_id = &self.config.installation_id;
        let owner_id = &self.config.owner_uuid;

        // Obter company_id
        let company_id: Option<String> = sqlx::query_scalar(
            r#"
            SELECT c.id
            FROM employees e
            JOIN positions p ON e.position_id = p.id
            JOIN departments d ON p.department_id = d.id
            JOIN companies c ON d.company_id = c.id
            WHERE e.id = ?
            "#,
        )
        .bind(employee_id)
        .fetch_optional(self.database.pool())
        .await?;

        let company_id =
            company_id.ok_or_else(|| TinaError::Other("Funcionário não encontrado".to_string()))?;

        infra::EmployeeRepository::new(self.database.pool())
            .reactivate_as_owner(employee_id, installation_id, owner_id, &company_id)
            .await
    }

    /// Lista eventos de auditoria organizacionais para a instalação.
    /// Filtra por empresa, se especificado. Requer sessão de proprietário ativa.
    pub async fn owner_audit_events(
        &self,
        company_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<infra::audit::AuditEvent>> {
        // Verificar sessão antes de qualquer operação
        let pool = self.database.pool();
        let installation_id = self.config.installation_id.clone();

        if !self.owner_access.is_unlocked()? {
            // Registrar tentativa de acesso negada apenas quando falha
            infra::audit::record_denied_owner_audit_read(pool, &installation_id).await?;
            return Err(TinaError::Unauthorized(
                "sessão do proprietário bloqueada".into(),
            ));
        }

        if limit == 0 {
            return Err(TinaError::Validation("limite deve ser positivo".into()));
        }
        if let Some(cid) = company_id {
            let belongs = infra::CompanyRepository::new(pool)
                .belongs_to_installation(cid, &installation_id)
                .await?;
            if !belongs {
                return Err(TinaError::Unauthorized("empresa fora da instalação".into()));
            }
        }
        infra::audit::list_events(pool, &installation_id, company_id, limit).await
    }

    /// Lista eventos de auditoria antes de um cursor, para paginação.
    pub async fn owner_audit_events_before(
        &self,
        company_id: Option<&str>,
        before_id: &str,
        limit: i64,
    ) -> Result<Vec<infra::audit::AuditEvent>> {
        // Verificar sessão antes de qualquer operação
        let pool = self.database.pool();
        let installation_id = self.config.installation_id.clone();

        if !self.owner_access.is_unlocked()? {
            // Registrar tentativa de acesso negada apenas quando falha
            infra::audit::record_denied_owner_audit_read(pool, &installation_id).await?;
            return Err(TinaError::Unauthorized(
                "sessão do proprietário bloqueada".into(),
            ));
        }

        if limit == 0 {
            return Err(TinaError::Validation("limite deve ser positivo".into()));
        }
        if let Some(cid) = company_id {
            let belongs = infra::CompanyRepository::new(pool)
                .belongs_to_installation(cid, &installation_id)
                .await?;
            if !belongs {
                return Err(TinaError::Unauthorized("empresa fora da instalação".into()));
            }
        }
        infra::audit::list_events_before(pool, &installation_id, company_id, before_id, limit).await
    }
}
