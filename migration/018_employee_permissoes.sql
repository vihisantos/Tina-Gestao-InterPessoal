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

CREATE TRIGGER agent_permission_active_employee BEFORE INSERT ON agent_permission_grants
WHEN NOT EXISTS (SELECT 1 FROM employees WHERE id=NEW.employee_id AND status='active')
BEGIN SELECT RAISE(ABORT,'inactive employee'); END;
CREATE TRIGGER agent_permission_immutable BEFORE UPDATE OF id,employee_id,tool,resource_scope,approved_by,granted_at ON agent_permission_grants
BEGIN SELECT RAISE(ABORT,'grant is immutable'); END;
CREATE TRIGGER agent_permission_no_reactivate BEFORE UPDATE OF revoked_at ON agent_permission_grants
WHEN OLD.revoked_at IS NOT NULL
BEGIN SELECT RAISE(ABORT,'revocation is permanent'); END;
CREATE TRIGGER agent_permission_preserve BEFORE DELETE ON agent_permission_grants
BEGIN SELECT RAISE(ABORT,'preserve grant history'); END;
CREATE VIEW active_agent_permissions AS
SELECT g.*, c.installation_id FROM agent_permission_grants g
JOIN employees e ON e.id=g.employee_id
JOIN positions p ON p.id=e.position_id
JOIN departments d ON d.id=p.department_id
JOIN companies c ON c.id=d.company_id
WHERE g.revoked_at IS NULL AND e.status='active' AND p.status='active'
AND d.status='active' AND c.status='active';
