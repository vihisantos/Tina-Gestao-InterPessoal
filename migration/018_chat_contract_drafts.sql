-- Uma minuta aprovada não é uma contratação nem uma concessão de acesso.
CREATE TABLE chat_contract_drafts (
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
    position_id TEXT NOT NULL REFERENCES positions(id) ON DELETE RESTRICT,
    position_name TEXT NOT NULL,
    name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 120),
    description TEXT NOT NULL,
    reason TEXT NOT NULL CHECK(length(trim(reason)) BETWEEN 1 AND 500),
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','approved','rejected')),
    result_message_id TEXT REFERENCES chat_messages(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY(session_id,installation_id) REFERENCES chat_sessions(id,installation_id) ON DELETE RESTRICT
);
CREATE INDEX idx_chat_contract_drafts_scope ON chat_contract_drafts(installation_id,session_id,status);
