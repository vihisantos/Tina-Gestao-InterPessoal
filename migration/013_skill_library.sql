-- Biblioteca local; nenhuma atribuição concede permissões de ferramentas.
CREATE TABLE skills (
 id TEXT PRIMARY KEY NOT NULL,
 company_id TEXT NOT NULL REFERENCES companies(id),
 name TEXT NOT NULL CHECK(length(trim(name)) > 0),
 objective TEXT NOT NULL,
 UNIQUE(id, company_id)
);
CREATE TABLE skill_versions (
 skill_id TEXT NOT NULL REFERENCES skills(id),
 version INTEGER NOT NULL CHECK(version > 0),
 manifest TEXT NOT NULL CHECK(json_valid(manifest)),
 status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','validated','published','retired')),
 PRIMARY KEY(skill_id, version)
);
CREATE TABLE position_skills (
 position_id TEXT NOT NULL REFERENCES positions(id),
 skill_id TEXT NOT NULL,
 version INTEGER NOT NULL,
 PRIMARY KEY(position_id, skill_id),
 FOREIGN KEY(skill_id, version) REFERENCES skill_versions(skill_id, version)
);
CREATE TABLE skill_assignments (
 employee_id TEXT NOT NULL REFERENCES employees(id),
 skill_id TEXT NOT NULL,
 version INTEGER NOT NULL,
 origin TEXT NOT NULL CHECK(origin IN ('hiring','training')),
 justification TEXT NOT NULL CHECK(length(trim(justification)) > 0),
 assigned_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 PRIMARY KEY(employee_id, skill_id),
 FOREIGN KEY(skill_id, version) REFERENCES skill_versions(skill_id, version)
);
CREATE TRIGGER position_skill_guard BEFORE INSERT ON position_skills
BEGIN
 SELECT CASE WHEN NOT EXISTS (
 SELECT 1 FROM positions p JOIN departments d ON d.id=p.department_id
 JOIN skills s ON s.company_id=d.company_id
 JOIN skill_versions v ON v.skill_id=s.id
 WHERE p.id=NEW.position_id AND s.id=NEW.skill_id AND v.version=NEW.version AND v.status='published'
 ) THEN RAISE(ABORT,'invalid position skill') END;
END;
CREATE TRIGGER employee_skill_guard BEFORE INSERT ON skill_assignments
BEGIN
 SELECT CASE WHEN NOT EXISTS (
 SELECT 1 FROM employees e JOIN positions p ON p.id=e.position_id JOIN departments d ON d.id=p.department_id
 JOIN skills s ON s.company_id=d.company_id JOIN skill_versions v ON v.skill_id=s.id
 WHERE e.id=NEW.employee_id AND e.status='active' AND s.id=NEW.skill_id AND v.version=NEW.version AND v.status='published'
 ) THEN RAISE(ABORT,'invalid employee skill') END;
END;
-- Alterações devem remover e recriar vínculos, passando novamente pelos guards.
CREATE TRIGGER position_skill_no_update BEFORE UPDATE ON position_skills BEGIN SELECT RAISE(ABORT,'replace position skill'); END;
CREATE TRIGGER employee_skill_no_update BEFORE UPDATE ON skill_assignments BEGIN SELECT RAISE(ABORT,'replace employee skill'); END;
-- A contratação herda apenas versões publicadas do cargo na mesma transação.
CREATE TRIGGER employee_hiring_skills AFTER INSERT ON employees
WHEN NEW.status='active'
BEGIN
 INSERT INTO skill_assignments(employee_id,skill_id,version,origin,justification)
 SELECT NEW.id, ps.skill_id, ps.version,'hiring','Habilidade publicada recomendada para o cargo'
 FROM position_skills ps JOIN skill_versions v ON v.skill_id=ps.skill_id AND v.version=ps.version
 WHERE ps.position_id=NEW.position_id AND v.status='published';
END;
-- Consultas de execução devem usar esta visão, além da autorização de ferramentas.
CREATE VIEW active_employee_skills AS
 SELECT a.* FROM skill_assignments a JOIN skill_versions v ON v.skill_id=a.skill_id AND v.version=a.version
 JOIN employees e ON e.id=a.employee_id JOIN positions p ON p.id=e.position_id
 JOIN departments d ON d.id=p.department_id JOIN skills s ON s.id=a.skill_id AND s.company_id=d.company_id
 WHERE v.status='published' AND e.status='active' AND p.status='active' AND d.status='active';
