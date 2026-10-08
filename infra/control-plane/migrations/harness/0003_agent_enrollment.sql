-- Expand-only. Initial agent policy permits Development heartbeats only.
ALTER TABLE automation_flags ADD COLUMN enrollment_enabled INTEGER NOT NULL DEFAULT 0 CHECK(enrollment_enabled IN (0,1));
CREATE TABLE agent_enrollment_tokens (
  token_hash TEXT PRIMARY KEY CHECK(length(token_hash)=64),
  environment_id TEXT NOT NULL REFERENCES environments(id) CHECK(environment_id='development'),
  capabilities_json TEXT NOT NULL CHECK(capabilities_json='["REPORT_HEARTBEAT"]'),
  expires_at INTEGER NOT NULL,
  consumed_at INTEGER,
  created_at INTEGER NOT NULL
);
CREATE INDEX enrollment_expiration ON agent_enrollment_tokens(expires_at);
CREATE TABLE agents (
  id TEXT PRIMARY KEY,
  key_id TEXT NOT NULL UNIQUE CHECK(length(key_id)=64),
  public_key TEXT NOT NULL UNIQUE CHECK(length(public_key)=43),
  enrollment_token_hash TEXT NOT NULL UNIQUE REFERENCES agent_enrollment_tokens(token_hash),
  environment_id TEXT NOT NULL REFERENCES environments(id) CHECK(environment_id='development'),
  capabilities_json TEXT NOT NULL CHECK(capabilities_json='["REPORT_HEARTBEAT"]'),
  status TEXT NOT NULL CHECK(status IN ('ACTIVE','REVOKED')),
  enrolled_at INTEGER NOT NULL,
  heartbeat_at INTEGER,
  heartbeat_json TEXT CHECK(heartbeat_json IS NULL OR json_valid(heartbeat_json)),
  revoked_at INTEGER
);
CREATE INDEX agents_environment_page ON agents(environment_id,id);
CREATE TABLE agent_nonces (
  agent_id TEXT NOT NULL REFERENCES agents(id),
  nonce TEXT NOT NULL,
  request_id TEXT NOT NULL,
  expires_at INTEGER NOT NULL,
  PRIMARY KEY(agent_id,nonce)
);
CREATE INDEX agent_nonce_expiration ON agent_nonces(expires_at);
UPDATE schema_metadata SET schema_version=3 WHERE id=1 AND purpose='velora-harness';
