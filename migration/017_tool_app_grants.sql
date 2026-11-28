-- Concessões de ferramentas são específicas por funcionário e escopo.
CREATE TABLE employee_tool_grants (
    installation_id TEXT NOT NULL REFERENCES installations(id) ON DELETE CASCADE,
    employee_id TEXT NOT NULL REFERENCES employees(id) ON DELETE CASCADE,
    company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    department_id TEXT NOT NULL REFERENCES departments(id) ON DELETE CASCADE,
    tool_key TEXT NOT NULL,
    granted_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY(installation_id,employee_id,tool_key)
);
CREATE TABLE chat_tool_access_proposals (
    id TEXT PRIMARY KEY NOT NULL,
    installation_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    user_message_id TEXT NOT NULL UNIQUE REFERENCES chat_messages(id) ON DELETE RESTRICT,
    proposal_message_id TEXT NOT NULL UNIQUE REFERENCES chat_messages(id) ON DELETE RESTRICT,
    employee_id TEXT NOT NULL REFERENCES employees(id) ON DELETE RESTRICT,
    employee_name TEXT NOT NULL,
    company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE RESTRICT,
    company_name TEXT NOT NULL,
    organization_kind TEXT NOT NULL CHECK(organization_kind IN ('personal','business')),
    department_id TEXT NOT NULL REFERENCES departments(id) ON DELETE RESTRICT,
    department_name TEXT NOT NULL,
    action TEXT NOT NULL CHECK(action IN ('assign_tool','revoke_tool','assign_app','revoke_app')),
    tool_keys TEXT NOT NULL CHECK(json_valid(tool_keys)),
    reason TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','approved','rejected')),
    result_message_id TEXT REFERENCES chat_messages(id) ON DELETE RESTRICT,
    FOREIGN KEY(session_id,installation_id) REFERENCES chat_sessions(id,installation_id) ON DELETE RESTRICT
);
CREATE INDEX idx_chat_tool_access_scope ON chat_tool_access_proposals(installation_id,session_id,status);

-- Mudanças organizacionais não podem reativar uma concessão antiga silenciosamente.
CREATE TRIGGER tool_grants_employee_scope_change AFTER UPDATE OF position_id,status ON employees
WHEN NEW.position_id IS NOT OLD.position_id OR NEW.status IS NOT OLD.status
BEGIN
    DELETE FROM employee_tool_grants WHERE employee_id=NEW.id;
END;
CREATE TRIGGER tool_grants_position_scope_change AFTER UPDATE OF department_id,status ON positions
WHEN NEW.department_id IS NOT OLD.department_id OR NEW.status IS NOT OLD.status
BEGIN
    DELETE FROM employee_tool_grants WHERE employee_id IN (SELECT id FROM employees WHERE position_id=NEW.id);
END;
CREATE TRIGGER tool_grants_department_scope_change AFTER UPDATE OF company_id,status ON departments
WHEN NEW.company_id IS NOT OLD.company_id OR NEW.status IS NOT OLD.status
BEGIN
    DELETE FROM employee_tool_grants WHERE department_id=NEW.id;
END;
CREATE TRIGGER tool_grants_company_scope_change AFTER UPDATE OF status,installation_id ON companies
WHEN NEW.status IS NOT OLD.status OR NEW.installation_id IS NOT OLD.installation_id
BEGIN
    DELETE FROM employee_tool_grants WHERE company_id=NEW.id;
END;
