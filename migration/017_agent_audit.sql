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
