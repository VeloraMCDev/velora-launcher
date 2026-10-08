-- Separate empty database; this does not migrate the existing native authority.
CREATE TABLE schema_metadata (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  schema_version INTEGER NOT NULL CHECK (schema_version > 0),
  purpose TEXT NOT NULL
);
INSERT INTO schema_metadata VALUES (1, 1, 'velora-identity');

CREATE TABLE bootstrap_checks (
  id TEXT PRIMARY KEY NOT NULL,
  marker TEXT NOT NULL CHECK (marker = 'binding-check')
);
