-- TINA-ORG-008: eventos organizacionais locais, append-only.
-- A migração cria o armazenamento; a integração transacional com os serviços
-- e a resolução de autor confiável são entregas obrigatórias ainda pendentes.
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
