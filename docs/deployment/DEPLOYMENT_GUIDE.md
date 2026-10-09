# Deploying Velora

## Place workloads according to their runtime

Cloudflare Workers hosts the deployment control plane with D1, R2 and Workflows.
Protect operator access with Access and keep machine ingress separately authorized.
Static documentation and frontend assets can use existing Cloudflare hosting;
share hostnames where practical and avoid duplicating paid resources.

The complete Panel currently runs native Rust/Axum with SQLite and filesystem
storage. Host it on existing native capacity or a VPS. Minecraft servers are
long-running Java workloads. The outbound native agent manages these hosts without
an inbound public listener. Libraries, desktop clients and Java integration JARs
need no dedicated server or domain. See [target assignments](../DEPLOYMENT_TARGETS.md).

## Prepare a release

1. Pass source CI and the checks for every changed segment.
2. Build immutable artifacts and validate the exact Panel image on Docker.
3. Register the canonical repository ID, workflow path and immutable SHA source
   policy; review package permissions for VeloraMCDev/velora-launcher.
4. Verify service definitions, schema versions, credentials, persistent stores,
   health checks and backup contracts before submitting a candidate.
5. Confirm native host/domain assignments, agent enrollment and job transport.
6. Exercise failure, rollback and restore in Development before environment promotion.

Use the [registration contract](https://github.com/VeloraMCDev/velora-launcher/blob/main/infra/control-plane/REGISTRATION.md),
[agent guide](https://github.com/VeloraMCDev/velora-launcher/blob/main/infra/agent/README.md) and maintained recipes under infra/recipes.
Bootstrap and registration must remain disabled except for an explicitly configured
rollout. A public CI run does not grant deployment authority.

## Preserve state and control cost

Keep installed identities, volume names, owned SQLite stores and signing material.
Do not replace a native store with D1 without a reviewed data/continuity conversion.
Keep runtime SQL migrations; they create and upgrade real service schemas.
Back up before promotion, verify backup restoration and permit only one writer
per credential store. Record image digests and source commits for recovery.

Reuse existing Cloudflare resources and machines. Keep expensive packaging manual,
avoid unused environment duplication and apply retention to release artifacts and
logs. No production host/domain assignment or completed rollout is implied by
the repository's passing source checks.
