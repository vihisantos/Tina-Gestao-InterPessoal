-- Tina Platform — migration 004: sincronização de status de funcionários
--
-- Fonte de verdade: `employees.status` ('active' | 'inactive').
-- A coluna `is_active` é mantida apenas por compatibilidade e passa a ser um
-- espelho de `status`.
--
-- Contexto: a migration 003 adicionou `status` com NOT NULL DEFAULT 'active',
-- marcando como ativos também os funcionários legados que tinham
-- `is_active = 0`. Esta migration corrige o backfill sem apagar histórico.
--
-- 1. Funcionários legados que ainda estão no valor padrão 'active' da
--    migration 003 mas tinham `is_active = 0` passam a `status = 'inactive'`.
-- 2. `is_active` é sincronizada com `status` (somente linhas divergentes).
-- 3. A view `active_employees` é recriada filtrando por `status = 'active'`.
--
-- A partir daqui, criação, desativação e reativação gravam as duas colunas
-- juntas; nenhum fluxo do aplicativo deve alterar apenas uma delas.

-- 1. Backfill dos funcionários legados inativos.
UPDATE employees
SET status = 'inactive'
WHERE is_active = 0 AND status = 'active';

-- 2. `is_active` espelha `status`.
UPDATE employees
SET is_active = CASE WHEN status = 'active' THEN 1 ELSE 0 END
WHERE is_active <> CASE WHEN status = 'active' THEN 1 ELSE 0 END;

-- 3. Recriar a view active_employees usando status como fonte de verdade.
DROP VIEW IF EXISTS active_employees;

CREATE VIEW IF NOT EXISTS active_employees AS
SELECT
    e.id,
    e.name,
    e.description,
    p.name AS position_name,
    d.name AS department_name,
    c.name AS company_name,
    e.model_config,
    e.authority_override,
    e.created_at
FROM employees e
JOIN positions p ON e.position_id = p.id
JOIN departments d ON p.department_id = d.id
JOIN companies c ON d.company_id = c.id
WHERE e.status = 'active';
