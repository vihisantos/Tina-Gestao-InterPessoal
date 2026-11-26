-- Contexto pessoal usa a mesma hierarquia e os mesmos controles de empresa.
-- Registros anteriores permanecem empresariais até decisão explícita do proprietário.
ALTER TABLE companies ADD COLUMN kind TEXT NOT NULL DEFAULT 'business'
    CHECK (kind IN ('business', 'personal'));

-- Uma organização pessoal por instalação, inclusive quando arquivada.
CREATE UNIQUE INDEX idx_companies_one_personal
    ON companies(installation_id) WHERE kind = 'personal';
