-- Ciclo de vida unificado das cinco famílias de propostas do chat.
-- A migration 019 cobria apenas departamento e organização; 016-018 criaram
-- cargo, acesso a ferramentas e minuta com o `status` legado. Esta migration é
-- aditiva e não altera migrations já publicadas.
ALTER TABLE chat_position_proposals
    ADD COLUMN lifecycle_status TEXT NOT NULL DEFAULT 'pending'
        CHECK (lifecycle_status IN ('pending','approved','rejected','superseded','cancelled'));
ALTER TABLE chat_position_proposals ADD COLUMN superseded_by TEXT;
ALTER TABLE chat_position_proposals ADD COLUMN decided_at TEXT;
ALTER TABLE chat_position_proposals ADD COLUMN cancelled_at TEXT;
ALTER TABLE chat_position_proposals ADD COLUMN cancel_reason TEXT;
ALTER TABLE chat_position_proposals ADD COLUMN updated_at TEXT;

ALTER TABLE chat_tool_access_proposals
    ADD COLUMN lifecycle_status TEXT NOT NULL DEFAULT 'pending'
        CHECK (lifecycle_status IN ('pending','approved','rejected','superseded','cancelled'));
ALTER TABLE chat_tool_access_proposals ADD COLUMN superseded_by TEXT;
ALTER TABLE chat_tool_access_proposals ADD COLUMN decided_at TEXT;
ALTER TABLE chat_tool_access_proposals ADD COLUMN cancelled_at TEXT;
ALTER TABLE chat_tool_access_proposals ADD COLUMN cancel_reason TEXT;
ALTER TABLE chat_tool_access_proposals ADD COLUMN updated_at TEXT;

ALTER TABLE chat_contract_drafts
    ADD COLUMN lifecycle_status TEXT NOT NULL DEFAULT 'pending'
        CHECK (lifecycle_status IN ('pending','approved','rejected','superseded','cancelled'));
ALTER TABLE chat_contract_drafts ADD COLUMN superseded_by TEXT;
ALTER TABLE chat_contract_drafts ADD COLUMN decided_at TEXT;
ALTER TABLE chat_contract_drafts ADD COLUMN cancelled_at TEXT;
ALTER TABLE chat_contract_drafts ADD COLUMN cancel_reason TEXT;
ALTER TABLE chat_contract_drafts ADD COLUMN updated_at TEXT;

-- Decisões já persistidas conservam o mesmo significado.
UPDATE chat_position_proposals SET lifecycle_status = status;
UPDATE chat_tool_access_proposals SET lifecycle_status = status;
UPDATE chat_contract_drafts SET lifecycle_status = status;

-- Bancos que já aplicaram 016-018 podem ter mais de uma proposta pendente
-- na mesma família e sessão. Mantém apenas a mais recente de cada família;
-- propostas de famílias diferentes permanecem independentes.

WITH ranked AS (
    SELECT p.id AS proposal_id, ROW_NUMBER() OVER (
        PARTITION BY p.installation_id, p.session_id ORDER BY m.rowid DESC, p.id DESC
    ) AS rn
    FROM chat_position_proposals p
    JOIN chat_messages m ON m.id = p.proposal_message_id
    WHERE p.lifecycle_status='pending'
)
UPDATE chat_position_proposals
SET status='rejected', lifecycle_status='superseded',
    superseded_by=(SELECT r.proposal_id FROM ranked r WHERE r.rn=1),
    decided_at=datetime('now'), updated_at=datetime('now')
WHERE lifecycle_status='pending'
  AND id NOT IN (SELECT proposal_id FROM ranked WHERE rn=1);

WITH ranked AS (
    SELECT p.id AS proposal_id, ROW_NUMBER() OVER (
        PARTITION BY p.installation_id, p.session_id ORDER BY m.rowid DESC, p.id DESC
    ) AS rn
    FROM chat_tool_access_proposals p
    JOIN chat_messages m ON m.id = p.proposal_message_id
    WHERE p.lifecycle_status='pending'
)
UPDATE chat_tool_access_proposals
SET status='rejected', lifecycle_status='superseded',
    superseded_by=(SELECT r.proposal_id FROM ranked r WHERE r.rn=1),
    decided_at=datetime('now'), updated_at=datetime('now')
WHERE lifecycle_status='pending'
  AND id NOT IN (SELECT proposal_id FROM ranked WHERE rn=1);

WITH ranked AS (
    SELECT p.id AS proposal_id, ROW_NUMBER() OVER (
        PARTITION BY p.installation_id, p.session_id ORDER BY m.rowid DESC, p.id DESC
    ) AS rn
    FROM chat_contract_drafts p
    JOIN chat_messages m ON m.id = p.proposal_message_id
    WHERE p.lifecycle_status='pending'
)
UPDATE chat_contract_drafts
SET status='rejected', lifecycle_status='superseded',
    superseded_by=(SELECT r.proposal_id FROM ranked r WHERE r.rn=1),
    decided_at=datetime('now'), updated_at=datetime('now')
WHERE lifecycle_status='pending'
  AND id NOT IN (SELECT proposal_id FROM ranked WHERE rn=1);

-- Uma única proposta ativa por instalação e sessão, em cada família.
CREATE UNIQUE INDEX IF NOT EXISTS uq_chat_position_one_pending
    ON chat_position_proposals(installation_id, session_id)
    WHERE lifecycle_status = 'pending';
CREATE UNIQUE INDEX IF NOT EXISTS uq_chat_tool_access_one_pending
    ON chat_tool_access_proposals(installation_id, session_id)
    WHERE lifecycle_status = 'pending';
CREATE UNIQUE INDEX IF NOT EXISTS uq_chat_contract_one_pending
    ON chat_contract_drafts(installation_id, session_id)
    WHERE lifecycle_status = 'pending';

-- Cada família mantém no máximo uma proposta pendente por instalação/sessão.
-- Propostas de famílias diferentes não se substituem automaticamente.
CREATE TRIGGER IF NOT EXISTS trg_chat_position_supersede_same_family
BEFORE INSERT ON chat_position_proposals
BEGIN
    UPDATE chat_position_proposals
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id
       AND lifecycle_status='pending';
END;

CREATE TRIGGER IF NOT EXISTS trg_chat_tool_access_supersede_same_family
BEFORE INSERT ON chat_tool_access_proposals
BEGIN
    UPDATE chat_tool_access_proposals
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id
       AND lifecycle_status='pending';
END;

CREATE TRIGGER IF NOT EXISTS trg_chat_contract_supersede_same_family
BEFORE INSERT ON chat_contract_drafts
BEGIN
    UPDATE chat_contract_drafts
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id
       AND lifecycle_status='pending';
END;
