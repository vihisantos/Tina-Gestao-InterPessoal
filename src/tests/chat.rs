//! Persistência e isolamento da fundação do chat.
use crate::infra::{chat, InstallationRepository};
use crate::Database;

#[tokio::test]
async fn chat_isolation_and_persistence() {
    let path = std::env::temp_dir().join(format!("tina-chat-{}.db", uuid::Uuid::new_v4()));
    let db = Database::open(&path).await.unwrap();
    let repo = InstallationRepository::new(db.pool());
    let a = repo.create("chat-a", "owner-a").await.unwrap();
    let b = repo.create("chat-b", "owner-b").await.unwrap();
    let session = chat::create_session(db.pool(), &a.id, "Conversa").await.unwrap();
    assert!(chat::create_session(db.pool(), &a.id, " ").await.is_err());
    assert!(chat::append_message(db.pool(), &b.id, &session.id, "user", "intrusão").await.is_err());
    assert!(chat::append_message(db.pool(), &a.id, &session.id, "system", "fake").await.is_err());
    assert!(chat::list_sessions(db.pool(), &b.id).await.unwrap().is_empty());
    chat::append_message(db.pool(), &a.id, &session.id, "user", "Olá Tina").await.unwrap();
    assert!(chat::list_messages(db.pool(), &b.id, &session.id).await.unwrap().is_empty());
    drop(db);
    let reopened = Database::open(&path).await.unwrap();
    let messages = chat::list_messages(reopened.pool(), &a.id, &session.id).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "Olá Tina");
    drop(reopened);
    let _ = std::fs::remove_file(path);
}
