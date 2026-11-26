CREATE TABLE employee_operational_events (
 id TEXT PRIMARY KEY NOT NULL,
 employee_id TEXT NOT NULL REFERENCES employees(id),
 state TEXT NOT NULL CHECK(state IN ('available','working','awaiting_approval','suspended','terminated')),
 task_ref TEXT,
 actor_id TEXT NOT NULL,
 reason TEXT NOT NULL,
 occurred_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX employee_operational_history ON employee_operational_events(employee_id,occurred_at);
