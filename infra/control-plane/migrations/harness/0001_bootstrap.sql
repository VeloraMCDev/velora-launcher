-- Expand-only initial schema for a new Development database. No application data.
CREATE TABLE schema_metadata (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  schema_version INTEGER NOT NULL CHECK (schema_version > 0),
  purpose TEXT NOT NULL
);
INSERT INTO schema_metadata VALUES (1, 1, 'velora-harness');

CREATE TABLE bootstrap_checks (
  id TEXT PRIMARY KEY NOT NULL,
  environment TEXT NOT NULL CHECK (environment = 'development'),
  status TEXT NOT NULL CHECK (status IN ('RESERVED', 'SUCCEEDED', 'FAILED')),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  completed_at TEXT
);
