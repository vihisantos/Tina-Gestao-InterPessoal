CREATE TABLE employee_professional_records (
 id TEXT PRIMARY KEY NOT NULL,
 employee_id TEXT NOT NULL REFERENCES employees(id),
 category TEXT NOT NULL CHECK(category IN ('qualification','training','promotion','transfer','review')),
 description TEXT NOT NULL CHECK(length(trim(description))>0),
 evidence_ref TEXT,
 recorded_by TEXT NOT NULL,
 recorded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX employee_records_history ON employee_professional_records(employee_id,recorded_at);
