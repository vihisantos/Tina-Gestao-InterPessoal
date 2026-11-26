-- Ciclo de vida explícito das propostas do chat.
-- 016-018 estão reservadas para a PR empilhada de RH; esta migration é aditiva.
-- `status` permanece como coluna legada (pending/approved/rejected) para leitores
-- antigos; `lifecycle_status` é a fonte autoritativa a partir de agora.
ALTER TABLE chat_department_proposals
    ADD COLUMN lifecycle_status TEXT NOT NULL DEFAULT 'pending'
        CHECK (lifecycle_status IN ('pending','approved','rejected','superseded','cancelled'));
ALTER TABLE chat_department_proposals ADD COLUMN superseded_by TEXT;
ALTER TABLE chat_department_proposals ADD COLUMN decided_at TEXT;
ALTER TABLE chat_department_proposals ADD COLUMN cancelled_at TEXT;
ALTER TABLE chat_department_proposals ADD COLUMN cancel_reason TEXT;
ALTER TABLE chat_department_proposals ADD COLUMN updated_at TEXT;

ALTER TABLE chat_organization_proposals
    ADD COLUMN lifecycle_status TEXT NOT NULL DEFAULT 'pending'
        CHECK (lifecycle_status IN ('pending','approved','rejected','superseded','cancelled'));
ALTER TABLE chat_organization_proposals ADD COLUMN superseded_by TEXT;
ALTER TABLE chat_organization_proposals ADD COLUMN decided_at TEXT;
ALTER TABLE chat_organization_proposals ADD COLUMN cancelled_at TEXT;
ALTER TABLE chat_organization_proposals ADD COLUMN cancel_reason TEXT;
ALTER TABLE chat_organization_proposals ADD COLUMN updated_at TEXT;

-- Backfill: decisões já persistidas conservam o mesmo significado.
UPDATE chat_department_proposals SET lifecycle_status = status;
UPDATE chat_organization_proposals SET lifecycle_status = status;

-- Bancos legados podem ter mais de uma proposta pendente na mesma sessão.
-- A mais recente (pelo rowid da mensagem da proposta) continua pendente; as
-- anteriores passam a 'superseded' e não podem mais ser aprovadas. O `status`
-- legado vira 'rejected' apenas para manter o CHECK antigo válido.
WITH ranked AS (
    SELECT p.id AS proposal_id, p.installation_id, p.session_id, m.rowid AS ord
      FROM chat_department_proposals p
      JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
    UNION ALL
    SELECT p.id, p.installation_id, p.session_id, m.rowid
      FROM chat_organization_proposals p
      JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
), winners AS (
    SELECT proposal_id, installation_id, session_id
      FROM (
        SELECT ranked.*,
               ROW_NUMBER() OVER (
                 PARTITION BY installation_id, session_id
                 ORDER BY ord DESC
               ) AS rn
          FROM ranked
      )
     WHERE rn = 1
)
UPDATE chat_department_proposals
   SET status = 'rejected',
       lifecycle_status = 'superseded',
       superseded_by = (
           SELECT w.proposal_id FROM winners w
            WHERE w.installation_id = chat_department_proposals.installation_id
              AND w.session_id = chat_department_proposals.session_id
       ),
       decided_at = datetime('now'),
       updated_at = datetime('now')
 WHERE lifecycle_status = 'pending'
   AND NOT EXISTS (
       SELECT 1 FROM winners w
        WHERE w.proposal_id = chat_department_proposals.id
   );

WITH ranked AS (
    SELECT p.id AS proposal_id, p.installation_id, p.session_id, m.rowid AS ord
      FROM chat_department_proposals p
      JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
    UNION ALL
    SELECT p.id, p.installation_id, p.session_id, m.rowid
      FROM chat_organization_proposals p
      JOIN chat_messages m ON m.id = p.proposal_message_id
     WHERE p.lifecycle_status = 'pending'
), winners AS (
    SELECT proposal_id, installation_id, session_id
      FROM (
        SELECT ranked.*,
               ROW_NUMBER() OVER (
                 PARTITION BY installation_id, session_id
                 ORDER BY ord DESC
               ) AS rn
          FROM ranked
      )
     WHERE rn = 1
)
UPDATE chat_organization_proposals
   SET status = 'rejected',
       lifecycle_status = 'superseded',
       superseded_by = (
           SELECT w.proposal_id FROM winners w
            WHERE w.installation_id = chat_organization_proposals.installation_id
              AND w.session_id = chat_organization_proposals.session_id
       ),
       decided_at = datetime('now'),
       updated_at = datetime('now')
 WHERE lifecycle_status = 'pending'
   AND NOT EXISTS (
       SELECT 1 FROM winners w
        WHERE w.proposal_id = chat_organization_proposals.id
   );

CREATE UNIQUE INDEX uq_chat_department_one_pending
    ON chat_department_proposals(installation_id, session_id)
    WHERE lifecycle_status = 'pending';
CREATE UNIQUE INDEX uq_chat_organization_one_pending
    ON chat_organization_proposals(installation_id, session_id)
    WHERE lifecycle_status = 'pending';

-- Rede de segurança: uma nova proposta substitui a anterior da mesma sessão,
-- inclusive quando ela veio da outra tabela. BEFORE INSERT evita violar o
-- índice parcial de unicidade.
CREATE TRIGGER trg_chat_department_supersede_pending
BEFORE INSERT ON chat_department_proposals
BEGIN
    UPDATE chat_department_proposals
       SET status = 'rejected', lifecycle_status = 'superseded',
           superseded_by = NEW.id, decided_at = datetime('now'),
           updated_at = datetime('now')
     WHERE installation_id = NEW.installation_id
       AND session_id = NEW.session_id
       AND lifecycle_status = 'pending';
    UPDATE chat_organization_proposals
       SET status = 'rejected', lifecycle_status = 'superseded',
           superseded_by = NEW.id, decided_at = datetime('now'),
           updated_at = datetime('now')
     WHERE installation_id = NEW.installation_id
       AND session_id = NEW.session_id
       AND lifecycle_status = 'pending';
END;

CREATE TRIGGER trg_chat_organization_supersede_pending
BEFORE INSERT ON chat_organization_proposals
BEGIN
    UPDATE chat_organization_proposals
       SET status = 'rejected', lifecycle_status = 'superseded',
           superseded_by = NEW.id, decided_at = datetime('now'),
           updated_at = datetime('now')
     WHERE installation_id = NEW.installation_id
       AND session_id = NEW.session_id
       AND lifecycle_status = 'pending';
    UPDATE chat_department_proposals
       SET status = 'rejected', lifecycle_status = 'superseded',
           superseded_by = NEW.id, decided_at = datetime('now'),
           updated_at = datetime('now')
     WHERE installation_id = NEW.installation_id
       AND session_id = NEW.session_id
       AND lifecycle_status = 'pending';
END;
