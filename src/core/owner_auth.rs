//! A conta local do proprietário usa Argon2id para login e o cofre do Windows
//! para uma chave aleatória de 256 bits reservada para uso futuro no banco.
//! O SQLite ainda NÃO é criptografado por essa chave.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand::{rngs::OsRng, RngCore};
use sqlx::SqlitePool;

use crate::core::key_recovery::{self, WrappedKey};
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
    PasswordHash::new(stored).ok().is_some_and(|hash| {
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
            .map_err(|_| {
                TinaError::Auth("não foi possível salvar a chave no cofre do Windows".into())
            })
    }
    #[cfg(not(windows))]
    {
        let _ = (installation_id, key);
        Err(TinaError::Auth(
            "primeiro acesso disponível no Windows".into(),
        ))
    }
}

fn vault_key_available(installation_id: &str) -> Result<bool> {
    #[cfg(windows)]
    {
        let secret = vault_entry(installation_id)?.get_password().map_err(|_| {
            TinaError::Auth(
                "chave do proprietário ausente ou inacessível no cofre do Windows".into(),
            )
        })?;
        Ok(secret.len() == 64 && secret.bytes().all(|b| b.is_ascii_hexdigit()))
    }
    #[cfg(not(windows))]
    {
        let _ = installation_id;
        Err(TinaError::Auth(
            "cofre do Windows não disponível neste sistema".into(),
        ))
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

#[cfg(windows)]
fn vault_missing(installation_id: &str) -> Result<bool> {
    match vault_entry(installation_id)?.get_password() {
        Ok(value) if value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()) => Ok(false),
        Ok(_) => Err(TinaError::Auth("chave presente no cofre inválida".into())),
        Err(keyring::Error::NoEntry) => Ok(true),
        Err(_) => Err(TinaError::Auth("cofre Windows indisponível".into())),
    }
}

#[cfg(not(windows))]
fn vault_missing(_installation_id: &str) -> Result<bool> {
    Err(TinaError::Auth("cofre Windows indisponível".into()))
}

async fn restore_from_backup(
    pool: &SqlitePool,
    installation_id: &str,
    password: &str,
) -> Result<()> {
    let item: Option<(Vec<u8>, Vec<u8>, Vec<u8>)> = sqlx::query_as(
        "SELECT salt, nonce, ciphertext FROM owner_key_recovery WHERE installation_id = ?",
    )
    .bind(installation_id)
    .fetch_optional(pool)
    .await?;
    let (salt, nonce, ciphertext) =
        item.ok_or_else(|| TinaError::Auth("chave ausente e backup indisponível".into()))?;
    let encrypted = WrappedKey {
        salt,
        nonce,
        ciphertext,
    };
    let mut original = key_recovery::unwrap(password, installation_id, &encrypted)?;
    let secret: String = original.iter().map(|b| format!("{b:02x}")).collect();
    original.fill(0);
    store_database_key(installation_id, &secret)?;
    if !vault_key_available(installation_id)? {
        return Err(TinaError::Auth("chave restaurada não verificada".into()));
    }
    Ok(())
}

async fn ensure_backup(pool: &SqlitePool, installation_id: &str, password: &str) -> Result<()> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM owner_key_recovery WHERE installation_id = ?")
            .bind(installation_id)
            .fetch_one(pool)
            .await?;
    if count > 0 {
        return Ok(());
    }
    #[cfg(windows)]
    {
        let value = vault_entry(installation_id)?
            .get_password()
            .map_err(|_| TinaError::Auth("cofre Windows indisponível".into()))?;
        if value.len() != 64 {
            return Err(TinaError::Auth("chave do cofre inválida".into()));
        }
        let mut original = [0u8; 32];
        for (i, pair) in value.as_bytes().chunks_exact(2).enumerate() {
            let part =
                std::str::from_utf8(pair).map_err(|_| TinaError::Auth("chave inválida".into()))?;
            original[i] = u8::from_str_radix(part, 16)
                .map_err(|_| TinaError::Auth("chave inválida".into()))?;
        }
        let wrapped = key_recovery::wrap(password, installation_id, &original)?;
        original.fill(0);
        sqlx::query(
            "INSERT OR IGNORE INTO owner_key_recovery (installation_id, salt, nonce, ciphertext) VALUES (?, ?, ?, ?)",
        )
        .bind(installation_id)
        .bind(&wrapped.salt)
        .bind(&wrapped.nonce)
        .bind(&wrapped.ciphertext)
        .execute(pool).await?;
    }
    #[cfg(not(windows))]
    let _ = password;
    Ok(())
}

/// Evita autenticar com uma chave do cofre trocada, ainda que seja 64 hex.
async fn verify_vault_matches_backup(
    pool: &SqlitePool,
    installation_id: &str,
    password: &str,
) -> Result<()> {
    let row: Option<(Vec<u8>, Vec<u8>, Vec<u8>)> = sqlx::query_as(
        "SELECT salt, nonce, ciphertext FROM owner_key_recovery WHERE installation_id = ?",
    )
    .bind(installation_id)
    .fetch_optional(pool)
    .await?;
    let (salt, nonce, ciphertext) =
        row.ok_or_else(|| TinaError::Auth("backup da chave ausente".into()))?;

    let wrapped = WrappedKey {
        salt,
        nonce,
        ciphertext,
    };
    let mut expected = key_recovery::unwrap(password, installation_id, &wrapped)?;
    #[cfg(windows)]
    {
        let observed = vault_entry(installation_id)?
            .get_password()
            .map_err(|_| TinaError::Auth("chave do cofre indisponível".into()))?;
        if observed.len() != 64 {
            expected.fill(0);
            return Err(TinaError::Auth("chave do cofre inválida".into()));
        }
        // Comparação uniforme, sem retornar a chave ao frontend.
        let mut difference = 0u8;
        for (idx, byte) in expected.iter().enumerate() {
            let ascii = b"0123456789abcdef";
            difference |= observed.as_bytes()[idx * 2] ^ ascii[(byte >> 4) as usize];
            difference |= observed.as_bytes()[idx * 2 + 1] ^ ascii[(byte & 0x0f) as usize];
        }
        expected.fill(0);
        if difference != 0 {
            return Err(TinaError::Auth(
                "a chave do cofre não corresponde ao backup autenticado".into(),
            ));
        }
    }
    #[cfg(not(windows))]
    {
        expected.fill(0);
        return Err(TinaError::Auth("cofre Windows indisponível".into()));
    }
    Ok(())
}

impl OwnerAccess {
    /// Lê estado público da conta e verifica a validade da sessão local.
    pub async fn status(
        &self,
        pool: &SqlitePool,
        installation_id: &str,
    ) -> Result<OwnerAccountStatus> {
        let display_name: Option<String> =
            sqlx::query_scalar("SELECT display_name FROM owner_accounts WHERE installation_id = ?")
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
            return Err(TinaError::Validation(
                "nome deve ter entre 2 e 80 caracteres".into(),
            ));
        }
        if password.chars().count() < 12 || password.chars().count() > 256 {
            return Err(TinaError::Validation(
                "senha deve ter entre 12 e 256 caracteres".into(),
            ));
        }
        // A escrita do cadastro reserva atomicamente a instalação em SQLite
        // ANTES de tocar no cofre. Isso impede que dois processos paralelos
        // sobrescrevam ou apaguem a chave do primeiro proprietário cadastrado.
        let hash = password_hash(password)?;
        let mut tx = pool.begin().await?;
        let inserted = sqlx::query(
            "INSERT INTO owner_accounts (installation_id, owner_uuid, display_name, password_hash) \
             SELECT id, owner_uuid, ?, ? FROM installations \
             WHERE id = ? AND owner_uuid = ? \
             AND NOT EXISTS (SELECT 1 FROM owner_accounts WHERE installation_id = ?)",
        )
        .bind(name)
        .bind(&hash)
        .bind(installation_id)
        .bind(owner_uuid)
        .bind(installation_id)
        .execute(&mut *tx)
        .await?;

        if inserted.rows_affected() != 1 {
            return Err(TinaError::PermissionDenied(
                "proprietário já cadastrado ou instalação inválida".into(),
            ));
        }

        let mut key_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut key_bytes);
        let key: String = key_bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        let wrapped = key_recovery::wrap(password, installation_id, &key_bytes)?;
        key_bytes.fill(0);
        sqlx::query(
            "INSERT INTO owner_key_recovery (installation_id, salt, nonce, ciphertext) VALUES (?, ?, ?, ?)",
        )
        .bind(installation_id)
        .bind(&wrapped.salt)
        .bind(&wrapped.nonce)
        .bind(&wrapped.ciphertext)
        .execute(&mut *tx)
        .await?;

        // A chave só existe no cofre do SO; se a gravação falhar,
        // descartar a transação remove a conta incompleta do banco.
        if let Err(error) = store_database_key(installation_id, &key) {
            rollback_database_key(installation_id);
            return Err(error);
        }
        let read_back = vault_key_available(installation_id);
        if !matches!(read_back, Ok(true)) {
            rollback_database_key(installation_id);
            return Err(TinaError::Auth(
                "falha ao verificar a chave no cofre do Windows".into(),
            ));
        }

        // SQLite e Credential Manager são sistemas independentes: em falha
        // de commit, tentar compensar removendo a chave recém-gerada.
        if let Err(err) = tx.commit().await {
            rollback_database_key(installation_id);
            return Err(err.into());
        }

        Ok(())
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

        let (hash, _failures, locked_until) =
            row.ok_or_else(|| TinaError::Auth("crie a conta de proprietário primeiro".into()))?;
        let now = chrono::Utc::now().timestamp();
        if locked_until > now {
            return Err(TinaError::Auth(
                "muitas tentativas; acesso temporariamente bloqueado".into(),
            ));
        }
        if !password_matches(password, &hash) {
            let limit = i64::from(max_attempts.max(1));
            let until =
                now.saturating_add(i64::try_from(lockout_seconds.max(1)).unwrap_or(i64::MAX));
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

        if vault_missing(installation_id)? {
            restore_from_backup(pool, installation_id, password).await?;
        }
        if !vault_key_available(installation_id)? {
            return Err(TinaError::Auth("chave inválida no cofre do Windows".into()));
        }
        ensure_backup(pool, installation_id, password).await?;
        verify_vault_matches_backup(pool, installation_id, password).await?;

        sqlx::query(
            "UPDATE owner_accounts SET failed_attempts = 0, locked_until = 0 WHERE installation_id = ?",
        )
        .bind(installation_id)
        .execute(pool)
        .await?;

        // Sessão sem expiração é proibida, inclusive por configuração inválida.
        if timeout_seconds == 0 {
            return Err(TinaError::Auth("duração de sessão deve ser positiva".into()));
        }
        let expiry = Instant::now()
            .checked_add(Duration::from_secs(timeout_seconds))
            .ok_or_else(|| TinaError::Auth("duração de sessão inválida".into()))?;
        let mut session = self
            .session
            .lock()
            .map_err(|_| TinaError::Auth("sessão indisponível".into()))?;
        session.unlocked = true;
        session.expires_at = Some(expiry);
        Ok(())
    }

    /// Indica se o proprietário está autenticado e se a sessão ainda não expirou.
    pub fn is_unlocked(&self) -> Result<bool> {
        let mut session = self
            .session
            .lock()
            .map_err(|_| TinaError::Auth("sessão indisponível".into()))?;
        if session
            .expires_at
            .is_some_and(|expires| expires <= Instant::now())
        {
            *session = Session::default();
        }
        Ok(session.unlocked)
    }

    /// Encerra a sessão imediatamente.
    pub fn lock(&self) -> Result<()> {
        let mut session = self
            .session
            .lock()
            .map_err(|_| TinaError::Auth("sessão indisponível".into()))?;
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
    fn expired_session_fails_closed_and_revokes_access() {
        let access = OwnerAccess::default();
        {
            let mut session = access.session.lock().unwrap();
            session.unlocked = true;
            session.expires_at = Some(Instant::now() - Duration::from_millis(1));
        }
        assert!(!access.is_unlocked().unwrap());
        assert!(!access.session.lock().unwrap().unlocked);
    }

    #[test]
    fn session_starts_closed_and_lock_is_idempotent() {
        let access = OwnerAccess::default();
        assert!(!access.is_unlocked().unwrap());
        access.lock().unwrap();
        assert!(!access.is_unlocked().unwrap());
    }
}
