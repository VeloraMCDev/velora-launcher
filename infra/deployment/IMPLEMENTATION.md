# Deployment implementation status

The operator's `llm-instruction/deployment-guide.md` (2026-10-07) is the ordered
implementation contract. Its exact SHA-256 is recorded in `repository-audit.json`.
Read the current contract before each milestone; do not replace it with this report.

## Milestone 0: repository audit and contract freeze — DONE

All eight migration checkouts were inspected unchanged, including toolchains,
lockfiles, commands, workflow definitions, Docker/Compose files, configuration
names, auth and updater implementations. `repository-catalog.json` is the live
eight-row gap matrix; `repository-audit.json` pins inspected source and evidence.
All seven existing workflows were rerun successfully at those same revisions. Docs has
no build toolchain or CI; it is classified incomplete rather than reported green.

Inspect/rebuild evidence with Node 24 and authenticated GitHub CLI:

```sh
node scripts/audit-repositories.mjs ../ repository-output.json /path/to/deployment-guide.md
```

The audited migration branches contain the maintained extracted components;
default branches are still `main`. No migration PR has been merged. A passing
component build is not proof of a complete deployable application.

## Verified conflicts and adaptations

- **Authentication:** existing Rust/Axum/SQLite/Argon2 identity and game service
  has filesystem keys/textures, writer exclusion, explicit SQL transactions and
  mandatory account-side-effect ports. It cannot be called a Worker/D1 deployment
  by changing its Dockerfile. Benchmark password hashing and prove compatible
  storage/key/session migration before Worker identity cutover; retain the tested
  service meanwhile. OIDC, PKCE and passkeys are not present.
- **Experiences:** preserve maintained private gameplay and the frozen archive.
  The guide adds declarative bundles; it does not authorize deleting private code
  or publishing gameplay into a platform repository.
- **Launcher:** destination owns only the engine. The compatibility Tauri v2 app
  has a custom installer updater with SHA-256 validation, not the signed updater
  plugin required by the new contract. Signing custody and a tested old-install
  transition are required before shipping an auto-updating production launcher.
- **Panel:** keep its existing Svelte libraries and gateway. The Deployment Panel
  will be a client of Infra's control API, never a browser shell or deploy authority.
- **Infra:** existing Compose recipes build local tagged component images. Preserve
  those development acceptance tests; release deployments need separate manifests
  containing published immutable image digests and exact health identity.
- **Docs:** Markdown is verified source, not an existing hosted static site.
- **Other sources:** additional repositories/artifact sources must be explicitly
  registered and authorized. The eight-row audit is not a hard-coded trust grant
  to every repository under the organization or to arbitrary external URLs.

## Ordered milestones

1. **PARTIAL:** repeatable CI and immutable build/artifact provenance baseline.
2. **PARTIAL:** Development D1 stores, private R2 bucket and exact-hostname Access
   policy provisioned; binding-check Worker and migrations pass local runtime tests.
   Both remote migrations, exact CI artifact deployment and authenticated live
   Workflow proof pass. Automated CI Cloudflare publishing remains deferred.
3. **DONE:** Development candidate registration exit passed with genuine GitHub
   OIDC, exact CI artifact identity, protected reads, RBAC, audit and retry proof.
   GitHub App provisioning and full deployment orchestration remain deferred.
4. **DONE:** signed one-use enrollment/heartbeat, revocation, native Linux
   agent, first-install bootstrap and protected enrollment screen implemented.
   Development migration/roles and exact Worker/Panel artifacts are deployed.
   Linux installer CI passes; exact artifact runs as an enabled native Hermes
   service under its dedicated account with no Docker membership. Protected API
   and refreshed Panel show ONLINE, expected source SHA and heartbeat-only scope.
   Guide Milestone 4 exit passed; signed execution jobs remain Milestone 5 work.
5. **PARTIAL (happy path proven live 2026-10-08):** the chain commit -> CI -> immutable GHCR digest -> OIDC-registered candidate -> operator-created deployment -> Cloudflare Workflow -> signed job -> Hermes agent -> Docker Compose -> expected-SHA health -> signed receipt -> SUCCEEDED passed on Development (deployment 77e42791). The first attempt (982ca595) FAILED because the fixed Compose topology used an internal Docker network that cannot publish the loopback port; fixed in PR #5. Rollback drill passed live (deployment 9dcb420a, candidate from release-probe fault_drill=true): HEALTH_CHECK > ROLLING_BACK > ROLLED_BACK, previous digest healthy again, lock released, single deployment record for the drill candidate. Still open: the Panel timeline against live data, backup failure/failed-rollback/data-restore drills, scheduled transport (the proof ran the executor via a one-shot root script, not a standing service) and public/credentialed GHCR pull policy. Deployment flags were switched off after the proof.
6. **MISSING:** failed-image rollback proof and bounded encrypted backup/restore.
7. **MISSING:** full deployment UI after the vertical slice is reliable.
8–13. **MISSING:** Beta promotion, Production guards, signed launcher distribution,
   experience/integration releases, identity hardening and go-live failure drills.

Hermes is operator-confirmed Linux Mint with Docker. Target a native systemd agent
with outbound HTTPS. SSH is optional for bootstrap/recovery, not normal deployment.
Development/Beta need separate projects, persistence, credentials and policy.
Production target remains UNKNOWN. Development resource configuration is kept in
ignored operator files. No agent or production workload has been provisioned.

## Milestone 1 implementation checkpoint ? PARTIAL, CI verified

Pinned remaining setup-node/setup-java actions to verified official commit SHAs
without changing toolchain versions or lockfiles. All eight owners now have bounded
CI jobs and disabled checkout credential persistence. Docs gains a real local-link
checker, sanitized static site, deterministic checksummed output and pinned CI.

Infra owns canonical v1 artifact/state schemas and a schema-backed metadata helper.
It rejects mutable OCI tags, missing/mismatched checksums, failed build/test status,
unknown fields/future schema, forged candidate identity and credential-bearing URLs.
Stable candidate IDs bind repository, source SHA, service, artifact kind and checksum.
Other source repositories remain structurally supported but require explicit API
registration/authorization; metadata never establishes trust on its own.

The stateless Development probe and main-only/manual release workflow are implemented.
A release builds once, tests expected-SHA health, then publishes those same bytes and
captures an exact GHCR digest plus checksummed candidate handoff (14-day retention).
The release workflow has not been executed and no package was published. Hosted CI has validated the disposable image because this Windows harness has no Docker CLI.
Local Node tests (seven), eight-owner workflow policy checks and Docs link tests pass.

Still open: actual GHCR release, authenticated/durable candidate registration,
artifact outputs/releases for the other deployable owners, signing custody,
attestation policy and production release proof. Reusable cross-repository workflows
must not make public Authentication depend on private Infra access; defer that sharing
until repository visibility/access policy is explicitly configured and tested.
No control API, agent or complete deployment UI is claimed to exist yet.

## Milestone 2 implementation checkpoint ? PARTIAL

Development binding checks run in an actual local Workers runtime with two D1
stores, R2 and Workflows. Both expand-only migrations are safely reapplied. Tests
prove signed Access JWT checks, exact operator admission, expected source identity,
bounded Workflow retries, success/failure records and synthetic object/row cleanup.
Bootstrap defaults disabled; the API cannot deploy a service or run a shell.

CI retains the compiled Worker module and a file-checksum manifest for 14 days.
Deploy this bundle without rebuilding and pin its source SHA in the live config.
Live account IDs, operator identity and exact hostnames stay in ignored configs.
Hermes has a read-only preflight script; SSH, architecture and systemd agent
installation are not yet verified. The full control API and Panel remain missing.

## Live Development verification ? 2026-10-07

The binding-check bundle from successful Infra push run 37665751824 at
`aed1f0418175d5f35131f432dcabfa85d203113a` was deployed without rebuilding.
Both remote initial migrations applied and safely reapplied. A provisioner-level
Workflow and a separate signed-Access browser request completed all five checks
with cleanup; only two successful small harness records remain. R2 contains zero
objects and the identity proof table is empty. Bootstrap writes were disabled
again and the authenticated endpoint returned `403 BOOTSTRAP_DISABLED`.

Docs push run 37662238288 at `ff534ec83f3437f1b350b13f306a7764213e6453`
provided the exact static artifact; all 19 file checksums matched. The protected
live site loaded its images without horizontal overflow. Both custom-domain
certificates are Active and unauthenticated clients are redirected to Access.
The report in Docs `deployment/MILESTONE_2.md` records this checkpoint. Live
configuration/evidence remains ignored; no operator URLs or keys are committed.

Hermes SSH is active; its host fingerprint was independently verified. The local
operator is preparing Blade key authentication. The native agent is still absent.
Milestone 3 identity/model helpers are local work, not deployed API endpoints.

## Milestone 3 foundation checkpoint - PARTIAL

Local expand-only models, GitHub OIDC identity policy, original-byte HMAC webhook
verification, environment-scoped operator roles, atomic delivery deduplication
and retry-safe audit statements pass 17 local tests, strict TypeScript and Worker
dry-run. The migrations and helpers are not applied to live databases or wired
to live routes. No CI candidate registration is claimed. See
`control-plane/REGISTRATION.md` for the integration gap.

## Milestone 3 API checkpoint - PARTIAL

The local Worker now routes bounded signed CI registration, role-scoped reads,
informational HMAC webhooks and idempotent registration pause/resume. Canonical
artifact validation uses generated standalone code checked by CI. Atomic D1
tests prove concurrent candidate/audit deduplication and conflicting-action denial.
All 19 local tests, TypeScript and Worker dry-run pass. OpenAPI and the opt-in
OIDC-only CI handoff are included. Registration remains disabled by default;
deployment is never enabled. Live upgrade, explicit trust bootstrap and genuine
CI registration/read proof remain open. GitHub ZIP custody expires in 14 days;
durable release custody, agent and deployment actions remain future milestones.

## Milestone 3 live exit proof - PASSED

Source `fcc2f293f626d4e9c287ecad51e659511726c3de` passed all four push/PR
deployment/preflight checks. Its exact checksummed compiled artifact is live;
both remote `0002` migrations preserved the earlier binding-check records.
Run 37688370959 registered one genuine OIDC candidate for the existing successful
build artifact. A registration-only retry returned the same checksum/candidate
with created=false. The protected API returned exact metadata and artifact with
200; Production read and bootstrap returned 403. Registration was paused again
through the audited API, CI opt-in disabled and all deployment flags remain zero.
The Docs Milestone 3 report records evidence and deferred GitHub App/custody work.

## Milestone 4 local implementation - PARTIAL

Rust CLI/daemon and Worker models/routes implement Development-only, hash-only
one-use enrollment, Ed25519 requests, nonce replay protection, heartbeat capability,
scoped reads and revoke. Shared synthetic vectors verify Node/Worker/Rust signing.
Native systemd service and Linux CI bootstrap artifact packaging are prepared.
Agent API defaults disabled; `0003` is not remote. The installer, operator screen,
Linux CI artifact and real Hermes enrollment/healthy heartbeat remain pending.
No host agent or workload is installed by this checkpoint.

Hermes key SSH authentication and the independent host fingerprint are verified.
Its read-only preflight passed: x86_64, systemd, Docker, Compose 5.0.2, about 54 GiB
free. Sudo remains password-gated. No agent or containers were installed.
