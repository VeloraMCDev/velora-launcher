# Milestone 3 — Development candidate registration

## Completed

The Development Worker has versioned routing, generated request IDs, sanitized
errors, repository/service/environment models, immutable artifacts/candidates,
RBAC, audit events, registration flags and an OpenAPI contract. Expand-only
migrations preserve the earlier resource checks and separate deployment roles
from game/player identity. All eight GitHub repository identities are recorded;
only Infra's exact reviewed Development workflow/service policy is enabled.

Human routes require signed Access admission plus an explicit active principal
and environment-scoped permission. CI registration has a separate hostname and
validates signed audience-bound GitHub OIDC with immutable repository/owner IDs,
source/workflow SHA, exact path/ref/subject and run identity. No reusable CI token
is needed. Candidate registration never starts a deployment.

## Verification

- Infra source `fcc2f293f626d4e9c287ecad51e659511726c3de`, push run
  [37688370959](https://github.com/VeloraMCDev/infra/actions/runs/37688370959),
  passed Worker, contracts, generated types, TypeScript, actual non-root disposable
  container and genuine OIDC registration. Both push and PR public-source preflights
  and PR deployment checks passed. The registration job was rerun separately using
  the existing successful build artifact; no release was rebuilt for registration.
- All 20 local tests passed. Actual D1 tests prove concurrent registration creates
  one artifact, one candidate and one audit event; conflicting pause/resume retries
  produce one accepted action and one conflict. Signed webhook retries deduplicate
  and changed content under an existing delivery ID conflicts.
- The compiled CI module's size and SHA-256 matched its manifest before deployment.
  Both remote `0002` migrations applied; the two existing successful binding-check
  records survived. All flags initially remained zero.
- Protected browser reads verified the live full source SHA, registry pagination,
  audit and Development flags. Production reads returned permission denial.
  Bootstrap was disabled after binding the exact verified Access principal to
  Development read/registration-management permissions. No Beta/Production role
  was created. Registration was resumed through the audited API; deployment stayed zero.
- The CI hostname serves valid HTTPS. Unsigned registration is rejected, and human
  API paths are rejected on that hostname. The existing operator route remains
  protected by Access.
- Genuine CI registration created candidate
  `rc_e043d427d8dbea7641b3ad30e906ca284dd6ebcfd215e0611b5cf4adf1cc97d5`
  for the exact uploaded artifact checksum
  `69f5c5aee5de35fd626c0dc453b71b719a4da17837476f63cc90ac9232a0c940`.
  D1 confirms one candidate and one registration audit event. A genuine CI retry
  returned the exact same candidate/checksum with `created: false`, using the
  original build artifact. Protected browser GET returned the exact registered
  metadata and artifact record with HTTP 200; Production read and bootstrap
  returned HTTP 403. Registration was paused again through the audited API and
  the CI opt-in was disabled. Milestone 3 registration/read exit criteria passed.

## Files changed

Infra owns `control-plane/src/api.ts`, identity/signature/RBAC helpers, migrations,
the canonical [OpenAPI contract](https://github.com/VeloraMCDev/infra/blob/fcc2f293f626d4e9c287ecad51e659511726c3de/control-plane/openapi.yaml),
shared artifact validation and generated standalone schema code, CI handoff,
workflow guard and actual-D1 API tests. Operator IDs, hostnames, principal mapping
and bootstrap SQL stay in ignored local configuration. Docs owns this report.

## Architecture decisions

This initial API serves Development only. Registration is capped at 1,000 candidates,
JSON bodies at 16 KiB and reads at 50 records with typed cursors. Candidate/audit
mutations share a D1 transaction; retries return the original immutable metadata.
Same-origin headers, explicit permissions and UUID idempotency keys guard operator
flag changes. The reviewed CI exception grants only contents-read and OIDC-token
permission after all build checks, behind exact branch and explicit opt-in gates.

Workers/Access remain on Free plans. No additional compute subscription, paid
certificate manager or hosted container service was enabled. The extra exact
hostname uses the Worker's managed certificate. Hermes key SSH authentication,
x86_64/systemd/Docker/Compose and approximately 54 GiB free space are verified;
sudo still requires the local operator password.

## Known issues / deferred work

GitHub Actions ZIP artifacts expire after 14 days. They provide the initial
registration proof; durable immutable custody is required before deploying
application releases. The disposable probe image is not a release artifact.
GitHub App installation/webhook secret, durable retention/recovery and complete
Panel client are pending. Webhook ingress remains disabled; implementation/tests
cover repository-scoped informational events, not App-level installation pings.
No agent, deploy/promotion/rollback API, Beta/Production resources or workload
cutover is delivered by this milestone.

## Security notes

Access admission alone never grants a role. CI names alone never grant trust;
immutable IDs and the exact workflow revision are checked. No JWT, cookie, secret,
raw webhook body or signing key is logged by the API. Agent permissions and player
credentials remain separate. All deployment flags remain zero.

## Next milestone

Implement Milestone 4:
native Rust agent enrollment, locally generated Ed25519 identity, hash-only
one-use enrollment tokens, signed heartbeats with nonce replay protection,
capabilities, revoke, doctor and a reviewable systemd installation flow.
