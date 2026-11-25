-- TINA-ORG-008: eventos organizacionais locais, append-only.
-- Triggers garantem que mutações persistidas gerem eventos na mesma transação.
-- Triggers não sabem identificar o ator autenticado: actor_type/id permanecem NULL.
-- Eventos de negação e atribuição de ator exigem integração explícita no backend.
CREATE TABLE organizational_audit_events (
    id TEXT PRIMARY KEY NOT NULL,
    installation_id TEXT NOT NULL,
    company_id TEXT,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    action TEXT NOT NULL,
    actor_type TEXT,
    actor_id TEXT,
    outcome TEXT NOT NULL CHECK (outcome IN ('success', 'denied')),
    occurred_at TEXT NOT NULL DEFAULT (datetime('now')),
    metadata_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(metadata_json))
);
CREATE INDEX idx_organizational_audit_entity
    ON organizational_audit_events (installation_id, entity_type, entity_id, occurred_at);
CREATE INDEX idx_organizational_audit_company
    ON organizational_audit_events (installation_id, company_id, occurred_at);
CREATE TRIGGER organizational_audit_events_reject_update
BEFORE UPDATE ON organizational_audit_events
BEGIN
    SELECT RAISE(ABORT, 'organizational_audit_events is append-only');
END;
CREATE TRIGGER organizational_audit_events_reject_delete
BEFORE DELETE ON organizational_audit_events
BEGIN
    SELECT RAISE(ABORT, 'organizational_audit_events is append-only');
END;

-- companies: registo automático atômico, sem atribuir autor não verificado.
CREATE TRIGGER audit_companies_insert
AFTER INSERT ON companies
BEGIN
    INSERT INTO organizational_audit_events
      (id, installation_id, company_id, entity_type, entity_id, action, outcome)
    VALUES (lower(hex(randomblob(16))), NEW.installation_id, NEW.id, 'company', NEW.id, 'created', 'success');
END;
CREATE TRIGGER audit_companies_update
AFTER UPDATE ON companies
WHEN NEW.name IS NOT OLD.name
   OR NEW.status IS NOT OLD.status
BEGIN
    INSERT INTO organizational_audit_events
      (id, installation_id, company_id, entity_type, entity_id, action, outcome)
    VALUES (lower(hex(randomblob(16))), NEW.installation_id, NEW.id, 'company', NEW.id, 'updated', 'success');
END;

-- departments: registo automático atômico, sem atribuir autor não verificado.
CREATE TRIGGER audit_departments_insert
AFTER INSERT ON departments
BEGIN
    INSERT INTO organizational_audit_events
      (id, installation_id, company_id, entity_type, entity_id, action, outcome)
    VALUES (lower(hex(randomblob(16))), (SELECT installation_id FROM companies WHERE id = NEW.company_id), NEW.company_id, 'department', NEW.id, 'created', 'success');
END;
CREATE TRIGGER audit_departments_update
AFTER UPDATE ON departments
WHEN NEW.name IS NOT OLD.name
   OR NEW.code IS NOT OLD.code
   OR NEW.description IS NOT OLD.description
   OR NEW.status IS NOT OLD.status
BEGIN
    INSERT INTO organizational_audit_events
      (id, installation_id, company_id, entity_type, entity_id, action, outcome)
    VALUES (lower(hex(randomblob(16))), (SELECT installation_id FROM companies WHERE id = NEW.company_id), NEW.company_id, 'department', NEW.id, 'updated', 'success');
END;

-- positions: registo automático atômico, sem atribuir autor não verificado.
CREATE TRIGGER audit_positions_insert
AFTER INSERT ON positions
BEGIN
    INSERT INTO organizational_audit_events
      (id, installation_id, company_id, entity_type, entity_id, action, outcome)
    VALUES (lower(hex(randomblob(16))), (SELECT c.installation_id FROM departments d JOIN companies c ON c.id = d.company_id WHERE d.id = NEW.department_id), (SELECT company_id FROM departments WHERE id = NEW.department_id), 'position', NEW.id, 'created', 'success');
END;
CREATE TRIGGER audit_positions_update
AFTER UPDATE ON positions
WHEN NEW.name IS NOT OLD.name
   OR NEW.level IS NOT OLD.level
   OR NEW.authority_level IS NOT OLD.authority_level
   OR NEW.description IS NOT OLD.description
   OR NEW.status IS NOT OLD.status
BEGIN
    INSERT INTO organizational_audit_events
      (id, installation_id, company_id, entity_type, entity_id, action, outcome)
    VALUES (lower(hex(randomblob(16))), (SELECT c.installation_id FROM departments d JOIN companies c ON c.id = d.company_id WHERE d.id = NEW.department_id), (SELECT company_id FROM departments WHERE id = NEW.department_id), 'position', NEW.id, 'updated', 'success');
END;

-- employees: registo automático atômico, sem atribuir autor não verificado.
CREATE TRIGGER audit_employees_insert
AFTER INSERT ON employees
BEGIN
    INSERT INTO organizational_audit_events
      (id, installation_id, company_id, entity_type, entity_id, action, outcome)
    VALUES (lower(hex(randomblob(16))), (SELECT c.installation_id FROM positions p JOIN departments d ON d.id = p.department_id JOIN companies c ON c.id = d.company_id WHERE p.id = NEW.position_id), (SELECT d.company_id FROM positions p JOIN departments d ON d.id = p.department_id WHERE p.id = NEW.position_id), 'employee', NEW.id, 'created', 'success');
END;
CREATE TRIGGER audit_employees_update
AFTER UPDATE ON employees
WHEN NEW.name IS NOT OLD.name
   OR NEW.position_id IS NOT OLD.position_id
   OR NEW.responsibilities IS NOT OLD.responsibilities
   OR NEW.supervisor_id IS NOT OLD.supervisor_id
   OR NEW.status IS NOT OLD.status
   OR NEW.is_active IS NOT OLD.is_active
   OR NEW.deactivated_at IS NOT OLD.deactivated_at
BEGIN
    INSERT INTO organizational_audit_events
      (id, installation_id, company_id, entity_type, entity_id, action, outcome)
    VALUES (lower(hex(randomblob(16))), (SELECT c.installation_id FROM positions p JOIN departments d ON d.id = p.department_id JOIN companies c ON c.id = d.company_id WHERE p.id = NEW.position_id), (SELECT d.company_id FROM positions p JOIN departments d ON d.id = p.department_id WHERE p.id = NEW.position_id), 'employee', NEW.id, 'updated', 'success');
END;