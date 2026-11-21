-- Tina Platform — migration 005: histórico de relações hierárquicas
--
-- A relação atual continua sendo `employees.supervisor_id` (única fonte de
-- verdade do presente). Esta tabela é um log append-only das alterações:
-- atribuição, troca e remoção de superior direto.
--
-- Regras:
-- - Nenhum evento é removido em cascata quando um funcionário é atualizado.
-- - `ON DELETE RESTRICT` em `employee_id` impede apagar o funcionário e perder
--   seu histórico; o aplicativo desativa funcionários, não os exclui.
-- - `previous_supervisor_id` / `new_supervisor_id` usam `ON DELETE SET NULL`
--   apenas como salvaguarda: a identidade do evento (quem mudou, quando e o
--   tipo de operação) permanece.
-- - `author_id` é NULL enquanto não houver identidade autenticada confiável
--   para a operação; não é preenchido com o proprietário da instalação.

CREATE TABLE IF NOT EXISTS employee_supervisor_history (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    employee_id TEXT NOT NULL REFERENCES employees(id) ON DELETE RESTRICT,
    previous_supervisor_id TEXT REFERENCES employees(id) ON DELETE SET NULL,
    new_supervisor_id TEXT REFERENCES employees(id) ON DELETE SET NULL,
    operation TEXT NOT NULL CHECK (operation IN ('assigned', 'changed', 'removed')),
    reason TEXT,
    author_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_employee_supervisor_history_employee
    ON employee_supervisor_history(employee_id, created_at);

CREATE INDEX IF NOT EXISTS idx_employee_supervisor_history_previous
    ON employee_supervisor_history(previous_supervisor_id);

CREATE INDEX IF NOT EXISTS idx_employee_supervisor_history_new
    ON employee_supervisor_history(new_supervisor_id);
