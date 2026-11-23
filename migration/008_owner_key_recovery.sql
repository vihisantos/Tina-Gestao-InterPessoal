-- Chave do cofre cifrada com senha mestre por AES-256-GCM/Argon2id.
-- Backup é portável junto ao SQLite; SQLite em si ainda NÃO é criptografado.
CREATE TABLE IF NOT EXISTS owner_key_recovery (
  installation_id TEXT PRIMARY KEY NOT NULL REFERENCES owner_accounts(installation_id) ON DELETE CASCADE,
  salt BLOB NOT NULL CHECK(length(salt) = 16),
  nonce BLOB NOT NULL CHECK(length(nonce) = 12),
  ciphertext BLOB NOT NULL CHECK(length(ciphertext) = 48),
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
