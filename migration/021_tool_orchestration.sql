-- Motor central de orquestração de ferramentas.
-- Migration aditiva: nenhuma migration publicada é alterada.
--
-- Um plano é a unidade de autorização. Suas etapas são imutáveis após a
-- decisão; apenas estado, resultados e auditoria evoluem. A existência de um
-- plano ativo por sessão impede que uma nova mensagem substitua silenciosamente
-- trabalho pendente ou em execução.

ALTER TABLE chat_messages ADD COLUMN tool_call_id TEXT;
ALTER TABLE chat_messages ADD COLUMN tool_calls_json TEXT;
CREATE INDEX IF NOT EXISTS idx_chat_messages_tool_call
    ON chat_messages(installation_id, session_id, tool_call_id)
    WHERE tool_call_id IS NOT NULL;

CREATE TABLE chat_tool_plans (
    id TEXT PRIMARY KEY NOT NULL,
    installation_id TEXT NOT NULL REFERENCES installations(id) ON DELETE RESTRICT,
    session_id TEXT NOT NULL,
    user_message_id TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN (
        'draft','awaiting_authorization','authorized','running',
        'completed','failed','cancelled','superseded'
    )),
    fingerprint TEXT NOT NULL,
    authorized_fingerprint TEXT,
    calls_json TEXT NOT NULL,
    steps_count INTEGER NOT NULL CHECK (steps_count BETWEEN 1 AND 8),
    max_calls INTEGER NOT NULL DEFAULT 8,
    max_rounds INTEGER NOT NULL DEFAULT 4,
    max_tokens_estimate INTEGER NOT NULL DEFAULT 16000,
    tokens_estimate INTEGER NOT NULL DEFAULT 0,
    rounds INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    decided_at TEXT,
    finished_at TEXT,
    cancel_reason TEXT,
    last_error TEXT,
    superseded_by TEXT,
    UNIQUE(id, installation_id),
    FOREIGN KEY (session_id, installation_id)
        REFERENCES chat_sessions(id, installation_id) ON DELETE RESTRICT
);

CREATE UNIQUE INDEX uq_chat_tool_plan_active
    ON chat_tool_plans(installation_id, session_id)
    WHERE state IN ('draft','awaiting_authorization','authorized','running');
CREATE INDEX idx_chat_tool_plans_session
    ON chat_tool_plans(installation_id, session_id, created_at DESC);

CREATE TABLE chat_tool_plan_steps (
    id TEXT PRIMARY KEY NOT NULL,
    plan_id TEXT NOT NULL REFERENCES chat_tool_plans(id) ON DELETE RESTRICT,
    installation_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    step_index INTEGER NOT NULL,
    tool_call_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    arguments_json TEXT NOT NULL,
    depends_on_json TEXT NOT NULL,
    effects_json TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN (
        'planned','authorized','running','executed','failed','skipped','cancelled'
    )),
    attempts INTEGER NOT NULL DEFAULT 0,
    result_json TEXT,
    result_message_id TEXT,
    proposal_message_id TEXT,
    error TEXT,
    started_at TEXT,
    finished_at TEXT,
    UNIQUE(plan_id, step_index),
    UNIQUE(plan_id, idempotency_key),
    UNIQUE(plan_id, tool_call_id),
    FOREIGN KEY (session_id, installation_id)
        REFERENCES chat_sessions(id, installation_id) ON DELETE RESTRICT
);
CREATE INDEX idx_chat_tool_plan_steps_plan
    ON chat_tool_plan_steps(plan_id, step_index);

CREATE TABLE chat_tool_plan_events (
    id TEXT PRIMARY KEY NOT NULL,
    plan_id TEXT NOT NULL REFERENCES chat_tool_plans(id) ON DELETE RESTRICT,
    installation_id TEXT NOT NULL,
    action TEXT NOT NULL,
    detail_json TEXT NOT NULL,
    occurred_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_chat_tool_plan_events_plan
    ON chat_tool_plan_events(plan_id, occurred_at);

-- Um único cartão de plano é exibido por sessão. Planos concluídos, falhados,
-- cancelados ou substituídos não bloqueiam o próximo turno.
CREATE VIEW vw_chat_tool_active_plan AS
    SELECT id, installation_id, session_id, state, fingerprint, calls_json,
           steps_count, max_calls, max_rounds, max_tokens_estimate,
           tokens_estimate, rounds, created_at, updated_at
      FROM chat_tool_plans
     WHERE state IN ('draft','awaiting_authorization','authorized','running');
