# Candidate registration API

Registration requires explicit environment-scoped source policies and remains
disabled until the canonical repository ID/workflow/SHA policy is reviewed.
See [deployment preparation](../../docs/deployment/DEPLOYMENT_GUIDE.md).

Sequential expand-only `0002` migrations define repository/service registries,
explicit CI identity policies, environments, immutable artifact/candidate records,
delivery replay records, automation flags and audit events. New operator-principal
and environment-scoped role tables are separate from native game/player authority.
No repository, service, principal, role binding or CI policy is trusted by default.
Every environment starts with registration and deployment disabled.

GitHub job authentication verifies RS256 signature, fixed issuer/JWKS, audience,
expiry/not-before, immutable repository/owner IDs, full source/workflow SHA, exact
workflow path/ref/subject and build run identity. Initial policy permits only
GitHub-hosted push/manual jobs with no environment context. Optional job-workflow
claims must exactly match the approved workflow reference and SHA; foreign or
independently pinned reusable workflows require a separate policy and are denied.
Untrusted PRs and unregistered sources are denied. Exact workflow-SHA policies
need an explicit operator update as reviewed workflows evolve; names alone do
not establish trust. New GitHub repositories use immutable ID-bearing subjects.
Legacy subjects require an explicit policy and still require immutable ID claims.

Access authentication is separate from role authorization. A valid Access app
JWT grants no implicit role. Principals must be active and have the exact permission
in the requested environment. Player/game tokens cannot satisfy this authentication.

Webhook verification uses the original bounded bytes and Web Crypto HMAC-SHA256
verification before JSON parsing. It rejects missing secrets, unsigned/modified
payloads, compression and bodies over 64 KiB. Only ping/workflow-run/release events
are currently understood when repository-scoped. App-level pings without a
repository are rejected pending an installation registry. D1 replay records enforce delivery-ID and event/body
uniqueness; a reused delivery ID with changed content is a conflict. Webhooks must
still be checked against registered repository identities, and cannot themselves
create candidates or deployments. No live GitHub App or webhook secret exists.

Audits can join the mutation's atomic D1 batch and use stable unique event keys.
No request headers, JWTs, cookies, signing secrets or raw webhook bodies are stored
in the audit helper. The API bounds reads to 50 records with typed cursors,
registration bodies to 16 KiB and the initial registry to 1,000 candidates.
Retention and recovery remain required before expanding this small registry.

Actual local D1 tests preserve a baseline proof record through the new schemas,
exercise active/disabled principals and cross-environment denial, prove concurrent
delivery deduplication and audit retry idempotency, and reject orphan service rows.
Synthetic RSA/HMAC tests verify signatures, identity policy and modified payloads.

`openapi.yaml` describes the Development-only routes. Exact CI/operator hostnames
separate ingress. The canonical artifact schema is compiled into a checked-in
standalone validator with no runtime compilation/evaluation; Worker SHA-256
candidate identity matches the Node contract helper. The service must belong to
the authenticated repository and permit the exact artifact type/URI prefix.
Candidate, artifact and stable audit event share an atomic batch; concurrent
retries return the original immutable metadata.

Pause/resume requires registration-management permission, a same-origin action
header and UUID idempotency key. Flag/audit mutations are atomic. Concurrent
conflicting requests with one key produce one accepted action and a 409 conflict.
Deployment stays disabled. Errors include generated request IDs and omit headers,
credentials and internal exceptions. The exported Worker defaults deny access.

The CI handoff runs after Worker/contract/actual-container checks, with only
contents-read and audience-bound OIDC token permission. An exact `main`-branch
gate and explicit repository-variable opt-in are required. The workflow guard
rejects weakening these gates or adding package-write permission. The upload
action's artifact ID/checksum are registered without a reusable account token.
Endpoint variables are operator configuration and must not be committed.

## Main-branch handoff (2026-10-08)

`deployment-baseline.yml` registers the tested Worker ZIP's exact upload ID and
checksum. `release-probe.yml` passes the tested/published OCI digest to a separate
`register-development` job after the complete candidate job succeeds. Package
publishing and OIDC permissions remain in separate jobs. Both registration jobs
require the repository variable `VELORA_DEVELOPMENT_REGISTRATION_ENABLED=true`;
unset/false skips registration. Neither job deploys anything.

The handoff validates the canonical artifact contract and current repository,
commit, ref and run before requesting an OIDC token. Candidate-file inputs must
be at most 16 KiB and cannot be combined with Worker upload coordinates. The API
receipt must return the exact deterministic candidate ID and a boolean creation
result; duplicates may return the original build provenance for the same immutable
candidate. Credentials stay in memory, and error output remains allowlisted.

Live activation still requires explicit registry configuration. After review and
merge, authorize the exact reviewed `main` workflow SHA and workflow path in the
existing CI policy registry, preserving immutable repository/owner IDs and subject
policy. Register `deployment-probe` as an enabled `oci` service owned by Infra with
the narrowly scoped GHCR image prefix. The old migration-branch policies do not
authorize these jobs. Configure the endpoint through the repository variable,
resume Development registration through the protected operator API, then enable
the CI opt-in for the verification run. Confirm the recorded candidate/digest and
retry deduplication through the protected API, then pause registration and remove
the CI opt-in. Keep deployment automation disabled throughout this check.

This is a prerequisite for Milestone 5. [Probe deployment orchestration](PROBE_DEPLOYMENTS.md)
now implements locks, a bounded Workflow, signed polling and authenticated results
in local runtime tests; live execution and the Panel timeline remain unfinished. The
control plane stays on Cloudflare; the native probe validates only the host adapter.
Workers/D1/R2/Access remain the preferred home for compatible product services.

GitHub ZIP artifacts expire after 14 days and serve the initial registration proof
only. Durable immutable custody is required before deploying application releases.
The disposable probe container is validation evidence, not a release artifact.

Additional actual-D1 API tests cover signed registration, concurrent candidate
and audit deduplication, scoped reads, pause/resume races, and signed webhook
replay/conflict handling. Remaining: GitHub App setup and retention/recovery.
No deploy, rollback, promotion endpoint or agent action is enabled here.

Milestone 4 enrollment/heartbeat code is subsequent local work: `0003` and those
routes are not deployed, agent API defaults disabled. See `agent/README.md`.

References: [GitHub OIDC claims](https://docs.github.com/en/actions/reference/security/oidc),
[webhook validation](https://docs.github.com/en/webhooks/using-webhooks/validating-webhook-deliveries).
