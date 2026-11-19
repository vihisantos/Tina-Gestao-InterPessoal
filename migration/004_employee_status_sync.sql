-- Tina Platform — migration 004: sincronização de status de funcionários
--
-- Objetivo: eliminar a inconsistência entre as colunas `status` e `is_active`
-- da tabela `employees`, garantindo uma única fonte de verdade para o estado
-- do funcionário.
--
-- 1. Backfill: preenche `status` a partir de `is_active` para registros
--    existentes criados antes da migration 003, de modo que funcionários
--    legados com `is_active = 0` continuem inativos.
-- 2. Define `status` como fonte de verdade: `is_active` é mantida por
--    compatibilidade e sempre sincronizada com `status`.
-- 3. Recria a view `active_employees` para filtrar por `status = 'active'`,
--   refletindo corretamente o estado atual dos funcionários.

-- Backfill: registros existentes herdam o status de is_active
UPDATE employees
SET status = CASE WHEN is_active = 1 THEN 'active' ELSE 'inactive' END
WHERE status IS NULL OR status = '';

-- Garantir que registros com is_active = 0 tenham status = 'inactive'
UPDATE employees
SET status = 'inactive'
WHERE is_active = 0 AND status <> 'inactive';

-- Garantir que registros com is_active = 1 tenham status = 'active'
UPDATE employees
SET status = 'active'
WHERE is_active = 1 AND status <> 'active';

-- Recriar a view active_employees usando status como fonte de verdade
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
