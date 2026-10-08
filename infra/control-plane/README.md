# Development resource baseline

This is the Milestone 2 binding-check Worker, not the deployment authority or a
complete Deployment Panel. Infra owns it. Panel will consume the versioned API
after candidate registration and the first safe deployment slice exist.

The committed configuration is for local tests/dry-run only: it contains no live
account, database IDs, routes, operator identity or secrets. Bootstrap is disabled.
Use an ignored operator configuration under `.runtime-checks/` for live bindings;
confirm the account, existing resource inventory, routes and Access application
before deploying. Keep `workers_dev` and `preview_urls` disabled.

## Local verification

```sh
npm ci --ignore-scripts
node --test tests/deployment*.test.mjs tests/control-plane-runtime.test.mjs
npx --no-install wrangler types control-plane/worker-configuration.d.ts --config control-plane/wrangler.jsonc --strict-vars=false --include-runtime=false --check
npx --no-install tsc -p control-plane/tsconfig.json
npx --no-install wrangler deploy --config control-plane/wrangler.jsonc --dry-run --outdir dist/control-plane
```

Tests use the actual local Workers runtime, separate local D1 stores, R2 and a
Workflow. They apply both migrations, verify expected health identity, execute
read/write checks, confirm object/identity-row cleanup and retain a small harness
verification record. Access tests use ephemeral synthetic signing keys; they do
not load a live secret, account cookie or operator JWT.

## Migrations and environments

`migrations/harness/0001_bootstrap.sql` and
`migrations/identity/0001_bootstrap.sql` are expand-only initial schemas for new,
empty Development stores. They do not import or change the existing SQLite
Authentication authority, signing assets, accounts or sessions. Future normalized
models need new sequential migrations and representative-data tests. Do not apply
these baseline migrations to an existing product database.

Use distinct resource names ending in `-development`, `-beta` and `-production`.
Initially provision Development only: one control Worker, two D1 stores, one
private Standard R2 operational bucket and one binding-check Workflow. Docs is an
independent static Worker artifact. Beta/Production resources, deployment/rollback
Workflows, identity service and full Panel are still absent.

The minimal operational bucket uses `bootstrap/` (one-day expiry), `logs/` and
`backups/` (seven-day expiry), with one-day multipart aborts for these prefixes.
Do not carry these Development backup-retention defaults into Production without
an explicit recovery policy. Public bucket access and S3/agent credentials are
not required. Restore remains an independently authorized operation.

## Access and resource proof

Protect the exact Development control hostname with a self-hosted Access app,
an Allow policy for the exact operator email and a short session duration. Reuse
the existing Free Zero Trust organization; do not create another organization or
add a paid plan. There is no bypass policy. Verify the origin JWT's RS256 signature,
issuer, audience, expiry, app type and exact bootstrap principal as well as edge
Access protection. A header by itself is not authentication.

With `BOOTSTRAP_ENABLED=1`, an authenticated same-origin request can POST
`/api/v1/bootstrap/bindings-check` with `X-Velora-Bootstrap: 1` and a v4 UUID
`Idempotency-Key`. No user SQL, object key, shell command or deployment target is
accepted. At most ten unique checks can be reserved in this database. Workflow
IDs bind exactly to their reserved check. Retries are bounded (two retries with
one-second exponential delay and a 30-second step timeout). GET the resulting
`/api/v1/bootstrap/bindings-check/{id}` to inspect completion. The proof writes
only tiny synthetic objects/rows and cleans them up. No service is deployed.

After successful live verification, set `BOOTSTRAP_ENABLED=0` again. Keep only the
bounded verification record. Do not log JWTs, cookies, credentials or request
headers. Live registration, RBAC, audit models and automation flags are Milestone 3.

## Cost controls

Start with Workers/Zero Trust Free, private R2 Standard storage and no Cloudflare
Containers, AI, Queues, paid certificates, external compute or automatic upgrades.
The R2 subscription has no fixed fee but bills usage above its shared free quota;
it is not a hard spending cap. Use lifecycle rules and small bounded test payloads.
Indexed models, capped upload sizes/retained bytes, jittered 15–30 second agent
polling, bounded jobs and paginated UI reads must be implemented before workloads
are onboarded. Existing account usage shares the free quotas.

Exact Workers Custom Domains receive certificates including multi-level names
without a separate Advanced Certificate Manager subscription. Configure specific
hostnames, not a catch-all route over the existing domain. Preserve existing DNS
and services. Confirm certificate activation before treating HTTPS as ready.

Official references checked 2026-10-07:
[Worker custom domains](https://developers.cloudflare.com/workers/configuration/routing/custom-domains/),
[D1 migrations](https://developers.cloudflare.com/d1/reference/migrations/),
[Access JWT validation](https://developers.cloudflare.com/cloudflare-one/access-controls/applications/http-apps/authorization-cookie/validating-json/),
[Workers pricing](https://developers.cloudflare.com/workers/platform/pricing/),
[Workflows pricing](https://developers.cloudflare.com/workflows/reference/pricing/),
[R2 pricing](https://developers.cloudflare.com/r2/pricing/).
