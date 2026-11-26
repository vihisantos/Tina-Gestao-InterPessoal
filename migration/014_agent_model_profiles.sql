-- Perfis aprovados são separados do valor provisório legado.
CREATE TABLE agent_model_profiles (
    employee_id TEXT PRIMARY KEY NOT NULL REFERENCES employees(id),
    nvidia_model_id TEXT NOT NULL CHECK(length(trim(nvidia_model_id)) > 0 AND nvidia_model_id = trim(nvidia_model_id)),
    approved_by TEXT NOT NULL CHECK(length(trim(approved_by)) > 0),
    justification TEXT NOT NULL CHECK(length(trim(justification)) > 0),
    approved_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    revision INTEGER NOT NULL DEFAULT 1 CHECK(revision > 0)
);
-- Sem perfil aprovado, o agente não deve executar tarefas.
CREATE VIEW approved_active_agent_models AS
SELECT a.employee_id, a.nvidia_model_id, a.revision, c.installation_id
FROM agent_model_profiles a
JOIN employees e ON e.id = a.employee_id
JOIN positions p ON p.id = e.position_id
JOIN departments d ON d.id = p.department_id
JOIN companies c ON c.id = d.company_id
WHERE e.status = 'active' AND p.status = 'active'
AND d.status = 'active' AND c.status = 'active';
