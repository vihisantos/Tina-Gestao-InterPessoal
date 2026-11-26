-- TINA-ORG-008: estende a autoria da migration 011 sem alterar a versão canônica.
-- Substitui apenas triggers de departamentos, cargos e funcionários.
-- Eventos anteriores e contexto transacional permanecem intactos.

-- Departments triggers with owner attribution
DROP TRIGGER IF EXISTS audit_departments_insert;
DROP TRIGGER IF EXISTS audit_departments_update;
CREATE TRIGGER audit_departments_insert
AFTER INSERT ON departments
BEGIN
    INSERT INTO organizational_audit_events
      (id, installation_id, company_id, entity_type, entity_id, action, outcome, actor_type, actor_id)
    VALUES (lower(hex(randomblob(16))), (SELECT c.installation_id FROM companies c JOIN installations i ON i.id = c.installation_id WHERE c.id = NEW.company_id), NEW.company_id, 'department', NEW.id, 'created', 'success',
      (SELECT 'owner' FROM organizational_audit_actor_context WHERE installation_id = (SELECT c.installation_id FROM companies c JOIN installations i ON i.id = c.installation_id WHERE c.id = NEW.company_id)),
      (SELECT actor_id FROM organizational_audit_actor_context WHERE installation_id = (SELECT c.installation_id FROM companies c JOIN installations i ON i.id = c.installation_id WHERE c.id = NEW.company_id)));
END;
CREATE TRIGGER audit_departments_update
AFTER UPDATE ON departments
WHEN NEW.name IS NOT OLD.name OR NEW.code IS NOT OLD.code OR NEW.description IS NOT OLD.description OR NEW.status IS NOT OLD.status
BEGIN
    INSERT INTO organizational_audit_events
      (id, installation_id, company_id, entity_type, entity_id, action, outcome, actor_type, actor_id)
    VALUES (lower(hex(randomblob(16))), (SELECT c.installation_id FROM companies c JOIN installations i ON i.id = c.installation_id WHERE c.id = NEW.company_id), NEW.company_id, 'department', NEW.id, 'updated', 'success',
      (SELECT 'owner' FROM organizational_audit_actor_context WHERE installation_id = (SELECT c.installation_id FROM companies c JOIN installations i ON i.id = c.installation_id WHERE c.id = NEW.company_id)),
      (SELECT actor_id FROM organizational_audit_actor_context WHERE installation_id = (SELECT c.installation_id FROM companies c JOIN installations i ON i.id = c.installation_id WHERE c.id = NEW.company_id)));
END;

-- Positions triggers with owner attribution
DROP TRIGGER IF EXISTS audit_positions_insert;
DROP TRIGGER IF EXISTS audit_positions_update;
CREATE TRIGGER audit_positions_insert
AFTER INSERT ON positions
BEGIN
    INSERT INTO organizational_audit_events
      (id, installation_id, company_id, entity_type, entity_id, action, outcome, actor_type, actor_id)
    VALUES (lower(hex(randomblob(16))), (SELECT c.installation_id FROM departments d JOIN companies c ON c.id = d.company_id WHERE d.id = NEW.department_id), (SELECT company_id FROM departments WHERE id = NEW.department_id), 'position', NEW.id, 'created', 'success',
      (SELECT 'owner' FROM organizational_audit_actor_context WHERE installation_id = (SELECT c.installation_id FROM departments d JOIN companies c ON c.id = d.company_id WHERE d.id = NEW.department_id)),
      (SELECT actor_id FROM organizational_audit_actor_context WHERE installation_id = (SELECT c.installation_id FROM departments d JOIN companies c ON c.id = d.company_id WHERE d.id = NEW.department_id)));
END;
CREATE TRIGGER audit_positions_update
AFTER UPDATE ON positions
WHEN NEW.name IS NOT OLD.name OR NEW.level IS NOT OLD.level OR NEW.authority_level IS NOT OLD.authority_level OR NEW.description IS NOT OLD.description OR NEW.status IS NOT OLD.status
BEGIN
    INSERT INTO organizational_audit_events
      (id, installation_id, company_id, entity_type, entity_id, action, outcome, actor_type, actor_id)
    VALUES (lower(hex(randomblob(16))), (SELECT c.installation_id FROM departments d JOIN companies c ON c.id = d.company_id WHERE d.id = NEW.department_id), (SELECT company_id FROM departments WHERE id = NEW.department_id), 'position', NEW.id, 'updated', 'success',
      (SELECT 'owner' FROM organizational_audit_actor_context WHERE installation_id = (SELECT c.installation_id FROM departments d JOIN companies c ON c.id = d.company_id WHERE d.id = NEW.department_id)),
      (SELECT actor_id FROM organizational_audit_actor_context WHERE installation_id = (SELECT c.installation_id FROM departments d JOIN companies c ON c.id = d.company_id WHERE d.id = NEW.department_id)));
END;

-- Employees triggers with owner attribution
DROP TRIGGER IF EXISTS audit_employees_insert;
DROP TRIGGER IF EXISTS audit_employees_update;
CREATE TRIGGER audit_employees_insert
AFTER INSERT ON employees
BEGIN
    INSERT INTO organizational_audit_events
      (id, installation_id, company_id, entity_type, entity_id, action, outcome, actor_type, actor_id)
    VALUES (lower(hex(randomblob(16))), (SELECT c.installation_id FROM positions p JOIN departments d ON d.id = p.department_id JOIN companies c ON c.id = d.company_id WHERE p.id = NEW.position_id), (SELECT d.company_id FROM positions p JOIN departments d ON d.id = p.department_id WHERE p.id = NEW.position_id), 'employee', NEW.id, 'created', 'success',
      (SELECT 'owner' FROM organizational_audit_actor_context WHERE installation_id = (SELECT c.installation_id FROM positions p JOIN departments d ON d.id = p.department_id JOIN companies c ON c.id = d.company_id WHERE p.id = NEW.position_id)),
      (SELECT actor_id FROM organizational_audit_actor_context WHERE installation_id = (SELECT c.installation_id FROM positions p JOIN departments d ON d.id = p.department_id JOIN companies c ON c.id = d.company_id WHERE p.id = NEW.position_id)));
END;
CREATE TRIGGER audit_employees_update
AFTER UPDATE ON employees
WHEN NEW.name IS NOT OLD.name OR NEW.position_id IS NOT OLD.position_id OR NEW.responsibilities IS NOT OLD.responsibilities OR NEW.supervisor_id IS NOT OLD.supervisor_id OR NEW.status IS NOT OLD.status OR NEW.is_active IS NOT OLD.is_active OR NEW.deactivated_at IS NOT OLD.deactivated_at
BEGIN
    INSERT INTO organizational_audit_events
      (id, installation_id, company_id, entity_type, entity_id, action, outcome, actor_type, actor_id)
    VALUES (lower(hex(randomblob(16))), (SELECT c.installation_id FROM positions p JOIN departments d ON d.id = p.department_id JOIN companies c ON c.id = d.company_id WHERE p.id = NEW.position_id), (SELECT d.company_id FROM positions p JOIN departments d ON d.id = p.department_id WHERE p.id = NEW.position_id), 'employee', NEW.id, 'updated', 'success',
      (SELECT 'owner' FROM organizational_audit_actor_context WHERE installation_id = (SELECT c.installation_id FROM positions p JOIN departments d ON d.id = p.department_id JOIN companies c ON c.id = d.company_id WHERE p.id = NEW.position_id)),
      (SELECT actor_id FROM organizational_audit_actor_context WHERE installation_id = (SELECT c.installation_id FROM positions p JOIN departments d ON d.id = p.department_id JOIN companies c ON c.id = d.company_id WHERE p.id = NEW.position_id)));
END;
