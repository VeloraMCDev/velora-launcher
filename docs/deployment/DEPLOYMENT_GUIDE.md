# Velora — Final Implementation, Deployment & Operations Specification

> **Status:** Implementation-ready final-stage specification  
> **Review date:** 2026-10-07  
> **Primary audience:** LLM coding agents working across the Velora repositories, plus human reviewers/operators  
> **Repository set:** `experiences`, `infra`, `docs`, `authentication`, `panel`, `sdk`, `minecraft-integrations`, `launcher`  
> **Core platforms:** GitHub, GitHub Actions, GHCR, GitHub Releases, Cloudflare Workers / Static Assets, D1, R2, Workflows, Access, Tunnel, Hermes/local infrastructure, production VPS  
> **Primary objective:** Turn the existing codebase into a safe, reproducible, observable Development → Beta → Production delivery system without unnecessarily rewriting working code.

---

# 0. LLM EXECUTION DIRECTIVE

This document is not a brainstorming document. Treat it as the **implementation contract and ordered task list** for completing Velora's deployment platform.

The implementing LLM MUST follow these rules:

1. **Audit before editing.** Inspect all eight repositories and identify what already exists, what is partial, what is missing, and what conflicts with this specification.
2. **Do not replace working systems merely to match example folder structures in this file.** Adapt the contract to the existing project unless the existing implementation violates a security, correctness, or deployment invariant.
3. **Do not invent repository contents.** If a file, command, environment, schema, endpoint, or service cannot be verified, mark it `UNKNOWN` until inspected.
4. **Preserve lockfiles and existing toolchains.** Upgrade dependencies only when the upgrade is required for correctness/security or explicitly part of the task.
5. **Never generate, commit, print, or persist real secrets.** Create placeholders and `.env.example`/configuration documentation instead.
6. **Never force-push, rewrite shared history, delete production resources, drop databases, erase volumes, or restore production data without explicit human authorization.**
7. **Do not use "latest" as the authoritative deployable version.** Deploy immutable artifacts identified by image digest and/or cryptographic checksum.
8. **Build once, promote the same artifact.** Development, Beta, and Production must not each build unrelated binaries from the same release.
9. **Do not make Git branches the environment state database.** Environment state belongs in the Velora control plane. Source branches represent source history.
10. **Do not expose a generic remote shell from the Deployment Panel.**
11. **Do not expose Docker's socket or daemon over the network.**
12. **Do not give agents broad Cloudflare or GitHub account tokens.** Agents authenticate only to the Velora control plane using per-agent credentials.
13. **Every production-changing operation must be authenticated, authorized, auditable, idempotent, and recoverable.**
14. **Every deployable service must have a machine-verifiable health check.**
15. **Application rollback may be automated. Data/database restore must be a separate, explicitly confirmed operation.**
16. **Run tests and regression checks before marking any milestone complete.**
17. **Use official upstream documentation for uncertain platform behavior. Do not guess Cloudflare, GitHub, Tauri, OAuth/OIDC, Docker, or Minecraft behavior.**
18. **Prefer the smallest architecture that satisfies the requirements.** Do not introduce Kubernetes, a service mesh, a custom container runtime, or a new message broker unless a verified requirement makes it necessary.
19. **Keep commits small enough to review.** Commit at milestone boundaries with descriptive messages.
20. **Maintain a live implementation matrix during work.** At minimum track `EXISTING`, `PARTIAL`, `MISSING`, `BLOCKED`, `DONE`, and verification evidence.

Before implementing, produce a table equivalent to:

| Repository | Current stack | Build command | Test command | Deployable outputs | Existing CI | Existing deployment | Gaps | Status |
|---|---|---|---|---|---|---|---|---|
| experiences | ... | ... | ... | ... | ... | ... | ... | ... |
| infra | ... | ... | ... | ... | ... | ... | ... | ... |
| docs | ... | ... | ... | ... | ... | ... | ... | ... |
| authentication | ... | ... | ... | ... | ... | ... | ... | ... |
| panel | ... | ... | ... | ... | ... | ... | ... | ... |
| sdk | ... | ... | ... | ... | ... | ... | ... | ... |
| minecraft-integrations | ... | ... | ... | ... | ... | ... | ... | ... |
| launcher | ... | ... | ... | ... | ... | ... | ... | ... |

Do not begin large architectural edits until this matrix is complete.

---

# 1. FINAL ARCHITECTURAL DECISION

Velora is split into four planes:

```text
SOURCE / BUILD PLANE
GitHub + GitHub Actions + self-hosted runners
          |
          | produces immutable artifacts
          v
ARTIFACT PLANE
GHCR + GitHub Releases + R2
          |
          | exact digest/checksum
          v
CONTROL PLANE
Cloudflare Workers + D1 + Workflows + R2
Velora Deployment Panel + Velora Identity
          |
          | signed/authenticated desired-state jobs
          v
EXECUTION PLANE
Velora Agent on Hermes / managed nodes / VPS
Docker Compose + host-local operations
```

The most important architectural rule is:

> **GitHub builds. Cloudflare orchestrates. The Velora Agent executes local host changes. The artifact itself is immutable between environments.**

A second rule is:

> **The browser never directly controls infrastructure.**

The browser talks to the Velora Control Plane API. The Control Plane validates identity, authorization, state, approvals, locks, artifact identity, and policy before an execution request exists.

A third rule is:

> **The Velora Agent is not a remote shell.**

It supports a finite list of typed operations. Arbitrary commands are not accepted from the network.

---

# 2. 2026 PLATFORM DECISIONS

## 2.1 Cloudflare frontend hosting

For **new Cloudflare frontend deployments**, prefer **Workers Static Assets**. Cloudflare currently recommends Workers Static Assets for new static, SPA, and full-stack projects; Pages remains supported.

Therefore:

- Existing working Cloudflare Pages deployments **do not need to be rewritten solely because of this recommendation**.
- New Velora frontend deployment code should target Workers Static Assets unless the existing repo is already correctly deployed with Pages and there is no practical benefit to changing it.
- Keep frontend hosting behind the same Cloudflare account/domain strategy either way.
- Do not use GitHub Pages for the transactional Velora application or deployment panel.

GitHub Pages remains useful for optional, purely static, policy-compliant developer/reference documentation.

## 2.2 Deployment orchestration

Use **Cloudflare Workflows** as the preferred durable orchestrator for deployment sequences.

Reasons:

- durable multi-step execution;
- automatic retry/error handling;
- persisted step progress;
- waiting for external events;
- human-in-the-loop approval support;
- better fit for multi-minute deployment workflows than manually chaining Worker requests.

D1 remains the authoritative Velora business/audit state. Workflows are the durable execution engine.

Cloudflare Queues remain useful for asynchronous event fan-out or bounded background jobs, but are **not required as the primary deployment state machine** once Workflows is used.

## 2.3 Agent transport

Agents MUST poll or long-poll the **Velora Control Plane API**.

Do not have every agent directly pull Cloudflare Queues using a Cloudflare account API token. That would unnecessarily distribute Cloudflare credentials to managed machines.

Recommended initial transport:

```text
agent -> HTTPS -> control plane
```

Idle polling:

```text
15–30 seconds + random jitter
```

Error handling:

```text
exponential backoff with a sane maximum
```

A later WebSocket or long-lived channel is optional, not required for v1.

## 2.4 Production approval

The **Velora Deployment Panel is the authoritative Production approval system**.

Do not depend on GitHub Environment "required reviewers" for this design. Most Velora repositories are private, and on GitHub Free/Pro/Team, required reviewers for environments are limited to public repositories.

GitHub Environments may still be used for:

- environment-specific secrets;
- variables;
- deployment history;
- environment naming.

But the production safety gate lives in Velora.

---

# 3. THE EIGHT REPOSITORIES

No ninth repository should be created simply to make the architecture look symmetrical. If a main website already lives in an existing repository, preserve it unless a separate migration is explicitly approved.

## 3.1 `experiences`

**Purpose:** Declarative source of truth for Velora Experiences.

This repository SHOULD contain:

- experience definitions;
- experience metadata;
- versions/channels;
- required Minecraft/modpack/integration versions;
- feature flags that are safe to store in Git;
- branding/theme references;
- server/profile definitions;
- resource references;
- schema-validated configuration;
- optional static experience assets if reasonably sized.

It MUST NOT contain:

- production secrets;
- access tokens;
- private signing keys;
- raw production database state;
- mutable runtime state;
- credentials for game nodes.

Recommended conceptual structure:

```text
experiences/
├── schemas/
│   ├── experience.schema.json
│   └── release.schema.json
├── experiences/
│   ├── <experience-id>/
│   │   ├── experience.yml
│   │   ├── development.yml
│   │   ├── beta.yml
│   │   └── production.yml
│   └── ...
├── tools/
├── tests/
└── .github/workflows/
```

Do not force this layout if a strong equivalent already exists.

### CI requirements

- [ ] Validate every experience against the canonical schema.
- [ ] Reject duplicate IDs.
- [ ] Reject unknown required integration IDs.
- [ ] Validate referenced mod/plugin/version constraints.
- [ ] Validate URLs/domains where applicable.
- [ ] Ensure no obvious secrets are committed.
- [ ] Build a deterministic "experience bundle" if the runtime consumes one.
- [ ] Generate a SHA-256 checksum for bundles.
- [ ] Register the exact Git commit + checksum with the control plane.
- [ ] Do not auto-promote a malformed experience config.

### Deployment behavior

Experience configuration is promoted as an immutable config revision:

```text
Git SHA
+ config bundle checksum
+ schema version
= Experience Revision
```

D1 stores which revision is active per environment.

The runtime must never fetch "whatever is currently on main" during startup.

---

## 3.2 `infra`

**Purpose:** Velora infrastructure/control-plane source of truth.

This is the most important deployment repository.

It SHOULD own, unless equivalent code already has a better verified home:

- Velora Control Plane Worker/API;
- Cloudflare Workflows deployment definitions;
- `velora_harness` D1 schema/migrations;
- Velora Agent;
- agent protocol types;
- deployment manifest schemas;
- OpenAPI specification for the deployment API;
- reusable GitHub Actions workflows;
- environment topology/manifests;
- Docker Compose production/development stack definitions;
- Cloudflare Wrangler/OpenTofu/Terraform definitions where useful;
- agent bootstrap scripts;
- deployment policies;
- operational runbooks.

Suggested conceptual layout:

```text
infra/
├── control-plane/
│   ├── src/
│   ├── migrations/
│   ├── workflows/
│   ├── tests/
│   └── wrangler.jsonc
├── agent/
│   ├── crates/
│   ├── src/
│   └── tests/
├── contracts/
│   ├── openapi/
│   ├── agent-protocol/
│   └── schemas/
├── environments/
│   ├── development/
│   ├── beta/
│   └── production/
├── github/
│   └── reusable-workflows/
├── cloudflare/
├── scripts/
└── runbooks/
```

Again: **adapt, do not blindly restructure.**

### `infra` deployment outputs

Potential outputs are different classes and MUST be tracked independently:

1. `velora-control-plane` Worker deployment bundle.
2. `velora-agent` native binaries.
3. Optional agent package/service installer.
4. Environment/Compose manifest revisions.
5. Reusable workflow revisions.
6. Schemas/OpenAPI contracts.

### Critical rule

The Velora Agent should normally run as a **native system service**, not a Docker container that needs the Docker socket mounted into it. A containerized agent with `/var/run/docker.sock` is effectively privileged and complicates bootstrap/recovery.

---

## 3.3 `docs`

**Purpose:** Public and internal documentation.

Recommended deployment:

```text
Primary docs:
Cloudflare Workers Static Assets
(or retain an existing healthy Cloudflare Pages deployment)

Optional static developer/API mirror:
GitHub Pages
```

GitHub Pages must not become the core Velora application backend.

### CI requirements

- [ ] Build docs from a clean checkout.
- [ ] Fail on broken internal links where tooling supports it.
- [ ] Validate code examples where practical.
- [ ] Generate API reference from the canonical OpenAPI contract rather than duplicating endpoint definitions by hand.
- [ ] Generate SDK reference from actual SDK source where practical.
- [ ] Preview pull requests.
- [ ] Publish only after successful build.
- [ ] Stamp docs with product/release version when docs are versioned.

### Deployment

Docs are static and do not need the Velora Agent.

---

## 3.4 `authentication`

**Purpose:** Velora Identity and authentication protocols.

Target responsibilities:

- Velora user identity;
- web sessions;
- passkeys/WebAuthn;
- password authentication if retained;
- email verification/recovery;
- OAuth 2.0 authorization server behavior;
- OpenID Connect provider behavior;
- OIDC Discovery;
- JWKS publication;
- authorization code + PKCE;
- token rotation/revocation;
- Velora RBAC integration;
- launcher authentication;
- game-specific token issuance;
- Minecraft/Yggdrasil-compatible protocol adapter if required.

Recommended runtime:

```text
Cloudflare Worker
+ velora_identity D1
+ R2 only for file/blob identity assets if needed
```

Do not require a persistent VPS container unless a verified feature cannot run correctly/safely in the Worker architecture.

### Required OIDC endpoints

At minimum, where applicable:

```text
GET  /.well-known/openid-configuration
GET  /.well-known/jwks.json

GET  /oauth/authorize
POST /oauth/token
POST /oauth/revoke
GET  /oauth/userinfo
```

Optional endpoints must be implemented only if the clients need them.

### OAuth requirements

- [ ] Authorization Code flow.
- [ ] PKCE required for public clients.
- [ ] `S256` PKCE.
- [ ] Exact registered redirect URI matching except standards-defined loopback behavior.
- [ ] No Resource Owner Password Credentials grant.
- [ ] No Implicit grant for new clients.
- [ ] `state`/CSRF protections.
- [ ] `nonce` for applicable OIDC flows.
- [ ] Short-lived access tokens.
- [ ] Refresh-token rotation.
- [ ] Refresh-token replay/reuse detection where practical.
- [ ] Store opaque token hashes instead of plaintext tokens where possible.
- [ ] Publish only public keys in JWKS.
- [ ] Support key rotation using key IDs (`kid`).
- [ ] Keep signing private keys in secret storage, never D1 plaintext or Git.
- [ ] Separate web/session, OAuth, game, and agent credential classes.

### Password requirements

Prefer passkeys as a first-class mechanism.

If passwords are supported:

- use a vetted password-hashing implementation;
- prefer Argon2id when the runtime/implementation can do so safely;
- do not invent a password hash;
- do not use raw SHA-256/SHA-512 as password storage;
- rate-limit login and recovery;
- protect abusive flows with Turnstile/risk checks;
- benchmark cost on the actual runtime.

Because Workers' Node-compatible crypto surface is not a reason to downgrade password security, do not silently replace Argon2id with a weak construction just for convenience.

### Game auth isolation

A Minecraft/game token must never authorize:

- deployment creation;
- backups;
- infrastructure changes;
- user-role administration.

---

## 3.5 `panel`

**Purpose:** Velora browser UI for users/admins/deployments, according to the existing application scope.

For deployment architecture, the important rule is:

> **The Panel is a client of the Control Plane API. It is not the deployment authority by itself.**

The browser must not contain:

- GitHub App private keys;
- GitHub PATs;
- Cloudflare API tokens;
- GHCR credentials;
- R2 secret keys;
- agent private keys;
- production SSH keys.

Recommended hosting:

```text
Cloudflare Workers Static Assets
or retain an existing healthy Pages deployment
```

Recommended admin protection:

```text
Internet
  -> Cloudflare Access
  -> Velora login
  -> server-side Velora RBAC
  -> Control Plane API
```

Cloudflare Access is the outer gate. Velora RBAC is the application authorization system.

### Required deployment UI areas

```text
Overview
Environments
  - Development
  - Beta
  - Production
Services
Experiences
Release Sets
Deployment Candidates
Deployments
Nodes / Agents
Backups
Artifacts
GitHub
Audit Log
Settings
Emergency Controls
```

### A production deployment screen MUST display

- current version;
- target version;
- Git commit;
- artifact digest/checksum;
- source repository;
- build status;
- test status;
- Beta verification status;
- target node(s);
- health-check definition;
- migration flag;
- backup requirement;
- current backup status;
- config/environment revision;
- known warnings;
- approval actor;
- timestamp.

The button must not merely call `docker compose pull`.

It creates an authorized deployment command through the Control Plane.

---

## 3.6 `sdk`

**Purpose:** Stable client/developer SDK and protocol types.

The repository is currently Rust-oriented and SHOULD remain strongly typed.

CI baseline:

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Add dependency/license/security tooling if compatible with the current project, such as `cargo audit`/`cargo deny`, without making development unusable.

### SDK rules

- [ ] Public API follows Semantic Versioning.
- [ ] Breaking changes require a major version or explicit pre-1.0 policy.
- [ ] Protocol types should derive from or validate against canonical contracts.
- [ ] Do not duplicate deployment state enums differently in several repositories.
- [ ] Generate API clients from OpenAPI where doing so improves consistency.
- [ ] Hand-written ergonomic wrappers may sit over generated clients.
- [ ] Release artifacts include checksums.
- [ ] Release artifacts should receive build provenance/attestations when practical.
- [ ] Examples compile in CI.
- [ ] Never embed production service credentials.

GitHub Releases are appropriate for versioned public SDK binaries/assets. A package registry can be added if the SDK is actually distributed as a package.

---

## 3.7 `minecraft-integrations`

**Purpose:** Velora's Minecraft-side Paper/Velocity/Fabric/Forge/NeoForge integrations according to supported targets.

Do not assume all loaders are already implemented. Audit the project first.

### CI

- [ ] Preserve the current Gradle multi-project structure if healthy.
- [ ] Build each supported module.
- [ ] Run shared/common tests.
- [ ] Run loader/platform-specific tests.
- [ ] Run integration tests against representative supported server/client versions where feasible.
- [ ] Verify Java toolchain versions.
- [ ] Verify artifacts do not accidentally include secrets/config.
- [ ] Generate deterministic artifact names.
- [ ] Generate SHA-256 checksums.
- [ ] Publish stable/prerelease JARs to GitHub Releases.
- [ ] Mirror artifacts to R2 if Launcher/Modpack Manager needs controlled/CDN distribution.
- [ ] Register exact artifact metadata with the control plane.

### Runtime install rule

Experience definitions reference an **exact artifact version/checksum**.

Never tell a game server:

```text
download latest.jar
```

Tell it:

```text
artifact ID
version
SHA-256
download URL or R2 object key
```

The node verifies the checksum before activation.

---

## 3.8 `launcher`

**Purpose:** Velora desktop launcher.

If the current implementation is Tauri v2, retain Tauri v2 unless a verified blocker exists.

### Release pipeline

```text
source SHA
  -> tests
  -> platform builds
  -> code signing where configured
  -> Tauri update signing
  -> checksums
  -> GitHub Release
  -> optional R2 mirror
  -> update manifest
```

### Critical Tauri updater rule

Tauri v2 updater signatures are mandatory and cannot be disabled.

Therefore:

- [ ] Generate the updater keypair once using the official Tauri process.
- [ ] Ship only the updater public key in the application.
- [ ] Store the updater private key as a protected CI secret.
- [ ] Back up that private key securely outside the repo.
- [ ] Never log it.
- [ ] Never commit it.
- [ ] Plan key custody before shipping the first production auto-updating launcher.

Losing the updater private key can prevent existing installations from accepting future signed updates.

### Platform builds

Use appropriate runners:

```text
Windows -> Windows runner
Linux   -> Linux runner
macOS   -> macOS runner
```

A self-hosted Windows runner is useful for frequent Windows builds. Use GitHub-hosted macOS where practical unless dedicated Mac build infrastructure exists.

### Distribution

Recommended canonical release:

```text
GitHub Release
```

Optional optimized delivery:

```text
GitHub Release -> verified mirror -> R2 -> updates.velora.*
```

The update manifest must contain exact:

- version;
- platform/architecture;
- download URL;
- updater signature;
- release date;
- optional notes;
- checksum if separately used by Velora;
- minimum launcher version if needed.

---

# 4. SOURCE-CONTROL AND PROMOTION MODEL

## 4.1 Preferred model

Prefer:

```text
feature/fix branches
       |
       v
      PR
       |
       v
     main
       |
       | build once
       v
Immutable Release Candidate
       |
       +--> Development (automatic)
       |
       +--> Beta (promotion)
       |
       +--> Production (explicit approval)
```

Do **not** rebuild unrelated environment binaries from `dev`, `beta`, and `main` branches.

Environment promotion must promote artifact identity, not merely merge source between branches.

## 4.2 Existing `dev` / `beta` branches

If existing repositories already use long-lived `dev` and `beta` branches, do not immediately delete them.

During migration:

- preserve existing development workflow;
- ensure artifacts are identified by commit SHA/digest;
- ensure Beta and Production can use a previously built artifact;
- remove branch-as-environment coupling only after the new promotion model is proven.

## 4.3 Release candidate identity

Every candidate must include enough information to reproduce and audit it:

```yaml
candidate_id: rc_...
repository: owner/repository
git_sha: full-40-character-sha
git_ref: refs/heads/main
build_run_id: ...
created_at: ...
artifact_type: oci | binary | static_bundle | worker_bundle | jar | config_bundle
artifact_uri: ...
sha256: ...
oci_digest: sha256:... # if OCI
provenance_id: ...      # if generated
sbom_ref: ...           # if generated
status: READY
```

Environment status points to the candidate/release set that is actually active.

---

# 5. ARTIFACT RULES

## 5.1 Containers / GHCR

Use GHCR for Docker/OCI images.

A production deployment MUST resolve to an immutable digest:

```text
ghcr.io/<owner>/<image>@sha256:<digest>
```

Tags are human conveniences:

```text
sha-<full-git-sha>
v1.4.2
beta
latest
```

but the deployment record uses the digest.

### Image best practices

- [ ] Multi-stage build.
- [ ] Minimal runtime image.
- [ ] Run as non-root where the application permits it.
- [ ] No build secrets in final layers.
- [ ] No `.env` copied into image.
- [ ] OCI labels:
  - `org.opencontainers.image.source`
  - `org.opencontainers.image.revision`
  - `org.opencontainers.image.version`
  - `org.opencontainers.image.created`
- [ ] Pin critical base images by digest where practical.
- [ ] Expose a health endpoint where applicable.
- [ ] Generate SBOM/provenance for important releases.
- [ ] Retain enough prior digests for rollback.

### Private GHCR images

If images remain private, managed nodes need read access.

Use a dedicated, low-privilege credential with only required package/repository read permissions. Do not use a developer's broad personal token.

If a package can safely be public, anonymous GHCR pulls simplify runtime credentials.

## 5.2 GitHub Releases

Use GitHub Releases for immutable, human-versioned downloadable project artifacts such as:

- launcher installers;
- stable Minecraft integrations;
- SDK binaries;
- Velora Agent installers/binaries;
- checksums;
- release notes.

Use R2 for:

- mutable or application-managed objects;
- private distribution;
- modpacks;
- generated bundles;
- user content;
- backups;
- logs;
- skins/capes;
- CDN mirrors.

## 5.3 GitHub Actions artifacts

Actions artifacts are **temporary CI handoff/debug objects**, not permanent product storage.

Set explicit retention periods.

Examples:

```text
test reports: 7–14 days
PR build artifacts: 7–14 days
release candidate intermediate bundles: 14–30 days
```

Permanent releases belong in Releases/GHCR/R2.

---

# 6. GITHUB ACTIONS STANDARD

Every repository should have a consistent workflow vocabulary.

Recommended:

```text
ci.yml
release.yml
deploy-cloudflare.yml        # only when relevant
```

or equivalent reusable workflows.

## 6.1 Pull request CI

A PR MUST NOT deploy Production.

PR jobs may:

- lint;
- format-check;
- type-check;
- test;
- build;
- scan;
- generate preview frontend deployments;
- publish short-lived preview artifacts.

## 6.2 Main build

Merge to `main`:

1. clean checkout;
2. restore safe caches;
3. install exact locked dependencies;
4. test;
5. build;
6. create artifact;
7. compute digest/checksum;
8. generate provenance/attestation where practical;
9. publish candidate artifact;
10. register artifact/candidate with Velora;
11. optionally auto-deploy Development.

## 6.3 GitHub token permissions

Set the default as narrowly as possible.

Example principle:

```yaml
permissions:
  contents: read
```

Elevate per job only where required:

```yaml
packages: write
id-token: write
attestations: write
```

Do not give all jobs `write-all`.

## 6.4 Third-party actions

Pin third-party actions to a **full commit SHA**.

Do not trust floating major tags for security-sensitive release jobs.

Dependabot/Renovate can be used to propose action SHA updates.

## 6.5 Self-hosted runners

Treat self-hosted runner execution as sensitive.

Rules:

- [ ] Do not run arbitrary untrusted fork code on privileged self-hosted runners.
- [ ] Keep public-repo workflows away from production-connected self-hosted runners unless deliberately sandboxed/ephemeral.
- [ ] Separate runner groups/labels by trust and capability.
- [ ] Do not store production secrets persistently in a runner workspace.
- [ ] Clean workspaces between builds.
- [ ] Prefer ephemeral runners for higher-risk/untrusted work if implemented later.
- [ ] Give build runners no SSH path to Production simply because they build releases.

Suggested labels:

```text
self-hosted
linux
x64
velora-build

self-hosted
windows
x64
velora-windows
```

## 6.6 Reusable workflows

Centralize common workflow logic rather than copy-pasting eight divergent pipelines.

Good candidates:

- Rust checks;
- Node/TypeScript checks;
- Gradle build;
- OCI build/push;
- provenance;
- candidate registration;
- Cloudflare deploy;
- release asset upload.

When private repositories call reusable workflows from another repository, configure repository access deliberately.

Pin cross-repository reusable workflows to a SHA for production/release use.

---

# 7. CI -> VELORA CONTROL PLANE AUTHENTICATION

Preferred design: **GitHub Actions OIDC** for build/candidate registration, avoiding a permanent shared CI token.

Conceptual flow:

```text
GitHub Actions
  |
  | request OIDC token
  | audience = velora-control-plane
  v
Velora Control Plane
  |
  | validate signature/JWKS + claims
  v
Register exact artifact
```

The Control Plane must validate at minimum:

- issuer;
- audience;
- repository owner;
- repository name;
- commit SHA;
- ref;
- workflow identity where applicable;
- token expiry/not-before.

Then authorize only known Velora repositories.

If OIDC is not implemented in the first milestone, use a narrowly scoped temporary CI credential, then replace it with OIDC before Production.

Do not use one unrestricted static token shared across all repositories forever.

---

# 8. GITHUB APP

Create one private GitHub App for Velora automation.

Responsibilities:

- receive repository/workflow/release webhooks;
- read repository metadata;
- read Action run status;
- link commits/workflows to candidates;
- optionally dispatch approved deployment workflows for Cloudflare-hosted components.

Start with the minimum GitHub App permissions that meet those requirements.

Do not request repository administration permission simply for convenience.

## 8.1 Webhook verification

Every incoming webhook must verify GitHub's HMAC signature.

Use:

```text
X-Hub-Signature-256
```

and the configured webhook secret.

Deduplicate using:

```text
X-GitHub-Delivery
```

Store delivery IDs in `github_deliveries` with a unique constraint.

If a delivery ID already exists, return a successful idempotent response without duplicating state.

## 8.2 Webhook behavior

A push event is not proof of a deployable artifact.

A workflow success or authenticated artifact registration must establish build readiness.

The control plane must ignore:

- failed builds;
- cancelled builds;
- unknown repositories;
- unknown SHAs;
- artifacts whose checksum/digest is missing;
- mismatched repository/claim data.

---

# 9. CLOUDFLARE RESOURCE MODEL

Recommended logical resources:

```text
Workers / Static Assets
├── velora-control-plane
├── velora-auth
├── velora-panel
└── velora-docs

D1
├── velora_harness
└── velora_identity

R2
├── velora-assets
├── velora-backups
├── velora-logs
└── velora-releases-or-mirrors   # if separate bucket is useful

Workflows
├── deployment-workflow
├── rollback-workflow
└── backup-workflow              # optional separate workflow

Access
└── admin/deployment applications

Tunnel
├── selected private web origins
└── optional internal service access
```

Keep resource count rational. Do not split Workers or buckets purely for aesthetics.

---

# 10. D1 DATABASE RESPONSIBILITIES

## 10.1 `velora_harness`

Recommended tables:

```text
repositories
services
environments
artifacts
deployment_candidates
release_sets
release_set_items

agents
agent_nonces
agent_capabilities

deployments
deployment_steps
deployment_locks
jobs
job_events
approvals

backups
health_checks

github_deliveries
automation_flags
audit_events
```

## 10.2 `velora_identity`

Recommended tables:

```text
users
user_emails
password_credentials
webauthn_credentials

sessions
oauth_clients
oauth_authorization_codes
oauth_refresh_tokens
oauth_consents

roles
permissions
role_permissions
role_bindings

experience_memberships
minecraft_profiles

security_events
```

Use actual normalization based on current code; do not force table names if equivalent data already exists.

## 10.3 Migration rules

All D1 schema changes must use committed migration files.

Never run ad-hoc production SQL as a normal deployment mechanism.

Rules:

- [ ] Sequential migration numbering/naming.
- [ ] Migration reviewed in PR.
- [ ] Migration tested against representative data.
- [ ] Backup requirement stated.
- [ ] Forward compatibility considered.
- [ ] Roll-forward plan documented.
- [ ] Destructive migration separated from code rollout when possible.

Prefer **expand-and-contract** migration patterns:

```text
1. add new column/table
2. deploy code that understands old + new
3. migrate/backfill
4. verify
5. switch reads/writes
6. later remove obsolete schema
```

Do not assume an old binary can run after an irreversible schema mutation.

---

# 11. RELEASE SETS

Velora spans multiple repositories, so Production must know the version of the **whole system**.

A `Release Set` is a named immutable snapshot of exact components.

Example:

```yaml
id: rs_2026_10_07_0042
channel: beta
created_at: 2026-10-07T16:00:00Z

items:
  authentication:
    git_sha: ...
    artifact_sha256: ...

  panel:
    git_sha: ...
    artifact_sha256: ...

  infra/control-plane:
    git_sha: ...
    artifact_sha256: ...

  minecraft-integrations:
    version: ...
    artifact_sha256: ...

  launcher:
    version: ...
    release_id: ...
```

Not every repo must be present in every deployment.

A Release Set can reference the current known-good item for unchanged components.

Production promotion should normally promote a known-good Beta release set or a precisely defined subset of it.

---

# 12. DEPLOYMENT CONTROL PLANE

The Control Plane API belongs in `infra` unless the current architecture has already established an equivalent canonical location.

It is responsible for:

- repository mapping;
- candidate registration;
- artifact verification metadata;
- environments;
- release sets;
- deployment authorization;
- approvals;
- workflow orchestration;
- agent enrollment;
- agent authentication;
- jobs;
- locks;
- backup metadata;
- health status;
- audit records;
- automation pause/resume;
- GitHub webhooks;
- dispatching Cloudflare deployment workflows when applicable.

It does **not** execute arbitrary shell commands on nodes.

---

# 13. DEPLOYMENT TYPES

A deployment target has a typed executor.

Recommended target types:

```text
AGENT_COMPOSE
CLOUDFLARE_WORKER
CLOUDFLARE_STATIC
STATIC_RELEASE_POINTER
CONFIG_BUNDLE
MINECRAFT_ARTIFACT_SET
```

## 13.1 `AGENT_COMPOSE`

Executed by a Velora Agent.

Use for:

- VPS Docker services;
- Hermes Development/Beta stacks;
- game-node services where appropriate.

## 13.2 `CLOUDFLARE_WORKER`

Prefer a GitHub Actions deploy workflow using environment-scoped Cloudflare credentials.

The panel authorizes the deployment, then the control plane dispatches the workflow with:

- candidate ID;
- Git SHA;
- target environment;
- expected artifact checksum.

GitHub Actions performs the Cloudflare upload and reports status.

This avoids turning the browser or agent into a Cloudflare administrator.

## 13.3 `CLOUDFLARE_STATIC`

Same concept for Panel/docs/site artifacts.

For existing Cloudflare Pages integrations, retain the working deploy mechanism until deliberately migrated.

## 13.4 `CONFIG_BUNDLE`

Promotes an immutable experience/config bundle and changes the environment's desired revision.

Agents/services consume that exact revision.

---

# 14. DEPLOYMENT WORKFLOW STATE MACHINE

Use explicit states.

Canonical high-level states:

```text
CREATED
VALIDATING
AWAITING_APPROVAL
QUEUED
ASSIGNED
PREFLIGHT
BACKING_UP
PULLING_OR_STAGING
MIGRATING
ACTIVATING
HEALTH_CHECK
STABILIZING
SUCCEEDED

FAILED
ROLLING_BACK
ROLLED_BACK
ROLLBACK_FAILED

CANCELLED
BLOCKED
```

Not every deployment uses every state.

Every transition records:

- timestamp;
- actor type;
- actor ID;
- previous state;
- next state;
- machine-readable reason;
- optional human-readable message.

A boolean `success=true/false` is insufficient.

---

# 15. CLOUDFLARE WORKFLOW FOR A HOST DEPLOYMENT

Conceptual durable sequence:

```text
START
  |
  v
Validate candidate
  |
  v
Check policy / environment / automation flag
  |
  v
Wait for production approval if required
  |
  v
Acquire deployment lock
  |
  v
Verify agent is healthy and capable
  |
  v
Create signed job
  |
  v
Wait for agent PRECHECK result
  |
  v
Wait for backup result if required
  |
  v
Wait for deployment result
  |
  v
Evaluate health/stabilization
  |                     |
 success                failure
  |                     |
  v                     v
Record success      Issue rollback job
  |                     |
  v                     v
Release lock        Wait for rollback
                        |
                        v
                  Record rolled back/failure
                        |
                        v
                   Release lock
```

Workflow steps must be idempotent because durable workflow steps may be retried.

D1 should still store the externally visible deployment state/audit trail.

---

# 16. DEPLOYMENT LOCKING

Only one state-changing deployment may target the same logical service/environment at once.

Lock key:

```text
<environment-id>:<service-id>
```

Suggested fields:

```text
lock_key
deployment_id
owner_workflow_id
acquired_at
expires_at
heartbeat_at
```

Acquisition must be atomic.

A stale lock may be reclaimed only after:

- the owning deployment/workflow is verified dead/expired;
- a safe timeout;
- an audit event.

Do not let two deployments simultaneously mutate the same Compose project.

---

# 17. IDEMPOTENCY

All externally retried mutations require idempotency.

Examples:

```text
candidate registration:
repo + git_sha + artifact_digest

GitHub webhook:
X-GitHub-Delivery

deployment create:
caller-provided idempotency key

agent job:
job_id

agent completion:
job_id + final result version
```

A retry should return the existing object/result, not create a duplicate deployment.

---

# 18. PRODUCTION APPROVAL

Production deployments are never automatic merely because CI passed.

Minimum authorization:

```text
deployment.approve.production
```

The approval record stores:

```text
approval_id
deployment_id
actor_user_id
decision
created_at
source_ip_or_access_context
target_candidate_id
target_digest
```

The digest/checksum in the approval is important.

If the target artifact changes after approval, the approval is invalid.

For high-risk operations, require recent reauthentication or a stronger-auth policy.

Examples:

- Production deploy;
- Production data restore;
- agent revoke;
- signing-key rotation;
- administrator role assignment;
- destructive infrastructure changes.

---

# 19. VELORA AGENT DESIGN

Target language: **Rust** unless the current implementation already uses a similarly suitable language.

The agent is a small daemon on every managed machine that needs host deployment actions.

Examples:

```text
Hermes:
  Development
  Beta
  build/test support if desired

Production VPS:
  Production

Future game nodes:
  experience workloads
```

## 19.1 Filesystem layout

Recommended:

```text
/etc/velora-agent/
├── config.toml
├── identity.key
└── trust/

 /var/lib/velora-agent/
├── state/
├── jobs/
├── staging/
├── manifests/
├── cache/
└── backups/

 /var/log/velora-agent/
└── agent.log
```

Adapt paths appropriately on Windows if Windows agent support is later required.

Private identity material:

```text
0600
dedicated owner/root as appropriate
```

## 19.2 Service management

Linux:

```text
systemd
```

Expected commands may be wrapped by installer:

```text
velora-agent install
velora-agent enroll
velora-agent status
velora-agent doctor
velora-agent rotate-key
```

Do not require a network-accessible SSH server for normal operation.

---

# 20. AGENT ENROLLMENT

Enrollment flow:

```text
1. Admin requests enrollment token in Panel.
2. Control Plane generates high-entropy one-use token.
3. Store only token hash in D1.
4. Token expires quickly (for example 10–15 minutes).
5. Agent generates Ed25519 keypair locally.
6. Agent sends:
   - enrollment token
   - public key
   - generated node identity metadata
   - agent version
7. Control Plane validates token hash, expiry, intended environment/policy.
8. Token becomes USED atomically.
9. Agent record is created.
10. Agent private key never leaves the machine.
```

Enrollment tokens must be:

- one use;
- high entropy;
- time limited;
- scopeable to an environment/node policy;
- audited.

---

# 21. AGENT REQUEST AUTHENTICATION

Use signed requests.

Suggested headers:

```text
Velora-Agent-ID: agt_...
Velora-Timestamp: 1791390000
Velora-Nonce: <random>
Velora-Body-SHA256: <hex>
Velora-Signature: <base64url>
Velora-Key-ID: ...
```

Canonical signed string:

```text
METHOD
PATH
CANONICAL_QUERY
TIMESTAMP
NONCE
BODY_SHA256
```

Server validation:

1. find agent;
2. verify not revoked;
3. verify timestamp within allowed skew (for example 5 minutes);
4. reject duplicate nonce;
5. hash body and compare;
6. verify Ed25519 signature using registered public key;
7. verify endpoint/capability authorization;
8. store/expire nonce replay protection.

Do not use only `agent_id` + a reusable bearer string if the asymmetric design is implemented.

---

# 22. SIGNED CONTROL-PLANE JOBS

The agent should also verify jobs received from the control plane.

The Control Plane maintains a deployment-signing keypair.

The job includes:

```json
{
  "schema": 1,
  "job_id": "job_...",
  "deployment_id": "dep_...",
  "agent_id": "agt_...",
  "environment": "production",
  "action": "DEPLOY_COMPOSE",
  "target": "velora-api",
  "artifact": {
    "image": "ghcr.io/org/image",
    "digest": "sha256:..."
  },
  "manifest_revision": "...",
  "issued_at": "...",
  "expires_at": "...",
  "nonce": "..."
}
```

The signature covers the canonical serialized job payload.

Agent rules:

- [ ] signature valid;
- [ ] trusted key ID;
- [ ] correct agent ID;
- [ ] not expired;
- [ ] never processed nonce/job ID before;
- [ ] action enabled by local policy;
- [ ] requested paths inside allowed roots;
- [ ] requested service belongs to the agent.

This protects against accidental/tampered job data even if an intermediate layer behaves incorrectly.

---

# 23. AGENT CAPABILITIES

Explicit capability set:

```text
REPORT_HEARTBEAT
REPORT_METRICS
READ_MANAGED_LOGS

DEPLOY_COMPOSE
ROLLBACK_COMPOSE
START_SERVICE
STOP_SERVICE
RESTART_SERVICE

CREATE_BACKUP
VERIFY_BACKUP
RESTORE_BACKUP     # disabled or approval-gated by default

WRITE_MANAGED_CONFIG
RUN_HEALTH_CHECK
PRUNE_SAFE_CACHE
```

Not supported:

```text
RUN_ARBITRARY_SHELL
EXECUTE_USER_STRING
WRITE_ANY_FILE
DELETE_ANY_PATH
FORMAT_DISK
```

Future host-management operations must be individually typed and policy controlled.

---

# 24. DOCKER PRIVILEGE MODEL

Access to Docker is effectively root-equivalent on a traditional Docker host.

MVP:

- a dedicated `velora-agent` service account may be granted Docker access if risk is understood and node access is tightly controlled;
- do not expose Docker TCP;
- do not mount Docker socket into unrelated services;
- keep the agent code surface small.

Production hardening target:

```text
unprivileged velora-agent
       |
       | narrow local IPC
       v
privileged velora-helper
       |
       v
predefined host operations only
```

Do not implement a generic `sudo <string>` helper.

---

# 25. AGENT POLLING PROTOCOL

Minimum API:

```text
POST /api/v1/agents/enroll
POST /api/v1/agents/heartbeat

POST /api/v1/agents/jobs/next
POST /api/v1/agents/jobs/{id}/ack
POST /api/v1/agents/jobs/{id}/events
POST /api/v1/agents/jobs/{id}/complete
POST /api/v1/agents/jobs/{id}/fail
```

Polling behavior:

```text
normal idle: 15–30 seconds + jitter
after job: immediate next poll
network failure: exponential backoff
maximum backoff: bounded (for example 2–5 minutes)
```

The agent should be useful while the control plane is temporarily unavailable:

- currently running services stay running;
- agent does not stop workloads merely because heartbeat fails;
- queued local destructive operations are not invented;
- once connectivity returns, state reconciles.

---

# 26. HEARTBEATS

Example:

```json
{
  "agent_id": "agt_prod_01",
  "agent_version": "0.1.0",
  "uptime_seconds": 92831,
  "capabilities": [
    "DEPLOY_COMPOSE",
    "CREATE_BACKUP",
    "RUN_HEALTH_CHECK"
  ],
  "host": {
    "os": "linux",
    "arch": "x86_64"
  },
  "resources": {
    "cpu_percent": 8.1,
    "memory_total_bytes": 8589934592,
    "memory_available_bytes": 4294967296,
    "disk_free_bytes": 181293381632
  },
  "services": {
    "known": 8,
    "healthy": 8
  }
}
```

Do not send second-by-second telemetry.

A 30–60 second heartbeat is adequate initially.

Mark agents:

```text
ONLINE
DEGRADED
STALE
OFFLINE
REVOKED
```

using defined time thresholds.

---

# 27. HOST DEPLOYMENT ALGORITHM

For an `AGENT_COMPOSE` deployment:

## 27.1 Preflight

- [ ] Validate job signature.
- [ ] Validate capability.
- [ ] Validate environment/service ownership.
- [ ] Validate target artifact digest.
- [ ] Validate manifest schema.
- [ ] Validate Compose configuration.
- [ ] Validate required secrets are present locally.
- [ ] Validate required ports/paths.
- [ ] Check disk space.
- [ ] Check backup capacity.
- [ ] Check container runtime.
- [ ] Check current known-good revision.
- [ ] Refuse if another local operation holds the same service lock.

## 27.2 Backup

Only if policy requires it.

Backup persistent data, not container images.

## 27.3 Stage

- fetch immutable manifest;
- fetch/pull exact image digest;
- verify artifact;
- write generated config into an atomic staging path;
- never mutate active manifest halfway through generation.

## 27.4 Migrate

Only if deployment metadata contains an expected migration action.

Database migration must be a typed operation, not arbitrary shell from Panel.

## 27.5 Activate

For Compose, use the exact desired manifest and image digest.

Avoid environment drift from manual edits.

## 27.6 Health check

Validate:

- expected container/process is running;
- app health endpoint;
- expected version;
- expected Git SHA where available;
- critical dependencies.

## 27.7 Stabilization

Production should require repeated successful checks across a short window.

Example policy:

```text
initial delay: 5s
checks: 5
interval: 5s
required successes: 5
```

Tune by service.

## 27.8 Commit success

Record active revision locally and in D1 only after health success.

## 27.9 Failure

On application deployment failure:

- capture relevant logs;
- mark failure;
- start application rollback if policy allows;
- restore previous manifest/image digest;
- health-check previous version;
- report `ROLLED_BACK` or `ROLLBACK_FAILED`.

Do not automatically restore a database backup solely because app health failed.

---

# 28. HEALTH CONTRACT

Every deployable HTTP service should expose:

```text
GET /health
```

Recommended response:

```json
{
  "status": "ok",
  "service": "velora-auth",
  "version": "1.8.4",
  "commit": "4cd27e1...",
  "build": "..."
}
```

Optional split:

```text
/health/live
/health/ready
```

Requirements:

- liveness must not perform extremely expensive checks;
- readiness may validate required dependencies;
- no secrets;
- no stack traces;
- version/commit should match deployment expectation.

Non-HTTP services must define an equivalent typed health probe.

---

# 29. BACKUPS

R2 is the recommended remote backup target for Velora-managed application state.

Backup only persistent state that cannot simply be rebuilt.

Examples:

- database dumps;
- SQLite backups;
- Minecraft worlds;
- experience-generated data;
- application uploads not already independently stored in R2;
- critical service configuration;
- selected persistent volumes.

Do not back up:

- Docker image layers;
- dependency caches;
- build directories;
- temporary files;
- entire `/`;
- entire home directories by accident.

## 29.1 Allowed roots

Agent policy must define allowed backup roots:

```yaml
backup:
  allowed_roots:
    - /srv/velora
    - /var/lib/velora
```

A request outside the allowlist fails closed.

## 29.2 Application-aware backups

Examples:

```text
PostgreSQL -> pg_dump / verified logical or physical strategy
SQLite     -> SQLite backup-safe operation
Minecraft  -> save/flush or coordinated stop, then snapshot/archive
Files      -> deterministic archive/snapshot
```

Never blindly copy a live database file/volume if doing so can produce an inconsistent backup.

## 29.3 Backup object metadata

Store:

```text
backup_id
environment
service
deployment_id
created_at
source_revision
size_bytes
sha256
encryption_version
r2_key
verified_at
expires_at
```

## 29.4 Encryption

Encrypt sensitive backups before or during upload using a reviewed approach.

Do not treat private R2 ACL alone as backup encryption.

Key material must not live inside the backup object.

## 29.5 Retention defaults

Suggested starting policy:

```text
Development:
  daily/triggered, ~7 days

Beta:
  ~14–30 days

Production:
  30 daily
  12 weekly
  manual milestone snapshots where useful
```

Tune according to actual storage and recovery needs.

Use R2 lifecycle rules for age-based expiration where appropriate.

---

# 30. ROLLBACK

Two separate concepts:

## Application rollback

Can be automatic.

Restores:

- previous image digest;
- previous manifest revision;
- previous compatible config.

## Data restore

Potentially destructive.

Requires:

- explicit restore action;
- strong permission;
- recent approval;
- selected backup;
- confirmation of target;
- audit event;
- restore verification.

Never silently couple these.

---

# 31. SECRETS

Secrets do not belong in Git, D1 plaintext fields, browser bundles, or logs.

## 31.1 Cloudflare secrets

Use secret bindings / secure platform configuration for Worker secrets.

## 31.2 Node runtime secrets

Initial model:

```text
/etc/velora-agent/secrets/
```

or another OS-native protected storage pattern.

The Panel should normally show:

```text
DATABASE_URL: configured
JWT_SIGNING_KEY: configured
R2_BACKUP_CREDENTIAL: configured
```

not the value.

## 31.3 Future sealed-secret model

Preferred future path:

1. agent has encryption public key;
2. control plane encrypts secret for that agent;
3. only ciphertext is stored centrally;
4. agent decrypts locally;
5. browser never sees plaintext.

Do not delay the entire first deployment pipeline to build a perfect secret-management product, but do not normalize plaintext secrets in D1.

---

# 32. LOGGING AND OBSERVABILITY

Use three tiers.

## Tier 1: Structured deployment state

D1:

- start/end;
- state transitions;
- error codes;
- actor;
- artifact identity;
- health result;
- rollback result.

## Tier 2: Live/recent job logs

Panel may show bounded recent deployment logs.

Redact:

- authorization headers;
- cookies;
- secrets;
- tokens;
- private keys;
- `.env` values.

## Tier 3: Archived logs

Large logs go to R2 with lifecycle expiration.

Do not dump unbounded stdout into D1.

---

# 33. AUDIT LOG

Sensitive actions must be append-oriented.

Recommended event families:

```text
AUTH_LOGIN_SUCCESS
AUTH_LOGIN_FAILURE
AUTH_TOKEN_REVOKED

ROLE_ASSIGNED
ROLE_REMOVED

AGENT_ENROLLMENT_CREATED
AGENT_ENROLLED
AGENT_KEY_ROTATED
AGENT_REVOKED

CANDIDATE_REGISTERED
RELEASE_SET_CREATED

DEPLOYMENT_CREATED
DEPLOYMENT_APPROVED
DEPLOYMENT_CANCELLED
DEPLOYMENT_STARTED
DEPLOYMENT_SUCCEEDED
DEPLOYMENT_FAILED

ROLLBACK_STARTED
ROLLBACK_SUCCEEDED
ROLLBACK_FAILED

BACKUP_CREATED
BACKUP_VERIFIED
BACKUP_RESTORE_APPROVED
BACKUP_RESTORED

AUTOMATION_PAUSED
AUTOMATION_RESUMED
```

Audit event:

```text
id
event_type
actor_type
actor_id
target_type
target_id
environment_id
request_id
metadata_json
source_ip_or_access_context
created_at
```

Do not let ordinary application code edit old audit records.

---

# 34. EMERGENCY CONTROLS

Panel:

```text
Automation

Development  ON
Beta         ON
Production   ON

[ PAUSE NEW AUTOMATED DEPLOYMENTS ]
```

Global pause behavior:

- blocks creation/assignment of new automated deployments;
- does not kill currently healthy production workloads;
- does not interrupt a safety rollback already in progress;
- does not block agent heartbeat;
- does not block read-only monitoring.

A separate scoped pause per environment/service is useful.

---

# 35. RATE AND RUNAWAY PROTECTION

Protect expensive/mutating endpoints.

Examples:

- login;
- registration;
- recovery;
- enrollment-token creation;
- deployment create;
- deployment approve;
- backup create;
- restore;
- GitHub webhook handling;
- artifact registration.

Suggested deployment policy starting points:

```text
Development:
  generous but bounded

Beta:
  bounded

Production:
  very low automated deployment rate
  explicit approval
```

Never build recursive behavior where a deployment commit creates another deployment commit indefinitely.

Workflow/Queue retries must be finite.

Dead-letter or terminal failure must be visible.

---

# 36. CLOUDFLARE ACCESS

Use Access around:

- Deployment Panel;
- internal admin tools;
- private development dashboards;
- sensitive preview tooling.

Access controls who may reach the app.

Velora Identity/RBAC still controls what they may do inside.

Do not treat an Access-protected route as permission to skip server-side RBAC.

---

# 37. CLOUDFLARE TUNNEL

Use Tunnel for selected private HTTP/TCP origins where Cloudflare's supported model fits, especially management web services that should not require inbound public ports.

Do not design normal public Minecraft gameplay UDP/TCP transport around standard HTTP-oriented Tunnel assumptions.

Game networking remains a separate networking concern.

The deployment agent itself does not require an inbound port because it initiates outbound HTTPS.

---

# 38. PANEL API

Version every API:

```text
/api/v1/...
```

Representative control-plane endpoints:

```text
POST /api/v1/github/webhooks

POST /api/v1/ci/artifacts/register

GET  /api/v1/environments
GET  /api/v1/services
GET  /api/v1/artifacts
GET  /api/v1/candidates
GET  /api/v1/release-sets

POST /api/v1/release-sets

GET  /api/v1/deployments
POST /api/v1/deployments
GET  /api/v1/deployments/{id}
POST /api/v1/deployments/{id}/approve
POST /api/v1/deployments/{id}/cancel
POST /api/v1/deployments/{id}/rollback

GET  /api/v1/agents
POST /api/v1/agents/enrollment-tokens
POST /api/v1/agents/enroll
POST /api/v1/agents/heartbeat
POST /api/v1/agents/jobs/next
POST /api/v1/agents/jobs/{id}/ack
POST /api/v1/agents/jobs/{id}/events
POST /api/v1/agents/jobs/{id}/complete
POST /api/v1/agents/jobs/{id}/fail

GET  /api/v1/backups
POST /api/v1/backups
POST /api/v1/backups/{id}/restore-request
POST /api/v1/backups/{id}/restore-approve

GET  /api/v1/audit

POST /api/v1/automation/pause
POST /api/v1/automation/resume
```

Do not create all endpoints before the vertical slice needs them. The contract can grow while preserving `/api/v1`.

---

# 39. API ERROR CONTRACT

Use machine-readable errors.

Example:

```json
{
  "error": {
    "code": "DEPLOYMENT_LOCKED",
    "message": "A deployment is already active for this service.",
    "request_id": "req_...",
    "details": {
      "deployment_id": "dep_..."
    }
  }
}
```

Rules:

- stable error code;
- safe message;
- request ID;
- no secret values;
- no raw SQL;
- no production stack trace to clients.

---

# 40. OPENAPI AS CONTRACT

Maintain the Control Plane/OpenAPI schema in one canonical location.

Generate or validate:

- Panel client;
- Rust SDK models/client where useful;
- API docs;
- mock/test fixtures.

Do not maintain four manually divergent copies of:

```text
DeploymentStatus
Agent
ReleaseSet
Artifact
Backup
```

If code generation is not practical for a particular consumer, add contract tests against the OpenAPI schema.

---

# 41. AUTHORIZATION MODEL

Authentication answers:

```text
Who are you?
```

Authorization answers:

```text
What may you do?
```

Recommended permission examples:

```text
profile.read
profile.write

experience.read
experience.join
experience.manage

deployment.read
deployment.create
deployment.approve.beta
deployment.approve.production
deployment.rollback

backup.read
backup.create
backup.restore

agent.read
agent.enroll
agent.revoke

audit.read

admin.users
admin.roles
```

Roles map to permissions.

Do not encode authorization as UI visibility only.

The API must enforce it.

---

# 42. SESSION / TOKEN SEPARATION

Keep distinct credential classes:

```text
browser session
OAuth access token
OAuth refresh token
launcher authorization
Minecraft/game access token
agent identity
GitHub CI OIDC token
```

Do not make a single bearer token work everywhere.

Recommended browser cookie properties where applicable:

```text
Secure
HttpOnly
SameSite=Lax or stricter when compatible
```

Use CSRF protections for cookie-authenticated state-changing endpoints.

---

# 43. LAUNCHER UPDATE SECURITY

A launcher update is executable code and should be treated as a software-supply-chain event.

Release checklist:

- [ ] Source commit identified.
- [ ] Tests passed.
- [ ] Build performed in known workflow.
- [ ] Platform signing completed where configured.
- [ ] Tauri updater signature generated.
- [ ] Checksum generated.
- [ ] Provenance/attestation generated where supported.
- [ ] Release notes generated/reviewed.
- [ ] GitHub Release created.
- [ ] R2 mirror checksum verified if mirrored.
- [ ] Update manifest references immutable artifact.
- [ ] Update tested from previous stable version.
- [ ] Rollout/channel (`beta`/`stable`) confirmed.

Never let the launcher install an update solely because a URL returns HTTP 200.

---

# 44. CLOUDFLARE DEPLOYMENT FLOW

Cloudflare-hosted code does not need the host Agent.

Recommended:

```text
Panel approval / auto-dev policy
      |
      v
Control Plane
      |
      | dispatch exact candidate
      v
GitHub Actions deploy workflow
      |
      | environment-scoped Cloudflare credential
      v
Cloudflare Workers / Static Assets
      |
      v
smoke test
      |
      v
webhook/callback/status
      |
      v
Control Plane records result
```

Use a scoped Cloudflare API token, not the Global API Key.

Development can deploy automatically.

Beta may deploy on promotion.

Production requires Velora approval.

The workflow must verify the supplied candidate SHA/checksum before deployment.

---

# 45. HOST ENVIRONMENT TOPOLOGY

Initial intended topology:

```text
Hermes / local infrastructure
├── Development
└── Beta

Production VPS
└── Production
```

Rules:

- Development and Beta must use distinct Compose project names.
- Use distinct secrets.
- Use distinct persistent volumes/directories.
- Use distinct database/config resources where practical.
- Do not let Beta point at Production D1/R2 prefixes accidentally.
- Production credentials are never needed by Development test services.
- A deployment to Development must be unable to address Production merely by changing one unvalidated string.

---

# 46. COMPOSE OWNERSHIP

Service repositories own application code.

`infra` owns environment topology.

Do not allow eight repositories to each independently redefine the Production network topology.

Concept:

```text
repo service contract:
  "I produce image X and expose health Y"

infra:
  "Production runs X on node Z with network/volume/policy Q"
```

Example service contract:

```yaml
schema: 1

service:
  id: example
  artifact_type: oci

runtime:
  health:
    type: http
    path: /health

deployment:
  backup_required: true
  stabilization_seconds: 30
```

No secret values in this file.

---

# 47. PRODUCTION PREFLIGHT GATE

Production deployment cannot begin unless:

- [ ] candidate exists;
- [ ] candidate is immutable;
- [ ] CI succeeded;
- [ ] artifact digest/checksum exists;
- [ ] source SHA exists;
- [ ] required provenance/signature policy satisfied;
- [ ] Beta evidence satisfies policy;
- [ ] production approval references this exact target;
- [ ] agent is online if host deployment;
- [ ] agent version is supported;
- [ ] agent capability is present;
- [ ] no conflicting lock;
- [ ] disk reserve is sufficient;
- [ ] secrets/config prerequisites are configured;
- [ ] required backup can be created;
- [ ] migration policy is known;
- [ ] automation is not paused;
- [ ] audit store is writable.

If any required condition fails:

```text
DO NOT DEPLOY
```

Return a specific block reason.

---

# 48. DISK SAFETY

Before pulling:

- inspect free bytes;
- estimate artifact pull/staging requirement;
- estimate backup requirement;
- enforce a minimum free reserve.

Example policy:

```yaml
disk:
  minimum_free_bytes: 21474836480
  refuse_deploy_below_minimum: true
```

Safe automatic cleanup may include:

- expired staging bundles;
- completed temporary job files;
- explicitly unreferenced old images after retention policy.

Never automatically delete persistent volumes to make a deployment fit.

---

# 49. CONFIGURATION DRIFT

The agent should periodically report enough metadata to detect drift:

```text
expected manifest revision
actual manifest revision
expected artifact digest
actual artifact digest
service health
```

Do not silently overwrite all drift immediately in v1.

Panel should display:

```text
IN_SYNC
DRIFTED
UNKNOWN
```

An operator can reconcile deliberately.

Later, selected low-risk resources can use automatic reconciliation.

---

# 50. FAILURE SCENARIOS THAT MUST BE TESTED

Before Production is declared ready, intentionally test:

- [ ] invalid GitHub webhook signature;
- [ ] duplicate GitHub delivery;
- [ ] unknown repository webhook;
- [ ] workflow failure;
- [ ] candidate registered with wrong SHA;
- [ ] missing GHCR image;
- [ ] wrong image digest;
- [ ] agent offline before deployment;
- [ ] agent disconnects mid-deployment;
- [ ] repeated agent nonce;
- [ ] expired signed job;
- [ ] revoked agent;
- [ ] container fails to start;
- [ ] health returns 500;
- [ ] health reports wrong version;
- [ ] stabilization succeeds then one check fails;
- [ ] backup fails;
- [ ] backup exceeds limit;
- [ ] disk falls below reserve;
- [ ] concurrent deploy requested;
- [ ] workflow step retries;
- [ ] D1 temporary failure;
- [ ] R2 temporary failure;
- [ ] GitHub API temporary failure;
- [ ] Cloudflare deploy fails;
- [ ] Production approval missing;
- [ ] Production approval references old digest;
- [ ] migration succeeds but app fails;
- [ ] application rollback succeeds;
- [ ] rollback fails;
- [ ] data restore request requires separate approval;
- [ ] automation global pause active;
- [ ] app/control plane deployment breaks a route;
- [ ] old agent communicates with newer API;
- [ ] old launcher uses current auth API;
- [ ] recovery from control-plane outage.

Record results, not just "tested".

---

# 51. SECURITY NON-NEGOTIABLES

1. No generic web-controlled shell.
2. No public Docker socket.
3. No production SSH key in frontend/build artifacts.
4. No broad permanent GitHub PAT shared by nodes.
5. No broad Cloudflare account token on agents.
6. No plaintext secrets in Git.
7. No plaintext signing keys in D1.
8. Verify GitHub webhook signatures.
9. Verify agent signatures.
10. Verify control-plane job signatures.
11. Reject replay.
12. Deploy by digest/checksum.
13. Finite retries.
14. Bounded backup paths/sizes/runtime.
15. Production approval tied to target artifact.
16. Server-side RBAC.
17. Append-oriented auditing.
18. Environment-isolated credentials.
19. Game tokens cannot call infrastructure APIs.
20. Launcher public client uses PKCE.
21. Tauri update signatures remain enabled.
22. Third-party GitHub Actions pinned to full SHA for sensitive workflows.
23. Self-hosted runners treated as persistent high-trust machines, not disposable sandboxes.
24. Production data restore is not an automatic health-check reaction.
25. Security failures fail closed.

---

# 52. BUILD / DEPLOY MATRIX

Target behavior by repository:

| Repo | CI | Artifact | Development | Beta | Production |
|---|---|---|---|---|---|
| `experiences` | schema/tests | config bundle/checksum | auto or quick promote | promote exact revision | approved exact revision |
| `infra` control plane | tests/typecheck | Worker bundle | auto | promote | approved deploy |
| `infra` agent | Rust tests | signed/checksummed binary/release | test nodes | Beta nodes | approved node update |
| `infra` manifests | validation | manifest revision | auto/controlled | promote | approved |
| `docs` | build/link checks | static site | preview/dev | optional | publish |
| `authentication` | unit/integration/security | Worker bundle | auto | promote | approved |
| `panel` | lint/test/build/e2e | static bundle | auto | promote | approved |
| `sdk` | fmt/clippy/test | library/release assets | prerelease if needed | prerelease | SemVer release |
| `minecraft-integrations` | Gradle tests/builds | JARs/checksums | dev channel | prerelease | release |
| `launcher` | platform tests/build | installers/update assets | dev/nightly | prerelease | signed stable release |

"Auto" means after successful main candidate build and environment policy, not unverified code.

---

# 53. DEPLOYMENT PANEL USER EXPERIENCE

## Dashboard

Show:

- environment health;
- active release set;
- pending candidates;
- active deployment;
- offline/degraded agents;
- recent failures;
- DLQ/terminal workflow failures if Queues are used;
- automation pause state.

## Environment page

Show:

```text
Production
Current Release Set: rs_...
Last deploy: ...
Health: Healthy

Services
  auth   v... / digest...
  panel  ... / checksum...
  api    ...
```

## Candidate page

Show source-to-artifact evidence:

```text
Repository
Commit
Commit message
Build run
Tests
Artifact
Digest/checksum
Provenance
Created at
Eligible environments
```

## Deployment page

Timeline:

```text
Created
Approved
Preflight
Backup
Staging
Activate
Health
Stabilization
Success
```

or failure/rollback.

## Agent page

Show:

- identity;
- status;
- last seen;
- version;
- OS/arch;
- capabilities;
- environment assignment;
- resource summary;
- service health;
- key rotation/revocation controls;
- recent jobs.

## Backup page

Show:

- source;
- environment;
- timestamp;
- deployment association;
- size;
- checksum;
- encrypted status;
- verification status;
- expiration;
- restore action with explicit warning.

---

# 54. DO NOT BUILD THESE IN THE FIRST PASS

Unless already present and needed:

- Kubernetes;
- Nomad;
- a custom OCI registry;
- a custom source-control server;
- a generic browser terminal;
- a second metrics database;
- a custom log database;
- distributed consensus;
- multi-region active/active control plane;
- automatic Production database restoration;
- arbitrary host package management;
- auto-scaling game nodes;
- dozens of microservices;
- custom cryptographic primitives.

The target scale is very manageable with:

```text
GitHub
Cloudflare
Docker Compose
Velora Agent
Hermes
VPS
```

---

# 55. IMPLEMENTATION MILESTONES

The implementing LLM MUST execute these in order unless the audit demonstrates a dependency requires a small adjustment.

---

## MILESTONE 0 — Repository Audit and Contract Freeze

### Tasks

- [ ] Inspect all eight repositories.
- [ ] Record default branches.
- [ ] Record languages/frameworks/toolchains.
- [ ] Record build/test commands.
- [ ] Record existing GitHub workflows.
- [ ] Record existing Cloudflare config.
- [ ] Record existing Dockerfiles/Compose.
- [ ] Record existing APIs/schemas.
- [ ] Record current environment variables and secret names without secret values.
- [ ] Record existing deployment code.
- [ ] Record current auth implementation.
- [ ] Record launcher updater implementation.
- [ ] Record Minecraft integration targets.
- [ ] Record what currently runs successfully.
- [ ] Build/test each repo unchanged first.
- [ ] Create gap matrix.
- [ ] Identify any contradictions with this spec.
- [ ] Prefer minimal adaptation rather than rewrite.

### Exit criteria

Every repository can be categorized as:

```text
buildable / failing / incomplete / blocked
```

with evidence.

---

## MILESTONE 1 — Cross-Repo CI and Supply-Chain Baseline

### Tasks

- [ ] Standardize PR CI.
- [ ] Set minimal `GITHUB_TOKEN` permissions.
- [ ] Pin security-sensitive third-party actions to commit SHA.
- [ ] Add caching only after correctness is proven.
- [ ] Add exact artifact naming.
- [ ] Add checksums.
- [ ] Add OCI metadata to containers.
- [ ] Add GHCR publishing for containerized services.
- [ ] Add GitHub Release flow for distributable binaries/JARs.
- [ ] Add Tauri updater signing pipeline prerequisites.
- [ ] Add artifact attestations/provenance where plan/repo eligibility allows and verification will be used.
- [ ] Configure short retention for transient artifacts.
- [ ] Protect self-hosted runners from untrusted code.
- [ ] Create reusable workflows where duplication is already significant.

### Exit criteria

Every repo has repeatable CI, and deployable outputs have exact identities.

---

## MILESTONE 2 — Cloudflare Resource and Migration Baseline

### Tasks

- [ ] Confirm Cloudflare zones/domains.
- [ ] Define Development/Beta/Production resource naming.
- [ ] Create/verify `velora_harness` D1.
- [ ] Create/verify `velora_identity` D1.
- [ ] Commit migrations.
- [ ] Create/verify R2 buckets.
- [ ] Create/verify Workflows bindings.
- [ ] Configure Access for admin/deployment UI.
- [ ] Configure static frontend deployment path.
- [ ] If starting new frontend deployment, use Workers Static Assets.
- [ ] Preserve existing Pages setup if healthy and migration has no immediate value.
- [ ] Define scoped API tokens for CI.
- [ ] Do not store those tokens in repository files.

### Exit criteria

A minimal test Worker can read/write expected Development bindings, and migrations can be applied predictably.

---

## MILESTONE 3 — Control Plane API Skeleton

### Tasks

- [ ] Implement API routing/versioning.
- [ ] Add request IDs.
- [ ] Add structured errors.
- [ ] Implement D1 repository/environment/service models.
- [ ] Implement artifact/candidate model.
- [ ] Implement GitHub webhook verification/dedupe.
- [ ] Implement CI candidate registration.
- [ ] Prefer GitHub OIDC for registration auth.
- [ ] Implement audit helper.
- [ ] Implement RBAC middleware.
- [ ] Implement automation flags.
- [ ] Publish OpenAPI contract.
- [ ] Add unit/integration tests.

### Exit criteria

A successful CI run can register one immutable candidate, and Panel/API can read it.

No deployment occurs yet.

---

## MILESTONE 4 — Agent Enrollment + Heartbeat

### Tasks

- [ ] Build agent CLI/daemon skeleton.
- [ ] Generate Ed25519 identity locally.
- [ ] Create one-time enrollment tokens.
- [ ] Store enrollment token hash only.
- [ ] Enforce expiration/use-once.
- [ ] Register agent public key.
- [ ] Implement signed request format.
- [ ] Implement nonce replay protection.
- [ ] Implement heartbeat.
- [ ] Implement capabilities.
- [ ] Implement revoke.
- [ ] Implement local config validation.
- [ ] Create systemd unit/install flow.
- [ ] Add `velora-agent doctor`.
- [ ] Test key/replay/revoke failures.

### Exit criteria

Hermes Development agent appears in Panel as healthy without any inbound port.

---

## MILESTONE 5 — First Complete Development Deployment

Use **one harmless service**.

### Tasks

- [ ] Create service contract.
- [ ] Create Development Compose topology.
- [ ] Register exact GHCR digest.
- [ ] Implement deployment lock.
- [ ] Start Cloudflare deployment Workflow.
- [ ] Create signed agent job.
- [ ] Agent pulls exact image digest.
- [ ] Agent activates service.
- [ ] Agent health-checks expected version.
- [ ] Agent reports events/result.
- [ ] Workflow records final state.
- [ ] Panel renders deployment timeline.
- [ ] Duplicate request does not create duplicate deployment.
- [ ] Failure produces clear result.

### Exit criteria

This exact chain works:

```text
Commit
-> CI
-> immutable artifact
-> Velora candidate
-> Development deployment
-> Agent
-> health
-> Panel success
```

Do not onboard all repos before this works.

---

## MILESTONE 6 — Rollback + Backup

### Tasks

- [ ] Record previous known-good manifest/digest.
- [ ] Implement backup profiles.
- [ ] Add R2 upload.
- [ ] Add encryption.
- [ ] Add checksum.
- [ ] Add allowed roots.
- [ ] Add size/runtime limits.
- [ ] Add retention metadata.
- [ ] Add app rollback.
- [ ] Add manual restore request/approval.
- [ ] Test bad image.
- [ ] Test failed health.
- [ ] Test failed rollback.
- [ ] Test backup failure.
- [ ] Test data restore in non-production.

### Exit criteria

A bad Development deployment automatically returns the app to the previous healthy version, while data restoration remains explicit.

---

## MILESTONE 7 — Deployment Panel Completion

### Tasks

- [ ] Dashboard.
- [ ] Environment pages.
- [ ] Service pages.
- [ ] Candidate pages.
- [ ] Release Sets.
- [ ] Deployment timeline.
- [ ] Agent pages.
- [ ] Backup pages.
- [ ] Audit log.
- [ ] Emergency automation controls.
- [ ] Production approval UX.
- [ ] Error/retry UX.
- [ ] Loading/empty/offline states.
- [ ] Cloudflare Access configuration.
- [ ] Velora RBAC enforcement.
- [ ] E2E tests for critical flows.

### Exit criteria

An authorized operator can understand exactly what is running, what will change, and why a deployment is blocked.

---

## MILESTONE 8 — Beta Promotion

### Tasks

- [ ] Isolate Beta secrets/state.
- [ ] Enroll Beta agent/node.
- [ ] Promote same immutable Development-tested artifact.
- [ ] Create Release Set.
- [ ] Add Beta stabilization policy.
- [ ] Add Beta backup policy.
- [ ] Run integration/regression suite.
- [ ] Record Beta evidence on candidate.
- [ ] Ensure no Production credentials used.

### Exit criteria

Beta behaves like a rehearsal for Production and runs exact promoted artifacts.

---

## MILESTONE 9 — Production Guardrails

### Tasks

- [ ] Enroll Production agent with Production-only policy.
- [ ] Separate Production credentials.
- [ ] Enforce production permission.
- [ ] Enforce approval tied to digest.
- [ ] Enforce immutable artifact.
- [ ] Enforce mandatory backup where needed.
- [ ] Enforce disk preflight.
- [ ] Enforce supported agent version.
- [ ] Enforce Beta evidence policy.
- [ ] Enforce one deployment lock.
- [ ] Enforce health stabilization.
- [ ] Test rollback.
- [ ] Test emergency pause.
- [ ] Verify all audit events.
- [ ] Create Production runbook.

### Exit criteria

No code path can deploy a new Production target merely because someone pushed to GitHub.

---

## MILESTONE 10 — Launcher Distribution

### Tasks

- [ ] Verify Tauri v2 updater setup.
- [ ] Establish updater signing key custody.
- [ ] Build Windows/Linux/macOS.
- [ ] Configure platform signing/notarization as available.
- [ ] Publish GitHub Release.
- [ ] Generate updater metadata.
- [ ] Mirror to R2 if desired.
- [ ] Verify mirrored hash/signature.
- [ ] Test update from prior stable build.
- [ ] Add Beta/stable channels.
- [ ] Register release in control plane.
- [ ] Document recovery/key policy.

### Exit criteria

A stable installed launcher can safely discover, verify, download, and install a signed newer release.

---

## MILESTONE 11 — Experiences + Minecraft Artifact Pipeline

### Tasks

- [ ] Finalize experience schema.
- [ ] Validate configs in CI.
- [ ] Build integration artifacts.
- [ ] Generate checksums.
- [ ] Publish release artifacts.
- [ ] Reference exact versions/checksums from experience revisions.
- [ ] Implement node-side verified artifact retrieval.
- [ ] Add staged update/rollback.
- [ ] Test incompatible/missing artifact failure.
- [ ] Document loader/server-version compatibility.

### Exit criteria

An experience deployment cannot accidentally install an unverified "latest" plugin/mod.

---

## MILESTONE 12 — Identity Hardening

### Tasks

- [ ] Audit existing auth routes.
- [ ] Implement/verify OIDC Discovery.
- [ ] Implement/verify JWKS.
- [ ] Authorization Code.
- [ ] PKCE S256 for public clients.
- [ ] Refresh rotation/revocation.
- [ ] Passkeys/WebAuthn.
- [ ] Password hashing review if passwords supported.
- [ ] Turnstile/rate controls.
- [ ] Session security.
- [ ] RBAC.
- [ ] Separate game token.
- [ ] Security event audit.
- [ ] Launcher OIDC integration.
- [ ] Verify redirect URI policy.
- [ ] Negative security tests.

### Exit criteria

Website/Panel/Launcher/Game auth use the same identity authority without sharing interchangeable credentials.

---

## MILESTONE 13 — Reliability Drills and Go-Live Readiness

### Tasks

- [ ] Execute every failure scenario in Section 50 that applies.
- [ ] Restore from a real backup in a non-production drill.
- [ ] Revoke an agent and verify it cannot operate.
- [ ] Rotate an agent key.
- [ ] Test GitHub outage behavior.
- [ ] Test Cloudflare API/deploy failure.
- [ ] Test Control Plane temporary outage.
- [ ] Test Production service restart without Control Plane.
- [ ] Verify old launcher/API compatibility policy.
- [ ] Verify monitoring/alert paths.
- [ ] Verify R2 lifecycle rules.
- [ ] Verify audit log completeness.
- [ ] Verify no secrets in repo history/current build output.
- [ ] Review dependencies.
- [ ] Review GitHub App permissions.
- [ ] Review Cloudflare API token scopes.
- [ ] Review runner permissions.
- [ ] Review agent host permissions.
- [ ] Review Production recovery runbook.

### Exit criteria

Velora is not "ready" because the happy path works. It is ready when known failure paths have verified, documented outcomes.

---

# 56. DEFINITION OF DONE FOR EACH TASK

A checkbox is not `DONE` until:

1. implementation exists;
2. code builds;
3. relevant automated tests pass;
4. negative/error paths are tested where applicable;
5. configuration is documented;
6. no real secrets were added;
7. deployment/release behavior is verified;
8. regression tests pass;
9. change is committed;
10. evidence is recorded in the implementation status.

"Looks correct" is not verification.

---

# 57. REQUIRED TEST LAYERS

Use the layers appropriate to each repo.

```text
Unit
Contract/schema
Integration
End-to-end
Deployment smoke
Upgrade
Rollback
Security-negative
```

Examples:

### Control Plane

- HMAC webhook test;
- duplicate delivery test;
- permission denial test;
- idempotency test;
- lock contention test;
- workflow retry test.

### Agent

- signature verification;
- nonce replay;
- path escape rejection;
- digest mismatch;
- disk preflight;
- failed health;
- rollback.

### Panel

- unauthenticated route;
- unauthorized action;
- create deployment;
- approval;
- failed deployment rendering;
- emergency pause.

### Auth

- PKCE success/failure;
- redirect URI mismatch;
- expired authorization code;
- refresh rotation;
- replay;
- logout/revocation;
- passkey registration/authentication.

### Launcher

- updater signature failure;
- wrong manifest;
- network interruption;
- previous-version upgrade.

---

# 58. VERSIONING

Use Semantic Versioning where product/package semantics fit.

Examples:

```text
launcher v1.4.2
agent v0.3.0
sdk v0.8.0
minecraft-integrations v0.5.0
```

Internal web services may deploy more frequently but must still expose:

```text
Git SHA
build ID
artifact checksum/digest
```

Protocol schemas have independent schema versions.

Do not use application version numbers as cryptographic artifact identity.

---

# 59. AGENT UPDATE POLICY

The agent updates the mechanism that deploys other software, so it needs extra care.

Do not blindly self-update all Production agents from `latest`.

Recommended:

```text
Development agent
 -> Beta agent
 -> Production agent
```

Agent update package includes:

- version;
- SHA-256;
- provenance/signature if implemented;
- compatibility range;
- release notes.

Keep one known-good previous agent binary for recovery.

The service manager should restart the agent and verify heartbeat after update.

If the agent cannot update itself safely initially, use a human-controlled package update process until the self-update path is proven.

---

# 60. CONTROL PLANE SELF-DEPLOYMENT

The Control Plane must not be the only mechanism capable of repairing itself.

For Cloudflare deployments:

- keep previous Worker versions available;
- keep deployment definitions in Git;
- keep migrations backward-compatible when possible;
- test in Development/Beta;
- use Cloudflare/GitHub deployment tooling independently enough to roll back a broken control-plane release.

Avoid:

```text
new control plane breaks
-> only new control plane can roll itself back
-> deadlock
```

Maintain a documented break-glass rollback procedure.

---

# 61. R2 OBJECT LAYOUT

Example:

```text
velora-assets/
├── skins/
├── capes/
├── branding/
├── modpacks/
├── experience-assets/
└── generated/

velora-backups/
├── development/
│   └── <service>/
├── beta/
│   └── <service>/
└── production/
    └── <service>/

velora-logs/
├── deployments/
└── exports/

velora-releases/
├── launcher/
├── integrations/
└── config-bundles/
```

Do not duplicate GitHub Release assets into R2 unless the mirror solves a real product requirement such as private access, stable domain, update CDN, or controlled download semantics.

---

# 62. DOMAIN MODEL

Exact domain names can match the actual Velora domain.

Recommended logical separation:

```text
velora.<tld>          product/site
app.velora.<tld>      user application if separate
admin.velora.<tld>    admin
deploy.velora.<tld>   deployment panel if separate
api.velora.<tld>      core/control API
auth.velora.<tld>     identity/OIDC/game auth
docs.velora.<tld>     docs
updates.velora.<tld>  launcher/update assets
cdn.velora.<tld>      public assets
```

Do not create domains merely because listed here. Reuse existing naming if established.

---

# 63. PERFORMANCE RULES

## Control Plane

- index D1 queries used in dashboard/deployment lists;
- paginate history;
- avoid N+1 D1 queries;
- avoid putting huge logs in D1;
- cache only safe read-mostly data;
- never cache authorization decisions longer than their security model allows.

## Panel

- code-split heavy pages;
- stream/poll deployment state efficiently;
- do not poll every second;
- use optimistic UI only where rollback of UI state is safe;
- keep deployment state server-authoritative.

## Agent

- bounded concurrency;
- streaming downloads where practical;
- no repeated pulls of existing digest;
- no unbounded in-memory logs;
- backpressure on log/event uploads;
- atomic manifest writes.

---

# 64. DATA CONSISTENCY RULES

- D1 is authoritative for control-plane metadata.
- Agent local state is authoritative for what it actually executed until reconciled.
- Reconciliation compares desired vs actual.
- Git is authoritative for source/declarative definitions, not live deployment success.
- GHCR/Release/R2 artifact checksum is authoritative for artifact bytes.
- Panel state is never authoritative by itself.

When two sources disagree, surface drift instead of silently guessing.

---

# 65. TIME AND IDENTIFIERS

Recommended:

- IDs: UUIDv4/UUIDv7 or similarly collision-resistant opaque IDs;
- API timestamps: RFC 3339 UTC;
- D1 sortable timestamps: integer Unix milliseconds or a consistent ISO UTC representation;
- never mix local timezone semantics into deployment state.

Human UI may render local time.

Agent request signatures should use UTC epoch timestamps.

---

# 66. ERROR RECOVERY PHILOSOPHY

Failure should be:

```text
bounded
observable
retryable when safe
terminal when unsafe
recoverable
audited
```

Retry only transient operations.

Do not automatically retry:

- invalid signature;
- permission denial;
- digest mismatch;
- invalid manifest;
- insufficient disk that cannot change;
- destructive operation lacking approval.

Retry may make sense for:

- temporary R2 failure;
- temporary GitHub API 5xx;
- temporary network error;
- short-lived D1/Cloudflare dependency errors.

---

# 67. MIGRATION SAFETY

Any deployment with a database migration declares:

```yaml
migration:
  required: true
  compatibility: backward_compatible | breaking
  data_restore_required_for_rollback: false | true
```

Production policy:

- `backward_compatible`: can proceed if other gates pass.
- `breaking`: require stronger/manual review and explicit rollback plan.

Prefer not to deploy destructive breaking migrations in the same irreversible step as application rollout.

---

# 68. BACKUP RESTORE RUNBOOK

A restore flow must:

1. select exact backup;
2. verify checksum;
3. verify encryption/decryption;
4. verify service/environment match;
5. stop or quiesce writes if required;
6. snapshot current state first when safe;
7. restore;
8. verify application-aware consistency;
9. start service;
10. health-check;
11. audit operator and result.

A restore is not simply "download tar and overwrite directory."

---

# 69. REPOSITORY README REQUIREMENTS

Each repo should document:

```text
Purpose
Supported environments
Prerequisites
Local development
Build
Test
Release
Deployment target
Required configuration names
Secret names (not values)
Health check
Rollback/recovery notes
Ownership of generated artifacts
```

The LLM should update stale READMEs after implementation, not before reality exists.

---

# 70. CONFIGURATION SCHEMAS

Machine-read configuration should have a schema version.

Example:

```yaml
schema: 1
```

Rules:

- unknown required fields fail;
- unsupported future schema fails clearly;
- migrations are deliberate;
- schema documents are testable.

For JSON/YAML user-edited config, publish JSON Schema when practical.

---

# 71. DEPENDENCY MANAGEMENT

- retain lockfiles;
- automate dependency PRs if desired;
- do not auto-merge breaking major updates without tests;
- security updates receive priority;
- release workflows use reproducible dependency resolution;
- record required Java/Node/Rust toolchain versions.

Avoid "use newest" in CI.

Pin major toolchains intentionally.

---

# 72. SUGGESTED SECURITY HEADERS / WEB BASICS

For Panel/Auth web properties where applicable:

- HTTPS only;
- HSTS after domains are stable;
- CSP appropriate to actual frontend;
- `X-Content-Type-Options: nosniff`;
- safe frame policy / `frame-ancestors`;
- referrer policy;
- secure cookie flags;
- narrow CORS rather than `*` for credentialed APIs.

Do not copy a CSP blindly; generate it from actual application dependencies.

---

# 73. CORS

API rules:

- explicit allowed origins;
- no wildcard + credentials;
- preflight handled correctly;
- environment origins separated;
- auth endpoints follow OAuth/OIDC standards;
- admin API does not accept arbitrary web origins.

Native launcher clients do not need browser CORS semantics in the same way a browser SPA does, so do not weaken browser CORS to accommodate a native client unnecessarily.

---

# 74. GITHUB PAGES POSITION

GitHub Pages is optional.

Use for:

- purely static developer docs;
- generated SDK docs;
- API reference mirror;
- non-transactional project pages.

Do not use for:

- account portal;
- Deployment Panel;
- auth;
- SaaS transaction flow;
- server-side API.

Primary product-facing Cloudflare static deployments should use Workers Static Assets for new work, or retain an already-working Pages deployment until intentionally migrated.

---

# 75. COST CONTROL

At Velora's current scale, architecture complexity is more dangerous than baseline infrastructure cost.

Still enforce:

- finite Workflows/Queue retries;
- R2 lifecycle policies;
- bounded logs;
- bounded backup retention;
- sensible heartbeat frequency;
- no per-second dashboard polling;
- D1 pagination/indexing;
- Cloudflare billing alerts;
- Actions artifact retention;
- self-hosted runners for heavy recurring workloads when economical.

Budget alerts are alerts, not hard application-level limits. Implement application-level limits too.

---

# 76. GO-LIVE CHECKLIST

Production is ready only when all applicable items are true:

### Source/build

- [ ] All eight repos audited.
- [ ] CI green.
- [ ] Lockfiles present.
- [ ] Release workflows reproducible.
- [ ] Third-party actions hardened.
- [ ] Self-hosted runners appropriately isolated.

### Artifacts

- [ ] Containers deploy by digest.
- [ ] Binaries/JARs have checksums.
- [ ] Launcher updates signed.
- [ ] Release provenance/attestations enabled where useful.
- [ ] Artifact retention defined.

### Cloudflare

- [ ] D1 migrations committed.
- [ ] R2 lifecycle rules.
- [ ] Workflows tested.
- [ ] Access protects admin/deploy UI.
- [ ] Worker secrets configured.
- [ ] Scoped CI Cloudflare token.
- [ ] Rollback/break-glass procedure.

### Control plane

- [ ] GitHub signatures verified.
- [ ] Webhook dedupe.
- [ ] OIDC or scoped CI auth.
- [ ] RBAC.
- [ ] audit log.
- [ ] idempotency.
- [ ] locks.
- [ ] explicit deployment states.
- [ ] emergency pause.
- [ ] Production approval tied to digest.

### Agents

- [ ] one-use enrollment.
- [ ] Ed25519 identity.
- [ ] signed requests.
- [ ] replay protection.
- [ ] signed jobs.
- [ ] capability checks.
- [ ] no arbitrary shell.
- [ ] disk preflight.
- [ ] health checks.
- [ ] rollback.
- [ ] revoke.
- [ ] supported-version policy.

### Backups

- [ ] application-aware backup.
- [ ] encryption.
- [ ] checksum.
- [ ] R2 upload.
- [ ] retention.
- [ ] restore drill.
- [ ] manual Production restore.

### Identity

- [ ] OIDC Discovery.
- [ ] JWKS.
- [ ] Authorization Code.
- [ ] PKCE S256.
- [ ] token rotation/revocation.
- [ ] session security.
- [ ] passkey/password policy.
- [ ] rate limiting/abuse controls.
- [ ] game token isolation.

### Launcher

- [ ] update signing private key safely backed up.
- [ ] public updater key embedded.
- [ ] stable update path tested.
- [ ] platform build matrix.
- [ ] release checksums.
- [ ] previous-version update test.

### Operations

- [ ] failure drills executed.
- [ ] monitoring useful.
- [ ] logs redact secrets.
- [ ] runbooks complete.
- [ ] operator can determine current Production artifact exactly.
- [ ] operator can roll application back without rebuilding.
- [ ] operator knows how to restore data separately.

---

# 77. FIRST VERTICAL SLICE — DO THIS BEFORE EVERYTHING ELSE

The first implementation target should be exactly:

```text
One repo commit
    |
    v
GitHub Actions test/build
    |
    v
One immutable GHCR artifact
    |
    v
Candidate registered in Control Plane
    |
    v
Development deployment created
    |
    v
Cloudflare Workflow
    |
    v
Hermes Velora Agent
    |
    v
Docker Compose deploy
    |
    v
Health check with expected SHA/version
    |
    v
Panel says SUCCEEDED
```

Then intentionally deploy a broken image and prove:

```text
FAILED
 -> ROLLING_BACK
 -> previous digest
 -> health
 -> ROLLED_BACK
```

Only after this exact slice is reliable should all remaining services be onboarded.

---

# 78. FINAL TARGET STATE

When this specification is complete, the normal lifecycle should look like:

```text
Developer
   |
   v
Pull Request
   |
   v
Tests / Review
   |
   v
main
   |
   v
Build once
   |
   +-----------------------------+
   |                             |
   v                             v
GHCR / GitHub Release           Candidate Metadata
immutable artifact                 |
   |                               v
   |                         Velora Control Plane
   |                               |
   |                         auto Development
   |                               |
   |                          promote to Beta
   |                               |
   |                      explicit Production approval
   |                               |
   +-------------------------------+
                                   |
                     +-------------+--------------+
                     |                            |
                     v                            v
              Cloudflare deploy              Velora Agent
              exact bundle/SHA              exact digest
                     |                            |
                     v                            v
               smoke checks                  backup/deploy
                     |                         health/rollback
                     +-------------+--------------+
                                   |
                                   v
                           D1 authoritative state
                                   |
                                   v
                          Deployment Panel + Audit
```

And the operational rule remains:

> **A release is not "whatever code is newest." A release is a known source revision mapped to a known immutable artifact, promoted through known environments, with known verification evidence.**

---

# 79. IMPLEMENTATION REPORT FORMAT FOR THE LLM

At the end of every milestone, respond with:

```markdown
## Milestone N — <name>

### Completed
- ...

### Verification
- Command: `...`
  - Result: PASS
- Test: ...
  - Result: PASS

### Files changed
- `...`
- `...`

### Architecture decisions
- ...

### Known issues / deferred work
- ...

### Security notes
- ...

### Next milestone
- ...
```

Never report a test as passing if it was not run.

If a command cannot be run, say exactly why.

---

# 80. OFFICIAL REFERENCES

These references are authoritative starting points. If platform behavior has changed after this document's review date, use the newest official documentation and update the implementation accordingly.

## Cloudflare

- Workers Static Assets:  
  https://developers.cloudflare.com/workers/static-assets/
- Workers best practices, including current recommendation to use Workers Static Assets for new projects:  
  https://developers.cloudflare.com/workers/best-practices/workers-best-practices/
- Cloudflare Pages:  
  https://developers.cloudflare.com/pages/
- Cloudflare Workflows:  
  https://developers.cloudflare.com/workflows/
- Workflows guide:  
  https://developers.cloudflare.com/workflows/get-started/guide/
- Workflows rules / idempotency:  
  https://developers.cloudflare.com/workflows/build/rules-of-workflows/
- D1 migrations:  
  https://developers.cloudflare.com/d1/reference/migrations/
- D1 database API:  
  https://developers.cloudflare.com/d1/worker-api/d1-database/
- D1 read replication / Sessions API:  
  https://developers.cloudflare.com/d1/best-practices/read-replication/
- Queues configuration:  
  https://developers.cloudflare.com/queues/configuration/configure-queues/
- Dead Letter Queues:  
  https://developers.cloudflare.com/queues/configuration/dead-letter-queues/
- Queues pull consumers:  
  https://developers.cloudflare.com/queues/configuration/pull-consumers/
- Workers Web Crypto:  
  https://developers.cloudflare.com/workers/runtime-apis/web-crypto/
- Cloudflare Access service tokens:  
  https://developers.cloudflare.com/cloudflare-one/access-controls/service-credentials/service-tokens/

## GitHub

- GitHub Actions deployments/environments:  
  https://docs.github.com/en/actions/reference/workflows-and-actions/deployments-and-environments
- Secure use of GitHub Actions:  
  https://docs.github.com/en/actions/reference/security/secure-use
- Reusable workflows:  
  https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows
- GitHub Container Registry:  
  https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry
- GitHub Releases:  
  https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases
- Actions artifact/log retention:  
  https://docs.github.com/en/organizations/managing-organization-settings/configuring-the-retention-period-for-github-actions-artifacts-and-logs-in-your-organization
- Artifact attestations:  
  https://docs.github.com/en/actions/concepts/security/artifact-attestations
- OpenID Connect in GitHub Actions:  
  https://docs.github.com/en/actions/concepts/security/openid-connect
- GitHub Pages limits/policy:  
  https://docs.github.com/en/pages/getting-started-with-github-pages/github-pages-limits

## Tauri

- Tauri v2 updater:  
  https://v2.tauri.app/plugin/updater/
- Tauri GitHub pipeline:  
  https://v2.tauri.app/distribute/pipelines/github/
- macOS signing:  
  https://v2.tauri.app/distribute/sign/macos/
- Windows signing:  
  https://v2.tauri.app/distribute/sign/windows/

## OAuth / OpenID Connect

- OAuth 2.0 Security Best Current Practice, RFC 9700:  
  https://www.rfc-editor.org/rfc/rfc9700.html
- OpenID Connect Discovery 1.0:  
  https://openid.net/specs/openid-connect-discovery-1_0.html
- OWASP Password Storage Cheat Sheet:  
  https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html

---

# 81. FINAL INSTRUCTION TO THE IMPLEMENTING LLM

Start with **Milestone 0**.

Do not respond by redesigning Velora from scratch.

Inspect the actual repositories, compare the existing implementation to this contract, preserve everything that is already correct, and create the smallest set of changes necessary to reach the target state.

The highest-priority proof is not a beautiful dashboard.

The highest-priority proof is:

```text
source
 -> verified build
 -> immutable artifact
 -> authenticated candidate
 -> durable deployment orchestration
 -> signed agent execution
 -> health verification
 -> visible result
 -> reliable rollback
```

Once that is working on Development, expand it carefully to the rest of Velora.
