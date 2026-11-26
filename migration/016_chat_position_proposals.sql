-- Cargo é uma proposta independente; não contrata nem concede permissões.
CREATE TABLE chat_position_proposals (
    id TEXT PRIMARY KEY NOT NULL,
    installation_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    user_message_id TEXT NOT NULL UNIQUE REFERENCES chat_messages(id) ON DELETE RESTRICT,
    proposal_message_id TEXT NOT NULL UNIQUE REFERENCES chat_messages(id) ON DELETE RESTRICT,
    company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE RESTRICT,
    company_name TEXT NOT NULL,
    organization_kind TEXT NOT NULL CHECK(organization_kind IN ('personal','business')),
    department_id TEXT NOT NULL REFERENCES departments(id) ON DELETE RESTRICT,
    department_name TEXT NOT NULL,
    name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 120),
    level INTEGER NOT NULL CHECK(level >= 0),
    description TEXT NOT NULL CHECK(length(trim(description)) BETWEEN 1 AND 2000),
    reason TEXT NOT NULL CHECK(length(trim(reason)) BETWEEN 1 AND 500),
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','approved','rejected')),
    result_message_id TEXT REFERENCES chat_messages(id) ON DELETE RESTRICT,
    FOREIGN KEY(session_id,installation_id) REFERENCES chat_sessions(id,installation_id) ON DELETE RESTRICT
);
CREATE INDEX idx_chat_position_proposal_scope
    ON chat_position_proposals(installation_id,session_id,status);
