# Development probe orchestration

This is the narrow host adapter for the Cloudflare-first deployment control plane.
It does not move compatible product services onto a VPS. Workers, D1, R2 and Access
remain the preferred product infrastructure; the probe has no product or game data.

Migration `0004` preserves existing enrollment and heartbeat-only capabilities.
A separate `probe_targets` row must explicitly approve one active Development
agent for `deployment-probe`. No target, operator role or execution flag is seeded.
Defaults remain disabled. Do not give the heartbeat account Docker membership.

## Execution and recovery

The protected operator POST `/api/v1/deployments` accepts only schema, candidate ID
and agent ID, with a same-origin action header and UUID idempotency key. The key
is the deployment ID. `deployment.create` permission is distinct from Access
authentication and heartbeat/enrollment permissions. The target must be online,
Linux x86_64, active and explicitly approved. The candidate must be an enabled
Infra-owned probe service's immutable OCI image; tags and other registries fail.

D1 atomically reserves the deployment and per-service lock and records an audit.
Retries with the same actor/input repair a missed Workflow creation. Conflicting
input fails. The Workflow signs one five-minute job and persists it before delivery;
step retries reuse that job. Its parameters contain only the deployment ID.
Polling returns the same bytes and does not renew their expiry. Agent requests use
Ed25519 signatures, bounded bodies, timestamps and durable nonce replay checks.

The native executor receipt binds the SHA-256 of the exact base64url job payload.
Only the delivered job's approved agent can report it. Identical signed retries
deduplicate; conflicting receipts fail. Events are a fixed bounded vocabulary,
not stdout, arbitrary messages, commands or secrets. Reads require deployment.read
and expose records for a future Panel timeline. The API does not itself prove
container health; the trusted executor checks expected source identity.

SUCCEEDED, FAILED (stopped first deployment) and ROLLED_BACK release the lock.
BLOCKED and ROLLBACK_FAILED retain it. A missing result never implies stopped
execution. A late authenticated terminal receipt can reconcile a timeout. An
already recorded blocked/failed-rollback receipt needs explicit operator
reconciliation; there is no unsafe reset-lock endpoint or automatic data restore.
Revoked agents/targets cannot poll or report. Pausing new deployment admission
still permits results from an active approved target.

## Activation requirements

Use an ignored operator configuration. Bind the added
`velora-probe-deployment-development` Workflow, apply only the expand-only harness
migration to the existing harness store, and deploy the reviewed compiled artifact.
Grant explicit Development deployment.read/create permissions, approve the target
and register its exact probe service/candidate. Supply the operator-owned Ed25519
private JWK as the Worker secret `PROBE_SIGNING_PRIVATE_JWK`; its public `x` value
must match `PROBE_SIGNING_PUBLIC_KEY`. Never generate real signing keys in tests,
write them in Git/D1/job payloads, or copy private keys to hosts. Local tests create
ephemeral synthetic keys and exercise the actual Workflow and D1 runtime.

Enable `PROBE_DEPLOYMENTS_ENABLED=1` and the separate Development deployment flag
only for the controlled execution proof. Native transport commands are opt-in:

```sh
velora-agent probe-poll --config /etc/velora-agent/config.toml --policy /etc/velora-probe/public-policy.json --job /var/lib/velora-agent/probe/job.json
velora-agent probe-report --config /etc/velora-agent/config.toml --receipt /var/lib/velora-agent/probe/receipt.json
```

Polling verifies the signed target before staging a mode-0600 file in an owned,
private, canonical directory. It never overwrites a different staged job. Reporting
uses a fresh signed request and checks the accepted job/status. Neither command
invokes Docker. The separately reviewed privileged host executor must pin its
own root-owned policy and expose only this typed probe action. Transport scheduling,
privileged installation, credential custody, real image pull, live rollback and
Panel integration remain required before the complete Milestone 5 exit is claimed.

## Minimal-cost policy

No Containers, Queues, additional databases, extra VPS, paid plan or new billing
subscription is introduced. This uses the existing D1 store and one bounded
Workflow per admitted deployment. Admission is capped at ten new deployments per
rolling 24 hours and 1,000 total retained records; retries do not consume admission
budget. Each Workflow performs at most fifteen 30-second waits, bounded step
retries, and one timeout reconciliation. Workflow runtime state expires after one
day; D1 retains the bounded deployment/audit records. Poll no faster than once per minute and
schedule it only during an approved deployment window. The Panel should refresh
on demand. There are no periodic scans, per-poll R2 writes or uploaded raw logs.

Infra CI runs once per PR revision and once on main after merge, rather than twice
for branch push plus PR. Obsolete PR runs cancel; main runs remain protected.
Manual checks remain available for a branch before opening a PR.

Current [Workflows pricing](https://developers.cloudflare.com/workflows/reference/pricing/)
includes charges/quotas for steps and retained state as well as invocations/CPU;
sleeping avoids CPU use but is not a blanket promise of zero cost. The Free plan's
limits are shared with other account workloads. Existing R2 usage may still incur
usage charges. Check the account's actual usage before expanding deployment
volume; budget caps here do not grant paid upgrades or promise a hard account-wide
spending ceiling. These limits are for the probe, not a certified product budget.
