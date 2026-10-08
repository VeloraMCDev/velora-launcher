-- No existing agent gains execution permission. Targets require explicit approval.
CREATE TABLE probe_targets (
  agent_id TEXT PRIMARY KEY REFERENCES agents(id),
  environment_id TEXT NOT NULL REFERENCES environments(id) CHECK(environment_id='development'),
  service_id TEXT NOT NULL REFERENCES services(id) CHECK(service_id='deployment-probe'),
  enabled INTEGER NOT NULL DEFAULT 0 CHECK(enabled IN (0,1))
);
CREATE TABLE probe_deployments (
  id TEXT PRIMARY KEY,
  candidate_id TEXT NOT NULL REFERENCES deployment_candidates(id),
  agent_id TEXT NOT NULL REFERENCES probe_targets(agent_id),
  environment_id TEXT NOT NULL REFERENCES environments(id) CHECK(environment_id='development'),
  service_id TEXT NOT NULL REFERENCES services(id) CHECK(service_id='deployment-probe'),
  actor_id TEXT NOT NULL,
  job_id TEXT NOT NULL UNIQUE,
  status TEXT NOT NULL CHECK(status IN ('RESERVED','JOB_READY','SUCCEEDED','FAILED','ROLLED_BACK','BLOCKED','ROLLBACK_FAILED')),
  job_json TEXT CHECK(job_json IS NULL OR json_valid(job_json)),
  payload_sha256 TEXT,
  expires_at INTEGER,
  delivered_at INTEGER,
  receipt_json TEXT CHECK(receipt_json IS NULL OR json_valid(receipt_json)),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
-- A timeout/uncertain rollback retains the lock until an authenticated result resolves it.
CREATE UNIQUE INDEX probe_deployment_lock ON probe_deployments(environment_id,service_id)
  WHERE status IN ('RESERVED','JOB_READY','BLOCKED','ROLLBACK_FAILED');
CREATE INDEX probe_deployment_page ON probe_deployments(environment_id,id);
CREATE INDEX probe_agent_jobs ON probe_deployments(agent_id,status);
CREATE INDEX probe_deployment_budget ON probe_deployments(created_at);
UPDATE schema_metadata SET schema_version=4 WHERE id=1 AND purpose='velora-harness';
