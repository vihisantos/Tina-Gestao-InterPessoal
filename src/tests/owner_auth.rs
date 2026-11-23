//! Testes offline do primeiro acesso sem depender do cofre Windows.

use crate::core::authorization::Operation;
use crate::TinaApp;
use uuid::Uuid;

#[tokio::test]
async fn owner_is_locked_before_registration_and_weak_password_is_rejected() {
    let path = std::env::temp_dir().join(format!("tina-owner-test-{}.db", Uuid::new_v4()));
    let app = TinaApp::initialize_at(&path, "owner-first-run", "owner-uuid")
        .await
        .expect("SQLite de teste deveria inicializar");

    let status = app.owner_status().await.unwrap();
    assert!(!status.configured);
    assert!(!status.unlocked);
    assert_eq!(status.display_name, None);
    assert!(app.require_owner(Operation::Read).is_err());
    assert!(app.require_owner(Operation::Write).is_err());
    assert!(app.create_owner_account("A", "123").await.is_err());
    assert!(!app.owner_status().await.unwrap().configured);
    assert!(app.sign_in_owner("123").await.is_err());
}

#[cfg(windows)]
#[tokio::test]
#[ignore = "Teste manual: requer Windows Credential Manager disponível no perfil da sessão"]
async fn windows_vault_creates_256_bit_key_and_requires_password_after_restart() {
    let identity = Uuid::new_v4().to_string();
    let path = std::env::temp_dir().join(format!("tina-owner-vault-{}.db", identity));
    let app = TinaApp::initialize_at(&path, &identity, "owner-id")
        .await
        .unwrap();

    app.create_owner_account("Proprietário", "UmaSenhaForteLocal-2026!")
        .await
        .unwrap();
    assert!(app.owner_status().await.unwrap().configured);
    assert!(!app.owner_unlocked().unwrap());
    assert!(app.sign_in_owner("SenhaErrada").await.is_err());
    assert!(app.require_owner(Operation::Read).is_err());
    app.sign_in_owner("UmaSenhaForteLocal-2026!").await.unwrap();
    assert!(app.require_owner(Operation::Write).is_ok());
    assert!(app.require_owner(Operation::ChangeOwner).is_err());

    let entry = keyring::Entry::new("Tina/owner/database-key/v1", &identity).unwrap();
    let key = entry.get_password().unwrap();
    assert_eq!(key.len(), 64);
    assert!(key.bytes().all(|byte| byte.is_ascii_hexdigit()));
    app.lock_owner().unwrap();
    assert!(!app.owner_unlocked().unwrap());

    drop(app);
    let reopened = TinaApp::initialize_at(&path, &identity, "owner-id")
        .await
        .unwrap();
    assert!(reopened.owner_status().await.unwrap().configured);
    assert!(!reopened.owner_unlocked().unwrap());
    reopened
        .sign_in_owner("UmaSenhaForteLocal-2026!")
        .await
        .unwrap();
    assert!(reopened.require_owner(Operation::Read).is_ok());
    // Simula perda do perfil Windows: recuperar do backup AES-GCM no SQLite
    // usando a senha mestre, sem nunca retornar a chave pelo IPC.
    entry.delete_credential().unwrap();
    reopened.lock_owner().unwrap();
    reopened
        .sign_in_owner("UmaSenhaForteLocal-2026!")
        .await
        .unwrap();
    assert_eq!(entry.get_password().unwrap(), key);
    entry.delete_credential().unwrap();
}
