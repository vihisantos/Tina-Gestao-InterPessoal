CREATE TABLE employee_professional_records (
 id TEXT PRIMARY KEY NOT NULL,
 employee_id TEXT NOT NULL REFERENCES employees(id),
 category TEXT NOT NULL CHECK(category IN ('qualification','training','promotion','transfer','review','hiring','termination','specialty')),
 description TEXT NOT NULL CHECK(length(trim(description))>0),
 evidence_ref TEXT,
 evidence_status TEXT NOT NULL DEFAULT 'claimed' CHECK(evidence_status IN ('claimed','verified')),
 supersedes_id TEXT REFERENCES employee_professional_records(id),
 recorded_by TEXT NOT NULL,
 recorded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 CHECK(evidence_status <> 'verified' OR length(trim(coalesce(evidence_ref,'')))>0)
);
CREATE INDEX employee_records_history ON employee_professional_records(employee_id,recorded_at);

CREATE TRIGGER professional_record_no_update BEFORE UPDATE ON employee_professional_records
BEGIN SELECT RAISE(ABORT,'append a correction instead'); END;
CREATE TRIGGER professional_record_no_delete BEFORE DELETE ON employee_professional_records
BEGIN SELECT RAISE(ABORT,'preserve professional history'); END;
CREATE TRIGGER professional_record_same_employee BEFORE INSERT ON employee_professional_records
WHEN NEW.supersedes_id IS NOT NULL AND NOT EXISTS (
 SELECT 1 FROM employee_professional_records WHERE id=NEW.supersedes_id AND employee_id=NEW.employee_id
)
BEGIN SELECT RAISE(ABORT,'invalid correction scope'); END;
CREATE UNIQUE INDEX professional_record_one_correction ON employee_professional_records(supersedes_id) WHERE supersedes_id IS NOT NULL;
