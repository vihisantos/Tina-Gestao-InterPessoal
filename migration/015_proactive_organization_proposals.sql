-- Iniciativa da Tina: proposta de organização e justificativa, sem escrita automática.
ALTER TABLE chat_department_proposals ADD COLUMN reason TEXT;
CREATE TABLE chat_organization_proposals (
    id TEXT PRIMARY KEY NOT NULL,
    installation_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    user_message_id TEXT NOT NULL UNIQUE REFERENCES chat_messages(id) ON DELETE RESTRICT,
    proposal_message_id TEXT NOT NULL UNIQUE REFERENCES chat_messages(id) ON DELETE RESTRICT,
    name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 160),
    organization_kind TEXT NOT NULL CHECK(organization_kind IN ('personal','business')),
    reason TEXT NOT NULL CHECK(length(trim(reason)) BETWEEN 1 AND 500),
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','approved','rejected')),
    result_message_id TEXT REFERENCES chat_messages(id) ON DELETE RESTRICT,
    FOREIGN KEY(session_id,installation_id) REFERENCES chat_sessions(id,installation_id) ON DELETE RESTRICT
);
CREATE INDEX idx_chat_organization_proposal_scope
    ON chat_organization_proposals(installation_id,session_id,status);
