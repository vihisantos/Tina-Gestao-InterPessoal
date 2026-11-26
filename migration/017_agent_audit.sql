CREATE TABLE agent_audit_events (
 id TEXT PRIMARY KEY NOT NULL,
 credential_id TEXT NOT NULL REFERENCES agent_credentials(id),
 task_id TEXT NOT NULL,
 requester_id TEXT NOT NULL,
 delegator_id TEXT,
 reviewer_id TEXT,
 model_version TEXT NOT NULL,
 nonce TEXT NOT NULL CHECK(length(nonce)=36),
 action TEXT NOT NULL,
 resource_type TEXT NOT NULL,
 resource_id TEXT NOT NULL,
 outcome TEXT NOT NULL CHECK(outcome IN ('executed','denied','failed')),
 reason TEXT,
 model_id TEXT NOT NULL,
 occurred_at TEXT NOT NULL,
 payload_digest BLOB NOT NULL CHECK(length(payload_digest)=32),
 signature BLOB NOT NULL CHECK(length(signature)=64),
 event_digest BLOB NOT NULL UNIQUE CHECK(length(event_digest)=32),
 previous_digest BLOB CHECK(previous_digest IS NULL OR length(previous_digest)=32),
 UNIQUE(credential_id, nonce),
 CHECK(outcome='executed' OR (reason IS NOT NULL AND length(trim(reason))>0))
);
CREATE TRIGGER agent_audit_no_update BEFORE UPDATE ON agent_audit_events
BEGIN SELECT RAISE(ABORT, 'audit events are append only'); END;
CREATE TRIGGER agent_audit_no_delete BEFORE DELETE ON agent_audit_events
BEGIN SELECT RAISE(ABORT, 'audit events are append only'); END;
CREATE INDEX agent_audit_task ON agent_audit_events(task_id, occurred_at);

CREATE TRIGGER agent_audit_active_credential BEFORE INSERT ON agent_audit_events
WHEN NOT EXISTS (
 SELECT 1 FROM agent_credentials c JOIN employees e ON e.id=c.employee_id
 WHERE c.id=NEW.credential_id AND c.revoked_at IS NULL AND e.status='active'
)
BEGIN SELECT RAISE(ABORT, 'inactive signing credential'); END;

-- Uma cadeia por credencial; impede bifurcações e múltiplas raízes.
CREATE UNIQUE INDEX agent_audit_single_root ON agent_audit_events(credential_id)
WHERE previous_digest IS NULL;
CREATE UNIQUE INDEX agent_audit_single_successor ON agent_audit_events(credential_id,previous_digest)
WHERE previous_digest IS NOT NULL;
CREATE TRIGGER agent_audit_chain_guard BEFORE INSERT ON agent_audit_events
BEGIN
 SELECT CASE WHEN NEW.previous_digest IS NULL AND EXISTS (
 SELECT 1 FROM agent_audit_events WHERE credential_id=NEW.credential_id
 ) THEN RAISE(ABORT,'chain already initialized') END;
 SELECT CASE WHEN NEW.previous_digest IS NOT NULL AND NOT EXISTS (
 SELECT 1 FROM agent_audit_events p WHERE p.credential_id=NEW.credential_id
 AND p.event_digest=NEW.previous_digest
 ) THEN RAISE(ABORT,'unknown chain predecessor') END;
END;
