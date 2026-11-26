-- Contexto de autoria estritamente transacional. O escritor autorizado insere
-- e remove a linha na mesma transação da mutação; SQLite serializa escritores.
-- Não atribuir autor a operações internas sem contexto.
CREATE TABLE organizational_audit_actor_context (
    installation_id TEXT PRIMARY KEY REFERENCES installations(id) ON DELETE RESTRICT,
    actor_id TEXT NOT NULL
);
DROP TRIGGER audit_companies_insert;
DROP TRIGGER audit_companies_update;
CREATE TRIGGER audit_companies_insert
AFTER INSERT ON companies
BEGIN
    INSERT INTO organizational_audit_events
      (id, installation_id, company_id, entity_type, entity_id, action, outcome, actor_type, actor_id)
    VALUES (lower(hex(randomblob(16))), NEW.installation_id, NEW.id, 'company', NEW.id, 'created', 'success',
      (SELECT 'owner' FROM organizational_audit_actor_context WHERE installation_id = NEW.installation_id),
      (SELECT actor_id FROM organizational_audit_actor_context WHERE installation_id = NEW.installation_id));
END;
CREATE TRIGGER audit_companies_update
AFTER UPDATE ON companies
WHEN NEW.name IS NOT OLD.name OR NEW.status IS NOT OLD.status
BEGIN
    INSERT INTO organizational_audit_events
      (id, installation_id, company_id, entity_type, entity_id, action, outcome, actor_type, actor_id)
    VALUES (lower(hex(randomblob(16))), NEW.installation_id, NEW.id, 'company', NEW.id, 'updated', 'success',
      (SELECT 'owner' FROM organizational_audit_actor_context WHERE installation_id = NEW.installation_id),
      (SELECT actor_id FROM organizational_audit_actor_context WHERE installation_id = NEW.installation_id));
END;