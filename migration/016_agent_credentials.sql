-- Chaves privadas permanecem no cofre Windows; nunca no SQLite.
CREATE TABLE agent_credentials (
    id TEXT PRIMARY KEY NOT NULL,
    employee_id TEXT NOT NULL REFERENCES employees(id),
    public_key BLOB NOT NULL CHECK(length(public_key) = 32),
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

CREATE TRIGGER agent_credential_active_employee
BEFORE INSERT ON agent_credentials
WHEN NOT EXISTS (
 SELECT 1 FROM employees WHERE id=NEW.employee_id AND status='active'
)
BEGIN SELECT RAISE(ABORT, 'employee is not active'); END;

CREATE TABLE agent_signing_nonces (
 credential_id TEXT NOT NULL REFERENCES agent_credentials(id),
 nonce TEXT NOT NULL CHECK(length(nonce)=36),
 consumed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 PRIMARY KEY(credential_id, nonce)
);
CREATE TRIGGER agent_nonce_active_credential
BEFORE INSERT ON agent_signing_nonces
WHEN NOT EXISTS (
 SELECT 1 FROM agent_credentials c JOIN employees e ON e.id=c.employee_id
 WHERE c.id=NEW.credential_id AND c.revoked_at IS NULL AND e.status='active'
)
BEGIN SELECT RAISE(ABORT, 'credential is not active'); END;
CREATE TRIGGER agent_nonce_no_update BEFORE UPDATE ON agent_signing_nonces
BEGIN SELECT RAISE(ABORT, 'nonce is immutable'); END;
CREATE TRIGGER agent_nonce_no_delete BEFORE DELETE ON agent_signing_nonces
BEGIN SELECT RAISE(ABORT, 'nonce history must be preserved'); END;
