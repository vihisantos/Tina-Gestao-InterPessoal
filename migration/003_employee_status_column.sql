-- Tina Platform — migration 003: coluna status na tabela employees
-- Adiciona coluna de status textual à tabela employees, consistente com
-- as demais tabelas organizacionais (companies, departments, positions).
-- Mantém a coluna is_active existente para compatibilidade.

ALTER TABLE employees ADD COLUMN status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'inactive'));
