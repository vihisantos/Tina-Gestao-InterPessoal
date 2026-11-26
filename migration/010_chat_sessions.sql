-- TINA-CHAT-021: histórico local do chat principal, isolado por instalação.
-- A chave NVIDIA nunca é persistida aqui.
CREATE TABLE chat_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    installation_id TEXT NOT NULL REFERENCES installations(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK(length(trim(title)) BETWEEN 1 AND 120),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(id, installation_id)
);
CREATE INDEX idx_chat_sessions_installation ON chat_sessions(installation_id, updated_at DESC);
CREATE TABLE chat_messages (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL,
    installation_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant')),
    content TEXT NOT NULL CHECK(length(content) BETWEEN 1 AND 100000),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (session_id, installation_id) REFERENCES chat_sessions(id, installation_id) ON DELETE RESTRICT
);
CREATE INDEX idx_chat_messages_session ON chat_messages(installation_id, session_id, created_at, id);
