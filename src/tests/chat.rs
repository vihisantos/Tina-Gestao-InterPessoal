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
    let session = chat::create_session(db.pool(), &a.id, "Conversa")
        .await
        .unwrap();
    assert!(chat::create_session(db.pool(), &a.id, " ").await.is_err());
    assert!(
        chat::append_message(db.pool(), &b.id, &session.id, "user", "intrusão")
            .await
            .is_err()
    );
    assert!(
        chat::append_message(db.pool(), &a.id, &session.id, "system", "fake")
            .await
            .is_err()
    );
    assert!(chat::list_sessions(db.pool(), &b.id)
        .await
        .unwrap()
        .is_empty());
    chat::append_message(db.pool(), &a.id, &session.id, "user", "Olá Tina")
        .await
        .unwrap();
    assert!(chat::list_messages(db.pool(), &b.id, &session.id)
        .await
        .unwrap()
        .is_empty());
    drop(db);
    let reopened = Database::open(&path).await.unwrap();
    let messages = chat::list_messages(reopened.pool(), &a.id, &session.id)
        .await
        .unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "Olá Tina");
    drop(reopened);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn chat_history_returns_latest_500_in_chronological_order() {
    let path = std::env::temp_dir().join(format!("tina-chat-window-{}.db", uuid::Uuid::new_v4()));
    let db = Database::open(&path).await.unwrap();
    let installation = InstallationRepository::new(db.pool())
        .create("chat-window", "owner")
        .await
        .unwrap();
    let session = chat::create_session(db.pool(), &installation.id, "Histórico")
        .await
        .unwrap();

    // Mesmo timestamp para todos: a ordem de inserção deve prevalecer
    // sobre UUIDs aleatórios quando o histórico é truncado.
    let mut tx = db.pool().begin().await.unwrap();
    for n in 0..505 {
        sqlx::query(
            "INSERT INTO chat_messages(id, session_id, installation_id, role, content, created_at)
             VALUES (?, ?, ?, 'user', ?, '2026-11-23 10:00:00')",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&session.id)
        .bind(&installation.id)
        .bind(format!("mensagem-{n:03}"))
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    tx.commit().await.unwrap();

    let messages = chat::list_messages(db.pool(), &installation.id, &session.id)
        .await
        .unwrap();
    assert_eq!(messages.len(), 500);
    assert_eq!(messages.first().unwrap().content, "mensagem-005");
    assert_eq!(messages.last().unwrap().content, "mensagem-504");
    assert!(chat::list_messages(db.pool(), "outra-instalacao", &session.id)
        .await
        .unwrap()
        .is_empty());

    drop(db);
    let _ = std::fs::remove_file(path);
}
