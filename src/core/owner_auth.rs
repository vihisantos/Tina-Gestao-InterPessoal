//! A conta local do proprietário usa Argon2id para login e o cofre do Windows
//! para uma chave aleatória de 256 bits reservada para uso futuro no banco.
//! O SQLite ainda NÃO é criptografado por essa chave.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand::{rngs::OsRng, RngCore};
use sqlx::SqlitePool;

use crate::error::{Result, TinaError};

/// Estado de cadastro do proprietário retornado à interface, sem segredos.
#[derive(Debug, Clone)]
pub struct OwnerAccountStatus {
    /// Se a conta inicial já foi criada.
    pub configured: bool,
    /// Se existe sessão local válida.
    pub unlocked: bool,
    /// Nome de exibição do proprietário, se cadastrado.
    pub display_name: Option<String>,
}

/// Guarda apenas uma sessão em memória; não é uma credencial persistente.
#[derive(Debug, Default)]
pub struct OwnerAccess {
    session: Mutex<Session>,
    setup_guard: tokio::sync::Mutex<()>,
}

#[derive(Debug, Default)]
struct Session {
    unlocked: bool,
    expires_at: Option<Instant>,
}

fn password_hash(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| TinaError::Auth("falha ao proteger senha".into()))
}

fn password_matches(password: &str, stored: &str) -> bool {
    PasswordHash::new(stored)
        .ok()
        .is_some_and(|hash| {
            Argon2::default()
                .verify_password(password.as_bytes(), &hash)
                .is_ok()
        })
}

#[cfg(windows)]
fn vault_entry(installation_id: &str) -> Result<keyring::Entry> {
    keyring::Entry::new("Tina/owner/database-key/v1", installation_id)
        .map_err(|_| TinaError::Auth("não foi possível acessar o cofre do Windows".into()))
}

fn store_database_key(installation_id: &str, key: &str) -> Result<()> {
    #[cfg(windows)]
    {
        vault_entry(installation_id)?
            .set_password(key)
            .map_err(|_| TinaError::Auth("não foi possível salvar a chave no cofre do Windows".into()))
    }
    #[cfg(not(windows))]
    {
        let _ = (installation_id, key);
        Err(TinaError::Auth("primeiro acesso disponível no Windows".into()))
    }
}

fn vault_key_available(installation_id: &str) -> Result<bool> {
    #[cfg(windows)]
    {
        let secret = vault_entry(installation_id)?
            .get_password()
            .map_err(|_| TinaError::Auth("chave do proprietário ausente ou inacessível no cofre do Windows".into()))?;
        Ok(secret.len() == 64 && secret.bytes().all(|b| b.is_ascii_hexdigit()))
    }
    #[cfg(not(windows))]
    {
        let _ = installation_id;
        Err(TinaError::Auth("cofre do Windows não disponível neste sistema".into()))
    }
}

/// Apaga uma entrada criada durante cadastro incompleto; não é fluxo de recuperação.
fn rollback_database_key(installation_id: &str) {
    #[cfg(windows)]
    {
        if let Ok(entry) = vault_entry(installation_id) {
            let _ = entry.delete_credential();
        }
    }
    #[cfg(not(windows))]
    let _ = installation_id;
}

impl OwnerAccess {
    /// Lê estado público da conta e verifica a validade da sessão local.
    pub async fn status(&self, pool: &SqlitePool, installation_id: &str) -> Result<OwnerAccountStatus> {
        let display_name: Option<String> = sqlx::query_scalar(
            "SELECT display_name FROM owner_accounts WHERE installation_id = ?",
        )
        .bind(installation_id)
        .fetch_optional(pool)
        .await?;
        Ok(OwnerAccountStatus {
            configured: display_name.is_some(),
            unlocked: self.is_unlocked()?,
            display_name,
        })
    }

    /// Cadastro inicial de uma única conta, com chave exclusiva no cofre do SO.
    pub async fn create_account(
        &self,
        pool: &SqlitePool,
        installation_id: &str,
        owner_uuid: &str,
        display_name: &str,
        password: &str,
    ) -> Result<()> {
        let _guard = self.setup_guard.lock().await;
        let name = display_name.trim();
        if name.chars().count() < 2 || name.chars().count() > 80 {
            return Err(TinaError::Validation("nome deve ter entre 2 e 80 caracteres".into()));
        }
        if password.chars().count() < 12 || password.chars().count() > 256 {
            return Err(TinaError::Validation("senha deve ter entre 12 e 256 caracteres".into()));
        }
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM owner_accounts WHERE installation_id = ?",
        )
        .bind(installation_id)
        .fetch_one(pool)
        .await?;
        if exists != 0 {
            return Err(TinaError::PermissionDenied("proprietário já cadastrado".into()));
        }

        let hash = password_hash(password)?;
        let mut key_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut key_bytes);
        let key: String = key_bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        // A chave secreta nunca é enviada ao frontend nem gravada no SQLite.
        store_database_key(installation_id, &key)?;
        if !vault_key_available(installation_id)? {
            rollback_database_key(installation_id);
            return Err(TinaError::Auth("falha ao conferir chave do cofre".into()));
        }

        let result = sqlx::query(
            "INSERT INTO owner_accounts (installation_id, owner_uuid, display_name, password_hash) \
             SELECT id, owner_uuid, ?, ? FROM installations \
             WHERE id = ? AND owner_uuid = ?",
        )
        .bind(name)
        .bind(&hash)
        .bind(installation_id)
        .bind(owner_uuid)
        .execute(pool)
        .await;

        match result {
            Ok(rows) if rows.rows_affected() == 1 => Ok(()),
            Ok(_) => {
                rollback_database_key(installation_id);
                Err(TinaError::Unauthorized("instalação não autorizada para cadastro".into()))
            }
            Err(err) => {
                rollback_database_key(installation_id);
                Err(err.into())
            }
        }
    }

    /// Valida senha e presença da chave de banco, sem revelar a chave.
    pub async fn sign_in(
        &self,
        pool: &SqlitePool,
        installation_id: &str,
        password: &str,
        timeout_seconds: u64,
        max_attempts: u32,
        lockout_seconds: u64,
    ) -> Result<()> {
        let row: Option<(String, i64, i64)> = sqlx::query_as(
            "SELECT password_hash, failed_attempts, locked_until \
             FROM owner_accounts WHERE installation_id = ?",
        )
        .bind(installation_id)
        .fetch_optional(pool)
        .await?;

        let (hash, _failures, locked_until) = row.ok_or_else(|| {
            TinaError::Auth("crie a conta de proprietário primeiro".into())
        })?;
        let now = chrono::Utc::now().timestamp();
        if locked_until > now {
            return Err(TinaError::Auth("muitas tentativas; acesso temporariamente bloqueado".into()));
        }
        if !password_matches(password, &hash) {
            let limit = i64::from(max_attempts.max(1));
            let until = now.saturating_add(i64::try_from(lockout_seconds.max(1)).unwrap_or(i64::MAX));
            sqlx::query(
                "UPDATE owner_accounts SET \
                 locked_until = CASE WHEN failed_attempts + 1 >= ? THEN ? ELSE 0 END, \
                 failed_attempts = CASE WHEN failed_attempts + 1 >= ? THEN 0 ELSE failed_attempts + 1 END \
                 WHERE installation_id = ?",
            )
            .bind(limit)
            .bind(until)
            .bind(limit)
            .bind(installation_id)
            .execute(pool)
            .await?;
            return Err(TinaError::Auth("senha incorreta".into()));
        }

        if !vault_key_available(installation_id)? {
            return Err(TinaError::Auth("chave inválida no cofre do Windows".into()));
        }

        sqlx::query(
            "UPDATE owner_accounts SET failed_attempts = 0, locked_until = 0 WHERE installation_id = ?",
        )
        .bind(installation_id)
        .execute(pool)
        .await?;

        let mut session = self.session.lock().map_err(|_| TinaError::Auth("sessão indisponível".into()))?;
        session.unlocked = true;
        session.expires_at = if timeout_seconds == 0 {
            None
        } else {
            Instant::now().checked_add(Duration::from_secs(timeout_seconds))
        };
        Ok(())
    }

    /// Indica se o proprietário está autenticado e se a sessão ainda não expirou.
    pub fn is_unlocked(&self) -> Result<bool> {
        let mut session = self.session.lock().map_err(|_| TinaError::Auth("sessão indisponível".into()))?;
        if session.expires_at.is_some_and(|expires| expires <= Instant::now()) {
            *session = Session::default();
        }
        Ok(session.unlocked)
    }

    /// Encerra a sessão imediatamente.
    pub fn lock(&self) -> Result<()> {
        let mut session = self.session.lock().map_err(|_| TinaError::Auth("sessão indisponível".into()))?;
        *session = Session::default();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_is_salted_and_never_persisted_as_cleartext() {
        let first = password_hash("senha-super-forte-123").unwrap();
        let second = password_hash("senha-super-forte-123").unwrap();
        assert_ne!(first, second);
        assert!(first.starts_with("$argon2id$"));
        assert!(password_matches("senha-super-forte-123", &first));
        assert!(!password_matches("errada", &first));
    }

    #[test]
    fn session_starts_closed_and_lock_is_idempotent() {
        let access = OwnerAccess::default();
        assert!(!access.is_unlocked().unwrap());
        access.lock().unwrap();
        assert!(!access.is_unlocked().unwrap());
    }
}
