-- Tina Platform — migration 002: coluna de desativação de funcionários
-- Adiciona coluna de_timestamp de desativação à tabela employees,
-- permitindo rastrear quando um funcionário foi desativado sem
-- apagar registros históricos de forma destrutiva.

ALTER TABLE employees ADD COLUMN deactivated_at TEXT;
