-- A senha de login é um hash Argon2id. A chave do banco fica SOMENTE no
-- Windows Credential Manager. Este schema NÃO criptografa o SQLite.
CREATE TABLE IF NOT EXISTS owner_accounts (
    installation_id TEXT PRIMARY KEY NOT NULL REFERENCES installations(id) ON DELETE CASCADE,
    owner_uuid TEXT NOT NULL,
    display_name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    failed_attempts INTEGER NOT NULL DEFAULT 0 CHECK (failed_attempts >= 0),
    locked_until INTEGER NOT NULL DEFAULT 0 CHECK (locked_until >= 0),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
