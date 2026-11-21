-- Tina Platform — migration 006: histórico de movimentações de cargo/departamento
--
-- Registra cada mudança de `employees.position_id`. Como o departamento é
-- derivado do cargo (`position → department`), o evento guarda também o
-- departamento anterior e o novo, permitindo rastrear transferências entre
-- departamentos mesmo quando o cargo muda dentro do mesmo departamento.
--
-- Regras:
-- - Log append-only: a relação atual continua apenas em `employees`.
-- - `ON DELETE RESTRICT` em `employee_id` e nas referências organizacionais
--   preserva a trilha; o aplicativo desativa funcionários, não os exclui.
-- - `author_id` é NULL enquanto não houver identidade autenticada confiável
--   para a operação; não é preenchido com o proprietário da instalação.

CREATE TABLE IF NOT EXISTS employee_assignment_history (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    employee_id TEXT NOT NULL REFERENCES employees(id) ON DELETE RESTRICT,
    previous_position_id TEXT NOT NULL REFERENCES positions(id) ON DELETE RESTRICT,
    new_position_id TEXT NOT NULL REFERENCES positions(id) ON DELETE RESTRICT,
    previous_department_id TEXT NOT NULL REFERENCES departments(id) ON DELETE RESTRICT,
    new_department_id TEXT NOT NULL REFERENCES departments(id) ON DELETE RESTRICT,
    reason TEXT,
    author_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_employee_assignment_history_employee
    ON employee_assignment_history(employee_id, created_at);

CREATE INDEX IF NOT EXISTS idx_employee_assignment_history_previous_department
    ON employee_assignment_history(previous_department_id);

CREATE INDEX IF NOT EXISTS idx_employee_assignment_history_new_department
    ON employee_assignment_history(new_department_id);

-- Imutabilidade: eventos históricos não podem ser alterados nem removidos.
CREATE TRIGGER IF NOT EXISTS employee_supervisor_history_reject_update
BEFORE UPDATE ON employee_supervisor_history
BEGIN
    SELECT RAISE(ABORT, 'employee_supervisor_history is append-only: UPDATE rejected');
END;

CREATE TRIGGER IF NOT EXISTS employee_supervisor_history_reject_delete
BEFORE DELETE ON employee_supervisor_history
BEGIN
    SELECT RAISE(ABORT, 'employee_supervisor_history is append-only: DELETE rejected');
END;

CREATE TRIGGER IF NOT EXISTS employee_assignment_history_reject_update
BEFORE UPDATE ON employee_assignment_history
BEGIN
    SELECT RAISE(ABORT, 'employee_assignment_history is append-only: UPDATE rejected');
END;

CREATE TRIGGER IF NOT EXISTS employee_assignment_history_reject_delete
BEFORE DELETE ON employee_assignment_history
BEGIN
    SELECT RAISE(ABORT, 'employee_assignment_history is append-only: DELETE rejected');
END;
