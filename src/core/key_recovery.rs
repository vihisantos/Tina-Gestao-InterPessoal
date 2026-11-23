//! Recuperação da chave do proprietário com AES-256-GCM e Argon2id.
//! O arquivo SQLite continua sem criptografia nesta fase.

use crate::error::{Result, TinaError};
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::Argon2;
use rand::{rngs::OsRng, RngCore};

/// Backup criptografado da chave de 256 bits armazenada no cofre Windows.
#[derive(Debug)]
pub struct WrappedKey {
    /// Salt de 16 bytes exclusivo deste backup.
    pub salt: Vec<u8>,
    /// Nonce aleatório AES-GCM de 12 bytes.
    pub nonce: Vec<u8>,
    /// Chave cifrada e autenticada (48 bytes).
    pub ciphertext: Vec<u8>,
}

fn derive(password: &str, salt: &[u8]) -> Result<[u8; 32]> {
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|_| TinaError::Auth("derivação da chave de recuperação falhou".into()))?;
    Ok(key)
}

/// Cria uma cópia cifrada associada a esta instalação.
pub fn wrap(password: &str, installation_id: &str, raw: &[u8; 32]) -> Result<WrappedKey> {
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce);
    let mut key = derive(password, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| TinaError::Auth("chave de recuperação inválida".into()))?;
    let result = cipher.encrypt(
        Nonce::from_slice(&nonce),
        Payload {
            msg: raw,
            aad: installation_id.as_bytes(),
        },
    );
    key.fill(0);
    Ok(WrappedKey {
        salt: salt.to_vec(),
        nonce: nonce.to_vec(),
        ciphertext: result.map_err(|_| TinaError::Auth("falha ao proteger chave".into()))?,
    })
}

/// Recupera uma chave com a senha e a instalação corretas.
pub fn unwrap(password: &str, installation_id: &str, backup: &WrappedKey) -> Result<[u8; 32]> {
    if backup.salt.len() != 16 || backup.nonce.len() != 12 || backup.ciphertext.len() != 48 {
        return Err(TinaError::Auth("backup inválido".into()));
    }
    let mut key = derive(password, &backup.salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| TinaError::Auth("chave de recuperação inválida".into()))?;
    let result = cipher.decrypt(
        Nonce::from_slice(&backup.nonce),
        Payload {
            msg: &backup.ciphertext,
            aad: installation_id.as_bytes(),
        },
    );
    key.fill(0);
    result
        .map_err(|_| TinaError::Auth("não foi possível recuperar a chave".into()))?
        .try_into()
        .map_err(|_| TinaError::Auth("chave recuperada de tamanho inválido".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_negative_cases() {
        let raw = [47u8; 32];
        let a = wrap("senha mestra longa do teste", "a", &raw).unwrap();
        let b = wrap("senha mestra longa do teste", "a", &raw).unwrap();
        assert_ne!(a.salt, b.salt);
        assert_ne!(a.nonce, b.nonce);
        assert_eq!(unwrap("senha mestra longa do teste", "a", &a).unwrap(), raw);
        assert!(unwrap("senha incorreta", "a", &a).is_err());
        assert!(unwrap("senha mestra longa do teste", "b", &a).is_err());
        let mut corrupted = a;
        corrupted.ciphertext[0] ^= 1;
        assert!(unwrap("senha mestra longa do teste", "a", &corrupted).is_err());
    }
}
