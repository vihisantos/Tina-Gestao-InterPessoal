-- Propostas administrativas do chat: não são ações executadas até aprovação local.
CREATE TABLE chat_department_proposals (
    id TEXT PRIMARY KEY NOT NULL,
    installation_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    user_message_id TEXT NOT NULL UNIQUE REFERENCES chat_messages(id) ON DELETE RESTRICT,
    proposal_message_id TEXT NOT NULL UNIQUE REFERENCES chat_messages(id) ON DELETE RESTRICT,
    company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE RESTRICT,
    company_name TEXT NOT NULL,
    organization_kind TEXT NOT NULL CHECK (organization_kind IN ('personal', 'business')),
    name TEXT NOT NULL,
    code TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','approved','rejected')),
    result_message_id TEXT REFERENCES chat_messages(id) ON DELETE RESTRICT,
    FOREIGN KEY(session_id, installation_id) REFERENCES chat_sessions(id, installation_id) ON DELETE RESTRICT
);
CREATE INDEX idx_chat_department_proposal_scope
    ON chat_department_proposals(installation_id, session_id, status);
