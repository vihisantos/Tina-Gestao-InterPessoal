CREATE TABLE employee_performance_reviews (
 id TEXT PRIMARY KEY NOT NULL,
 employee_id TEXT NOT NULL REFERENCES employees(id),
 task_ref TEXT NOT NULL,
 evidence_ref TEXT NOT NULL,
 outcome TEXT NOT NULL CHECK(outcome IN ('success','failure','rework','legitimate_refusal')),
 assessment TEXT NOT NULL,
 reviewer_id TEXT NOT NULL,
 reviewed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX employee_review_history ON employee_performance_reviews(employee_id,reviewed_at);
