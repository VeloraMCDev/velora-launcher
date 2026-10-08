# Milestone 4 — Development agent enrollment — EXIT PASSED

## Completed

Infra implements a native Rust/Linux Ed25519 identity, signed enrollment and
heartbeat requests, bounded polling and a hardened systemd service. Development
enrollment tokens expire after ten minutes, are stored only as hashes and can bind
one public key once. Nonces reject replay; revocation blocks later signatures.
Only `REPORT_HEARTBEAT` is permitted. There is no job, shell, Docker operation,
inbound listener, broad account token or production permission.

The first-install bootstrap validates the checksum-pinned manifest and every
artifact file before creating a dedicated service account. It refuses existing
identities/configuration/destinations. The private key is generated on the host
and remains there. Enrollment uses hidden interactive terminal input and starts
the service only after enrollment and an accepted heartbeat.

Panel owns an independent operator screen for repositories, candidates, agent
health and paginated audit records, plus enrollment and revocation actions.
It uses same-origin API requests and stores no credentials in browser storage.
Permission loss clears records and invalidates concurrent reads; one-use codes
clear on expiry, hide, navigation or session loss. Refresh is manual.

Development now has expand-only migration `0003`, explicit agent operator
permissions and enrollment enabled. Registration and deployment remain paused;
Beta and Production have all three flags disabled. The pre-existing candidate
and binding-check evidence remain intact. No additional paid resource was added.

## Verification

- Linux native binary and private identity-file tests passed at Infra source
  `5ac7cf36f7f6a4cee07710dab683eddffbad94b2`.
- Local Worker-runtime tests verify actual Ed25519 enrollment, heartbeat and
  replay rejection. Actual D1 API tests cover concurrent one-use consumption,
  forged keys, expired tokens, privilege escalation and revoked signatures.
- Operator asset tests require signed admission, explicit active Development
  role, exact hostname and permitted paths before reading assets.
- Windows Rust tests, strict clippy, formatting and strict Worker TypeScript pass.
- The initial installer CI rehearsal successfully created the private identity;
  its unprivileged metadata check failed because the directory is protected.
  That check now uses sudo. Final installer CI passed at Infra source
  `5933b32f35cb914fbbd211b199b966d08bb1390f`,
  [push run 37693282600](https://github.com/VeloraMCDev/infra/actions/runs/37693282600),
  including first-install, private permissions, no Docker membership, repeated
  install refusal and a stopped service before enrollment.
- The exact three-file agent artifact was checksum-verified locally and on Hermes.
  Its portable binary executes on Hermes and reports the same source SHA. It is
  staged under the operator's home; no service/account/identity is created yet.
- All 25 local Node tests, strict Worker TypeScript and workflow policy checks
  passed. Panel's two session/action tests and syntax check passed; Docs' three
  tests and 25 local link targets passed.
- Remote readback confirms harness schema version 3, zero agents, one existing
  candidate, Development enrollment 1 and all deployment flags 0.
- Compiled Worker source `9ce23a8ce08a203ddd26c08d8513f3bd36a25c6e`, successful
  [push run 37692856015](https://github.com/VeloraMCDev/infra/actions/runs/37692856015),
  was deployed without rebuilding after checking its exact file checksum/size.
- Panel static source `1a80c66037b25a8249cd6a6e1850bff478125bf1`, successful
  [push run 37692902600](https://github.com/VeloraMCDev/panel/actions/runs/37692902600),
  was attached to the same protected operator Worker after checking all files.
  Assets run through the Worker first; origin admission is enforced independently
  of the edge Access policy. Machine ingress cannot serve the operator screen.
- Live protected browser verification passed after renewed Access sign-in. A
  browser-specific fetch receiver bug was corrected with a regression test and
  deployed from exact Panel source `750b886cc89a56e664babfea226e53a4fb14a036`,
  successful [push run 37701280259](https://github.com/VeloraMCDev/panel/actions/runs/37701280259).
  The protected screen loads eight repositories, one candidate and zero agents;
  the enrollment button is enabled and the viewport has no horizontal overflow.
  No real enrollment token was issued/read by automation. API reads returned 200
  and confirmed deployments remain paused.
- Hermes now has the dedicated agent account and installed systemd unit. After
  operator enrollment, SSH readback confirms `active/running`, boot-enabled,
  successful process status and the expected native source SHA. The account has
  only its own group, without Docker membership.
- Protected agent API returns 200 with one `ONLINE` Development agent, exact
  source `5933b32f35cb914fbbd211b199b966d08bb1390f`, Linux/x86_64 and only
  `REPORT_HEARTBEAT`. The refreshed Panel visibly renders that ONLINE record.
  The enrollment code was cleared from the browser. Agent transport remains
  outbound HTTPS; no inbound agent listener or Docker network port is introduced.

## Files changed

- Infra: `agent/`, Cargo workspace/lockfile, protocol fixture, agent API/protocol,
  migration `0003`, OpenAPI paths, Worker routing, Linux artifact/installer CI,
  and protocol/D1/runtime/asset authorization tests.
- Panel: `deployment-ui/`, independent checksummed static artifact and CI.
- Docs: this checkpoint and deployment index.

## Architecture decisions

The heartbeat agent has its own account without Docker group membership. Linux
CI builds a portable musl binary; Hermes needs no package installation. Static
operator assets and APIs share one protected host, avoiding cross-origin tokens
and a second frontend service. Worker and Panel artifacts retain separate owner
source identities. Runtime account/host configuration stays in ignored operator
files; public examples are synthetic.

## Known issues / deferred work

Milestone 4's guide exit criterion passed: the real Hermes Development agent is
healthy in the protected Panel. The operator supplied sudo/password and one-use
enrollment input only in the terminal; the agent never supplied/read those values.
Status/key rotation commands, supported-version policy, signed jobs and local
desired-state reconciliation remain open. Fourteen-day CI artifacts are temporary
custody, not durable signed releases or a native updater.

The screen is a bounded enrollment view. Full deployment workflows, typed Compose
execution, exact GHCR digests, health gates, timelines and the failed-image rollback
drill are subsequent milestones. Application migration and production cutover
across all eight owners are still incomplete.

## Security notes

No real key/token is printed or exported. Enrollment issuance is an explicit
operator action; it is never exercised by browser automation with real tokens.
Terminal rejection stops the daemon with exit 78, and systemd does not restart
that exit. Network failures, 429 and 5xx use bounded backoff. Revocation does not
stop unrelated workloads. Database restore is not part of this bootstrap.

## Next milestone

Implement Milestone 5's harmless Development service using the exact immutable
GHCR digest, a deployment lock, durable Workflow and signed typed agent job.
