CREATE TABLE employee_memory_candidates (
 id TEXT PRIMARY KEY NOT NULL,
 employee_id TEXT NOT NULL REFERENCES employees(id),
 source_ref TEXT NOT NULL,
 content TEXT NOT NULL,
 status TEXT NOT NULL DEFAULT 'candidate' CHECK(status IN ('candidate','approved','rejected','revoked')),
 reviewed_by TEXT,
 reviewed_at TEXT,
 created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 CHECK(status='candidate' OR (reviewed_by IS NOT NULL AND reviewed_at IS NOT NULL))
);
CREATE INDEX employee_memory_review ON employee_memory_candidates(employee_id,status);
