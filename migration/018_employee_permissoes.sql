CREATE TABLE agent_permission_grants (
 id TEXT PRIMARY KEY NOT NULL,
 employee_id TEXT NOT NULL REFERENCES employees(id),
 tool TEXT NOT NULL CHECK(length(trim(tool))>0),
 resource_scope TEXT NOT NULL CHECK(length(trim(resource_scope))>0),
 approved_by TEXT NOT NULL,
 granted_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 revoked_at TEXT
);
CREATE UNIQUE INDEX agent_permission_active ON agent_permission_grants(employee_id,tool,resource_scope) WHERE revoked_at IS NULL;
CREATE TRIGGER agent_permissions_revoke AFTER UPDATE OF status ON employees WHEN NEW.status='inactive' BEGIN
 UPDATE agent_permission_grants SET revoked_at=CURRENT_TIMESTAMP WHERE employee_id=NEW.id AND revoked_at IS NULL;
END;
