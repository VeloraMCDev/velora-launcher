-- Expand-only. Empty registries grant no CI trust and no deployment capability.
CREATE TABLE repositories (
  id TEXT PRIMARY KEY,
  full_name TEXT NOT NULL UNIQUE COLLATE NOCASE,
  github_repository_id TEXT NOT NULL UNIQUE,
  github_owner_id TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 0 CHECK(enabled IN (0,1))
);
CREATE TABLE environments (
  id TEXT PRIMARY KEY CHECK(id IN ('development','beta','production')),
  display_name TEXT NOT NULL
);
INSERT INTO environments VALUES ('development','Development'),('beta','Beta'),('production','Production');
CREATE TABLE services (
  id TEXT PRIMARY KEY,
  repository_id TEXT NOT NULL REFERENCES repositories(id),
  artifact_type TEXT NOT NULL CHECK(artifact_type IN ('oci','binary','static_bundle','worker_bundle','jar','config_bundle')),
  artifact_uri_prefix TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 0 CHECK(enabled IN (0,1))
);
CREATE INDEX services_repository ON services(repository_id,id);
CREATE TABLE ci_identity_policies (
  id TEXT PRIMARY KEY,
  repository_id TEXT NOT NULL REFERENCES repositories(id),
  git_ref TEXT NOT NULL,
  workflow_path TEXT NOT NULL,
  workflow_sha TEXT NOT NULL CHECK(length(workflow_sha)=40),
  subject_format TEXT NOT NULL DEFAULT 'immutable' CHECK(subject_format IN ('immutable','legacy')),
  enabled INTEGER NOT NULL DEFAULT 0 CHECK(enabled IN (0,1)),
  UNIQUE(repository_id,git_ref,workflow_path,workflow_sha)
);
CREATE TABLE artifacts (
  id TEXT PRIMARY KEY,
  repository_id TEXT NOT NULL REFERENCES repositories(id),
  service_id TEXT NOT NULL REFERENCES services(id),
  artifact_type TEXT NOT NULL,
  artifact_uri TEXT NOT NULL,
  sha256 TEXT NOT NULL CHECK(length(sha256)=64),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(service_id,artifact_type,artifact_uri,sha256)
);
CREATE TABLE deployment_candidates (
  id TEXT PRIMARY KEY,
  artifact_id TEXT NOT NULL REFERENCES artifacts(id),
  repository_id TEXT NOT NULL REFERENCES repositories(id),
  service_id TEXT NOT NULL REFERENCES services(id),
  git_sha TEXT NOT NULL CHECK(length(git_sha)=40),
  git_ref TEXT NOT NULL,
  build_run_id TEXT NOT NULL,
  metadata_json TEXT NOT NULL CHECK(json_valid(metadata_json)),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(repository_id,service_id,git_sha,artifact_id)
);
CREATE INDEX candidates_repository_page ON deployment_candidates(repository_id,id);
CREATE INDEX candidates_service_page ON deployment_candidates(service_id,id);
CREATE TABLE github_deliveries (
  id TEXT PRIMARY KEY,
  event_name TEXT NOT NULL,
  body_sha256 TEXT NOT NULL CHECK(length(body_sha256)=64),
  received_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  status TEXT NOT NULL CHECK(status IN ('RECEIVED','PROCESSED','IGNORED','FAILED')),
  UNIQUE(event_name,body_sha256)
);
CREATE TABLE automation_flags (
  environment_id TEXT PRIMARY KEY REFERENCES environments(id),
  registration_enabled INTEGER NOT NULL DEFAULT 0 CHECK(registration_enabled IN (0,1)),
  deployment_enabled INTEGER NOT NULL DEFAULT 0 CHECK(deployment_enabled IN (0,1)),
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
INSERT INTO automation_flags(environment_id) SELECT id FROM environments;
CREATE TABLE audit_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  event_key TEXT NOT NULL UNIQUE,
  actor_id TEXT NOT NULL,
  action TEXT NOT NULL,
  environment_id TEXT REFERENCES environments(id),
  target_id TEXT NOT NULL,
  request_id TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX audit_environment_page ON audit_events(environment_id,id);
UPDATE schema_metadata SET schema_version=2 WHERE id=1 AND purpose='velora-harness';
