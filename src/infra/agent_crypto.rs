//! Primitivas internas; autorização e persistência devem preceder a assinatura.
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;

pub(crate) fn generate() -> SigningKey {
    SigningKey::generate(&mut OsRng)
}

pub(crate) fn verify(public: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool {
    VerifyingKey::from_bytes(public)
        .map(|key| key.verify_strict(message, &Signature::from_bytes(signature)).is_ok())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
pub(crate) fn store_key(id: uuid::Uuid, key: &SigningKey) -> Result<(), keyring::Error> {
    keyring::Entry::new("Tina.AgentSigning.v1", &id.to_string())?
        .set_secret(key.as_bytes())
}

#[cfg(target_os = "windows")]
pub(crate) fn load_key(id: uuid::Uuid) -> Result<SigningKey, keyring::Error> {
    let secret = zeroize::Zeroizing::new(
        keyring::Entry::new("Tina.AgentSigning.v1", &id.to_string())?.get_secret()?,
    );
    let bytes = <&[u8; 32]>::try_from(secret.as_slice())
        .map_err(|_| keyring::Error::Invalid("secret".into(), "invalid length".into()))?;
    Ok(SigningKey::from_bytes(bytes))
}

pub(crate) fn sign(key: &SigningKey, message: &[u8]) -> [u8; 64] {
    key.sign(message).to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_modified_message_and_wrong_key() {
        let key = generate();
        let other = generate();
        let signature = sign(&key, b"tina-action-v1");
        assert!(verify(&key.verifying_key().to_bytes(), b"tina-action-v1", &signature));
        assert!(!verify(&key.verifying_key().to_bytes(), b"modified", &signature));
        assert!(!verify(&other.verifying_key().to_bytes(), b"tina-action-v1", &signature));
    }
}
