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

-- Bancos que já aplicaram 016-018 podem ter mais de uma proposta pendente na
-- mesma sessão, inclusive em famílias diferentes. Mantém apenas a mais recente;
-- as anteriores viram `superseded` e não podem mais ser decididas.
WITH ranked AS (
    SELECT p.id AS proposal_id, p.installation_id, p.session_id, m.rowid AS ord
      FROM chat_department_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
    UNION ALL
    SELECT p.id, p.installation_id, p.session_id, m.rowid
      FROM chat_organization_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
    UNION ALL
    SELECT p.id, p.installation_id, p.session_id, m.rowid
      FROM chat_position_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
    UNION ALL
    SELECT p.id, p.installation_id, p.session_id, m.rowid
      FROM chat_tool_access_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
    UNION ALL
    SELECT p.id, p.installation_id, p.session_id, m.rowid
      FROM chat_contract_drafts p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
), winners AS (
    SELECT proposal_id, installation_id, session_id
      FROM (
        SELECT ranked.*,
               ROW_NUMBER() OVER (
                 PARTITION BY installation_id, session_id
                 ORDER BY ord DESC, proposal_id DESC
               ) AS rn
          FROM ranked
      )
     WHERE rn = 1
)
UPDATE chat_department_proposals
   SET status = 'rejected', lifecycle_status = 'superseded',
       superseded_by = (SELECT w.proposal_id FROM winners w
                         WHERE w.installation_id = chat_department_proposals.installation_id
                           AND w.session_id = chat_department_proposals.session_id),
       decided_at = datetime('now'), updated_at = datetime('now')
 WHERE lifecycle_status = 'pending'
   AND NOT EXISTS (SELECT 1 FROM winners w WHERE w.proposal_id = chat_department_proposals.id);

WITH ranked AS (
    SELECT p.id AS proposal_id, p.installation_id, p.session_id, m.rowid AS ord
      FROM chat_department_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
    UNION ALL
    SELECT p.id, p.installation_id, p.session_id, m.rowid
      FROM chat_organization_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
    UNION ALL
    SELECT p.id, p.installation_id, p.session_id, m.rowid
      FROM chat_position_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
    UNION ALL
    SELECT p.id, p.installation_id, p.session_id, m.rowid
      FROM chat_tool_access_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
    UNION ALL
    SELECT p.id, p.installation_id, p.session_id, m.rowid
      FROM chat_contract_drafts p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
), winners AS (
    SELECT proposal_id, installation_id, session_id
      FROM (
        SELECT ranked.*,
               ROW_NUMBER() OVER (
                 PARTITION BY installation_id, session_id
                 ORDER BY ord DESC, proposal_id DESC
               ) AS rn
          FROM ranked
      )
     WHERE rn = 1
)
UPDATE chat_organization_proposals
   SET status = 'rejected', lifecycle_status = 'superseded',
       superseded_by = (SELECT w.proposal_id FROM winners w
                         WHERE w.installation_id = chat_organization_proposals.installation_id
                           AND w.session_id = chat_organization_proposals.session_id),
       decided_at = datetime('now'), updated_at = datetime('now')
 WHERE lifecycle_status = 'pending'
   AND NOT EXISTS (SELECT 1 FROM winners w WHERE w.proposal_id = chat_organization_proposals.id);

WITH ranked AS (
    SELECT p.id AS proposal_id, p.installation_id, p.session_id, m.rowid AS ord
      FROM chat_department_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
    UNION ALL
    SELECT p.id, p.installation_id, p.session_id, m.rowid
      FROM chat_organization_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
    UNION ALL
    SELECT p.id, p.installation_id, p.session_id, m.rowid
      FROM chat_position_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
    UNION ALL
    SELECT p.id, p.installation_id, p.session_id, m.rowid
      FROM chat_tool_access_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
    UNION ALL
    SELECT p.id, p.installation_id, p.session_id, m.rowid
      FROM chat_contract_drafts p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
), winners AS (
    SELECT proposal_id, installation_id, session_id
      FROM (
        SELECT ranked.*,
               ROW_NUMBER() OVER (
                 PARTITION BY installation_id, session_id
                 ORDER BY ord DESC, proposal_id DESC
               ) AS rn
          FROM ranked
      )
     WHERE rn = 1
)
UPDATE chat_position_proposals
   SET status = 'rejected', lifecycle_status = 'superseded',
       superseded_by = (SELECT w.proposal_id FROM winners w
                         WHERE w.installation_id = chat_position_proposals.installation_id
                           AND w.session_id = chat_position_proposals.session_id),
       decided_at = datetime('now'), updated_at = datetime('now')
 WHERE lifecycle_status = 'pending'
   AND NOT EXISTS (SELECT 1 FROM winners w WHERE w.proposal_id = chat_position_proposals.id);

WITH ranked AS (
    SELECT p.id AS proposal_id, p.installation_id, p.session_id, m.rowid AS ord
      FROM chat_department_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
    UNION ALL
    SELECT p.id, p.installation_id, p.session_id, m.rowid
      FROM chat_organization_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
    UNION ALL
    SELECT p.id, p.installation_id, p.session_id, m.rowid
      FROM chat_position_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
    UNION ALL
    SELECT p.id, p.installation_id, p.session_id, m.rowid
      FROM chat_tool_access_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
    UNION ALL
    SELECT p.id, p.installation_id, p.session_id, m.rowid
      FROM chat_contract_drafts p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
), winners AS (
    SELECT proposal_id, installation_id, session_id
      FROM (
        SELECT ranked.*,
               ROW_NUMBER() OVER (
                 PARTITION BY installation_id, session_id
                 ORDER BY ord DESC, proposal_id DESC
               ) AS rn
          FROM ranked
      )
     WHERE rn = 1
)
UPDATE chat_tool_access_proposals
   SET status = 'rejected', lifecycle_status = 'superseded',
       superseded_by = (SELECT w.proposal_id FROM winners w
                         WHERE w.installation_id = chat_tool_access_proposals.installation_id
                           AND w.session_id = chat_tool_access_proposals.session_id),
       decided_at = datetime('now'), updated_at = datetime('now')
 WHERE lifecycle_status = 'pending'
   AND NOT EXISTS (SELECT 1 FROM winners w WHERE w.proposal_id = chat_tool_access_proposals.id);

WITH ranked AS (
    SELECT p.id AS proposal_id, p.installation_id, p.session_id, m.rowid AS ord
      FROM chat_department_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
    UNION ALL
    SELECT p.id, p.installation_id, p.session_id, m.rowid
      FROM chat_organization_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
    UNION ALL
    SELECT p.id, p.installation_id, p.session_id, m.rowid
      FROM chat_position_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
    UNION ALL
    SELECT p.id, p.installation_id, p.session_id, m.rowid
      FROM chat_tool_access_proposals p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
    UNION ALL
    SELECT p.id, p.installation_id, p.session_id, m.rowid
      FROM chat_contract_drafts p JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
), winners AS (
    SELECT proposal_id, installation_id, session_id
      FROM (
        SELECT ranked.*,
               ROW_NUMBER() OVER (
                 PARTITION BY installation_id, session_id
                 ORDER BY ord DESC, proposal_id DESC
               ) AS rn
          FROM ranked
      )
     WHERE rn = 1
)
UPDATE chat_contract_drafts
   SET status = 'rejected', lifecycle_status = 'superseded',
       superseded_by = (SELECT w.proposal_id FROM winners w
                         WHERE w.installation_id = chat_contract_drafts.installation_id
                           AND w.session_id = chat_contract_drafts.session_id),
       decided_at = datetime('now'), updated_at = datetime('now')
 WHERE lifecycle_status = 'pending'
   AND NOT EXISTS (SELECT 1 FROM winners w WHERE w.proposal_id = chat_contract_drafts.id);

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

-- Toda nova proposta substitui a anterior em qualquer uma das cinco famílias.
-- Os gatilhos da 019 continuam válidos para departamento/organização; estes
-- cobrem as três famílias novas e fecham a malha cruzada.
CREATE TRIGGER IF NOT EXISTS trg_chat_position_supersede_all
BEFORE INSERT ON chat_position_proposals
BEGIN
    UPDATE chat_department_proposals
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id AND lifecycle_status='pending';
    UPDATE chat_organization_proposals
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id AND lifecycle_status='pending';
    UPDATE chat_position_proposals
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id AND lifecycle_status='pending';
    UPDATE chat_tool_access_proposals
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id AND lifecycle_status='pending';
    UPDATE chat_contract_drafts
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id AND lifecycle_status='pending';
END;

CREATE TRIGGER IF NOT EXISTS trg_chat_tool_access_supersede_all
BEFORE INSERT ON chat_tool_access_proposals
BEGIN
    UPDATE chat_department_proposals
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id AND lifecycle_status='pending';
    UPDATE chat_organization_proposals
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id AND lifecycle_status='pending';
    UPDATE chat_position_proposals
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id AND lifecycle_status='pending';
    UPDATE chat_tool_access_proposals
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id AND lifecycle_status='pending';
    UPDATE chat_contract_drafts
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id AND lifecycle_status='pending';
END;

CREATE TRIGGER IF NOT EXISTS trg_chat_contract_supersede_all
BEFORE INSERT ON chat_contract_drafts
BEGIN
    UPDATE chat_department_proposals
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id AND lifecycle_status='pending';
    UPDATE chat_organization_proposals
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id AND lifecycle_status='pending';
    UPDATE chat_position_proposals
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id AND lifecycle_status='pending';
    UPDATE chat_tool_access_proposals
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id AND lifecycle_status='pending';
    UPDATE chat_contract_drafts
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id AND lifecycle_status='pending';
END;

-- Departamento e organização também precisam substituir as três famílias novas.
CREATE TRIGGER IF NOT EXISTS trg_chat_department_supersede_rh
BEFORE INSERT ON chat_department_proposals
BEGIN
    UPDATE chat_position_proposals
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id AND lifecycle_status='pending';
    UPDATE chat_tool_access_proposals
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id AND lifecycle_status='pending';
    UPDATE chat_contract_drafts
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id AND lifecycle_status='pending';
END;

CREATE TRIGGER IF NOT EXISTS trg_chat_organization_supersede_rh
BEFORE INSERT ON chat_organization_proposals
BEGIN
    UPDATE chat_position_proposals
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id AND lifecycle_status='pending';
    UPDATE chat_tool_access_proposals
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id AND lifecycle_status='pending';
    UPDATE chat_contract_drafts
       SET status='rejected', lifecycle_status='superseded', superseded_by=NEW.id,
           decided_at=datetime('now'), updated_at=datetime('now')
     WHERE installation_id=NEW.installation_id AND session_id=NEW.session_id AND lifecycle_status='pending';
END;
