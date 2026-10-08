-- Deployment principals are separate from native game/player credentials.
-- No principal, role binding or implicit administrator is seeded.
CREATE TABLE operator_principals (
  id TEXT PRIMARY KEY,
  access_issuer TEXT NOT NULL,
  access_subject TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'DISABLED' CHECK(status IN ('ACTIVE','DISABLED')),
  UNIQUE(access_issuer,access_subject)
);
CREATE TABLE operator_roles (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL
);
CREATE TABLE operator_role_permissions (
  role_id TEXT NOT NULL REFERENCES operator_roles(id),
  permission TEXT NOT NULL,
  PRIMARY KEY(role_id,permission)
);
CREATE TABLE operator_role_bindings (
  principal_id TEXT NOT NULL REFERENCES operator_principals(id),
  role_id TEXT NOT NULL REFERENCES operator_roles(id),
  environment_id TEXT NOT NULL CHECK(environment_id IN ('development','beta','production')),
  PRIMARY KEY(principal_id,role_id,environment_id)
);
CREATE INDEX role_bindings_environment ON operator_role_bindings(environment_id,principal_id);
UPDATE schema_metadata SET schema_version=2 WHERE id=1 AND purpose='velora-identity';
