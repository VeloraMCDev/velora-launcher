# Milestone 2 — Development resource baseline

## Completed

Development now has a control Worker, two new D1 databases, a private Standard
R2 bucket, a bounded binding-check Workflow and an independent Docs Static Assets
Worker. Both Workers use exact custom domains with managed certificates. The
self-hosted Access application allows only the operator's exact email and uses
30-minute sessions. Existing DNS records and applications were preserved.

## Verification

- Infra source `aed1f0418175d5f35131f432dcabfa85d203113a`: push run
  [37665751824](https://github.com/VeloraMCDev/infra/actions/runs/37665751824)
  passed runtime, access, schema/workflow and actual disposable-container checks.
- `npm test`: 14 tests passed, including actual local Workers/D1/R2/Workflow
  success and failure cleanup. Type generation, TypeScript and dry-run passed.
- Both remote initial D1 migrations applied successfully; reapplying reported
  no outstanding migrations. Existing Authentication databases were untouched.
- One Cloudflare-account-authenticated live Workflow and one separate
  signed-Access browser request completed with harness,
  identity, R2, Workflow and cleanup all `PASS`. No service deployment occurred.
  The identity proof table and R2 bucket were empty afterward; two bounded
  successful verification records remain in the harness database.
- Docs source `ff534ec83f3437f1b350b13f306a7764213e6453`: push run
  [37662238288](https://github.com/VeloraMCDev/docs/actions/runs/37662238288)
  passed. All 19 manifest file checksums matched before deployment.
- The live Worker module also matched its CI manifest checksum. It was deployed
  without rebuilding. Both managed certificates became Active; unauthenticated
  HTTPS requests redirect to Access. Authenticated browser verification passed: POST accepted the signed
  Access identity, Workflow completed with all five checks PASS, then bootstrap
  writes were disabled and returned `403 BOOTSTRAP_DISABLED`. The protected
  live Docs homepage loaded all images with no horizontal overflow.
- All seven platform repository public-source preflights passed at this checkpoint.
  That scanner does not certify every historical release, dependency or migration.

## Files changed

Infra: `control-plane/`, `tests/deployment-access.test.mjs`,
`tests/control-plane-runtime.test.mjs`, `scripts/worker-artifact.mjs`,
`.github/workflows/deployment-baseline.yml`, `deployment/HERMES.md` and its
read-only host preflight. Docs: the static build, locked toolchain, source checks
and deployment documentation. Live resource IDs, domains and operator identity
remain in Infra's ignored `.runtime-checks/` configuration and evidence files.

## Architecture decisions

The first Worker proves resource access only. It is not the deployment authority,
full Identity service or Panel. New D1 schemas do not import or replace native
Authentication's SQLite data, keys or transactions. Hermes will run a native
systemd agent; regular deployment traffic uses outbound HTTPS.

Workers and Zero Trust stay on Free plans. R2 has no fixed monthly fee; usage
above shared free allowances can be billed. One-day bootstrap and seven-day
log/backup lifecycle rules bound this empty Development bucket's retention.
Exact Worker custom domains include their managed multi-level certificates
without a separate paid certificate subscription.

## Known issues / deferred work

CI candidate registration, scoped automated Cloudflare publishing and durable
artifact custody are not implemented. CI artifacts expire after 14 days.
No GHCR release workflow has run. Beta/Production resources, the complete Panel,
agents, rollback and backup/restore acceptance remain absent. Hermes SSH is active and its host fingerprint matches; key authentication
is being configured. Architecture, disk capacity and intended agent permissions are
unverified. Follow Infra's first-time SSH instructions on the local network.

## Security notes

Bootstrap API writes are disabled after the live account-level proof. When testing
the browser endpoint, enable them only temporarily and disable them afterward.
Signed Access JWTs are validated at the origin; a header alone is insufficient.
The check accepts no SQL, object paths, shell commands or deployment targets.
No production workload, database or signing asset was changed.

## Next milestone

Implement Milestone 3: authenticated
candidate registration, normalized repository/service/environment models, RBAC,
audit, webhook verification/deduplication, automation flags and OpenAPI. A CI run
must register an exact immutable candidate before agent/deployment work begins.
