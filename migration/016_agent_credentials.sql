-- Chaves privadas permanecem no cofre Windows; nunca no SQLite.
CREATE TABLE agent_credentials (
    id TEXT PRIMARY KEY NOT NULL,
    employee_id TEXT NOT NULL REFERENCES employees(id),
    public_key BLOB NOT NULL CHECK(length(public_key) > 0),
    algorithm TEXT NOT NULL CHECK(algorithm = 'Ed25519'),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    revoked_at TEXT,
    UNIQUE(public_key)
);
CREATE UNIQUE INDEX agent_one_active_credential ON agent_credentials(employee_id)
WHERE revoked_at IS NULL;
CREATE TRIGGER agent_credential_identity_immutable
BEFORE UPDATE OF id, employee_id, public_key, algorithm, created_at ON agent_credentials
BEGIN SELECT RAISE(ABORT, 'credential identity is immutable'); END;
CREATE TRIGGER agent_credential_no_reactivate
BEFORE UPDATE OF revoked_at ON agent_credentials
WHEN OLD.revoked_at IS NOT NULL
BEGIN SELECT RAISE(ABORT, 'revocation is permanent'); END;
CREATE TRIGGER agent_credential_preserve BEFORE DELETE ON agent_credentials
BEGIN SELECT RAISE(ABORT, 'preserve verification history'); END;
CREATE TRIGGER agent_credential_revoke_on_deactivation
AFTER UPDATE OF status ON employees
WHEN NEW.status = 'inactive'
BEGIN
 UPDATE agent_credentials SET revoked_at = CURRENT_TIMESTAMP
 WHERE employee_id = NEW.id AND revoked_at IS NULL;
END;
