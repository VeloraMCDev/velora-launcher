# Independent telemetry snapshot security/state review

Date: 2026-10-15. **Verdict: BOUNDED ACCEPT for telemetry capture coherence and
credential destination scoping only.** No must-fix defect found in this scope.
This closes the mixed destination/token/username acquisition issue identified in
`reports/telemetry-consent-review.md` for telemetry capture, not other auth callers.
It is not whole-product privacy, native, release or acceptance-matrix approval.
**Ledger remains 62/76; actual billed USD is unknown.**

## Scope and provenance

- Read root `AGENTS.md`, `MONOREPO.md`, `docs/BOUNDARIES.md`,
  `docs/VALIDATION.md` and `docs/DEPLOYMENT_TARGETS.md`, plus the prior telemetry
  consent and Panel-client reviews. Inspected current telemetry capture/send,
  state helpers/mutators, Panel construction, accounts, Secrets, transaction
  fencing, shutdown and production callers taking the relevant locks.
- Reviewer owns and edits only this report, using `apply_patch`. No production
  or test edits, Cargo invocation, real stores, keychain access, network request,
  commit, deployment, release or subagent used by this reviewer. The checkout
  contains extensive earlier/concurrent work; this is not approval of that diff.
- Independently checked extraction compatibility in memory with Node: moving
  `parse_base` back into the constructor and removing `same_base` recreates the
  prior Panel-client review SHA-256
  `0297efe0135d042cca1badf4786c8aeea6392b85135382e662e9c16209be8b8f`.
  Reversing the settings helper extraction recreates the prior telemetry review's
  state SHA-256
  `61c87456b1084d2c34e31587d1e9092874d4bac86411258390291fc76b8c0130`.
  These comparisons substantiate unchanged constructor parsing and baked-policy
  selection, not provenance for a released binary.

Reviewed SHA-256 values, paths relative to `launcher/src-tauri/src/`:

| Path | SHA-256 |
| --- | --- |
| `telemetry.rs` | `95bf64ba4386e9b5dfda5d3271258ae133bcec01ff1477b274d46cc54612e988` |
| `state.rs` | `4757be8daa4206d58923274d9d2f302f667c9ef9efe545e2ed0523d0b16366fc` |
| `panel_client.rs` | `d58a112be1f3bb00247441db9e186dce2c9a21d950add4e2d4921ba41674a464` |
| `telemetry/snapshot_tests.rs` | `b82958f94a7c59ab2c3bd940c3a818937f5f2a6377709a1254e45444dfdb3116` |
| `accounts.rs` | `66472f3ebf23add9a75628f5c00d4297f0eae4aeaadda4640b87fcad6050907d` |
| `secrets.rs` | `4979185198192062e554641165dd5b6e29a5126d2f5991db1754a84072fd1eb6` |

## Snapshot and lock ordering

- `telemetry.rs:19` holds one settings read guard across consent, resolution by
  `AppState::panel_url_from_settings`, acquisition of the accounts read guard,
  active-account lookup and optional `Secrets::get`. Username and credential ID
  come from that same borrowed active account. Guards live through construction
  of the owned Recorder and drop on every return; capture has no async boundary.
  It no longer resolves the Panel or active identity a second time through
  `accounts::panel_token`.
- The new nested order is **settings read -> accounts read -> Secrets data
  mutex**. `accounts::remove` and `commit_panel_account` hold accounts write
  through `remove_with_accounts`/`set_with_accounts` and publish the candidate
  account before dropping that guard. Capture cannot see their intermediate
  credential/account publication. Account selection/update takes accounts write;
  its production callbacks do not request settings or call telemetry.
- Settings mutations in bootstrap commands and game preference persistence take
  settings write and change the supplied settings only. Existing Panel-token and
  manifest lookup resolve Panel before accounts, releasing the settings guard
  first. Bootstrap clones accounts before later settings access; game launch
  similarly clones active identity before its later settings read. No inspected
  production accounts-held -> settings wait was found.
- `Secrets::get` takes only its private data mutex and calls the existing
  filesystem-only `ensure_clean` fence. Production secret writes/transaction
  validation do not reacquire AppState settings/accounts or invoke state callbacks.
  The callback-based `set_with_commit` is test-only. Session refresh reads an
  owned Secret, releases the mutex before HTTP, and later writes secrets without
  waiting for settings/accounts. No inspected secrets-held -> settings/accounts
  wait or new lock-order cycle was found.
- Shutdown admission releases its internal mutex before returning a work permit;
  that permit is not a held mutex. Capture releases all its guards before public
  send/background proceeds. Recorder's consent guard is a temporary in the `if`
  condition, dropped before request construction, shutdown admission, spawn and
  network `.await`. No new state lock spans network work.
- This is not a nonblocking or poison-recovery guarantee. Settings writes can
  now wait behind capture waiting for account/secret persistence. Synchronous
  filesystem work and the existing `unwrap` poison behavior remain. Future
  callbacks/mutators must preserve the order and avoid reentrant settings reads.

## URL scope and compatibility

- `panel_client.rs:85` compares two successful results of the exact parser used
  by `PanelClient::new`. HTTP(S), host presence, empty username/no password, no
  query and no fragment are required on both sides. Invalid values cannot compare
  equal merely because their original strings match. Path case stays significant;
  this is full normalized base equality, not host-only or prefix equality.
- Existing URL parsing canonicalization, default-port handling and trailing-path
  slash normalization apply identically to comparison and request construction.
  Host spelling/default-port/trailing-slash equivalents can match, whereas
  different tenant path, path case, host, scheme or nondefault port cannot. This
  adds no new DNS, redirect, proxy or server-side decoding security policy.
- Only kind `panel` with an explicit matching `account.panel_url` can read its
  stored Panel token for telemetry. Missing origin, foreign/invalid origin,
  offline kind, absent secret or pending-transaction fence yields no bearer.
  The existing `Secrets::get` fence remains in use; capture does not bypass it
  with direct credential-map access or fetch a key from the OS store.
- **Fail-closed applies to the token, not the event or username.** With an active
  account and configured Panel, missing/foreign origin still captures that
  account's username and can send an unauthenticated event to a valid destination.
  An invalid destination may produce a Recorder but `send` rejects client
  construction. No active account/no configured Panel still means no Recorder.
  This deliberately does not promise that every transmitted username belongs to
  the destination's account domain.
- The helper preserves locked-baked precedence, unlocked user-setting preference,
  baked fallback and trailing-slash handling. It does not reinterpret stored URLs
  or change persisted IDs, schema, credential/updater identities or paths.
- `accounts.rs:275` retains unrelated legacy behavior: missing account origin can
  still yield a token, present URLs use case-insensitive string comparison after
  trimming slashes, and callers can still resolve destination separately. No
  other auth caller was migrated to `same_base`; this review does not approve or
  claim to fix their races/compatibility risks. Retained recorders continue to own
  their old destination/token/username after settings changes or account removal.

## Consent semantics retained

- Consent and destination now come from the same settings value held stable
  during capture; false returns before account/credential acquisition. Retained
  recorders still recheck shared current consent in both background and send.
- Successful persisted opt-out suppresses work that observes false at those
  gates; an already-admitted/in-flight request is not cancelled. Capture may
  finish before a competing opt-out writer acquires settings. This does not
  establish toggle-time revocation, a transactional UI/native boundary, or a new
  guarantee for external file edits.
- Existing default-true/unreadable-settings fallback and save-before-publish
  behavior remain. A failed opt-out save preserves prior enabled consent. Nothing
  here supersedes the prior review's UI, privacy or persistence limitations.

## Tests inspected and validation limits

- Waited for the new file and inspected its final corrected fixture and
  `#[cfg(test)]` path wiring at `telemetry.rs:89`. The twelve scope cases exercise
  exact/normalized matching and ten rejected or absent origins, retaining
  username and stored token assertions. Additional tests cover an identical
  invalid destination/account URL, offline identity with a stored token, and a
  pending journal whose evidence remains unchanged.
- The blocked-accounts test observes settings `try_write` failure while capture
  is waiting for accounts, directly exercising the new lock lifetime. It uses a
  bounded observation deadline but cannot time out a deadlocked thread join.
  The 2,000-capture/2,000-writer fixture checks matched tuples with settings and
  account changes made together under the intended lock order. It is stress
  coverage, not exhaustive interleaving/model-checking evidence or proof that
  separate production IPC mutations form one cross-resource transaction.
- The authenticated loopback test captures first Panel/identity/token, changes
  state to a second Panel/identity/token, then checks the original prefixed route,
  first bearer, username and event kind on actual HTTP, plus silence at the
  second listener. It uses `no_proxy`, bounded reads and timeout. `.invalid`
  scope fixtures never call send; unit-test Secrets initialization uses temporary
  fallback keys, not a real keychain. Existing five telemetry consent tests also
  remain in the host suite.
- Inspected completed lead logs under
  `/tmp/velora-revitalization-20261014/`: `telemetry-snapshot-tests-fixed.log`
  reports **7 passed / 0 failed**; `telemetry-snapshot-host.log` reports
  **150 passed / 0 failed**; `telemetry-snapshot-feature.log` reports
  **157 passed / 0 failed**. These are inspected lead results, not reviewer-run
  Cargo suites. No parallel Cargo was launched while the lead owned validation.
- The earlier `telemetry-snapshot-tests.log` had five passes and two failures.
  The final fixture removes an existing synthetic ID before upsert, avoiding the
  duplicate-ID fixture bug when changing Panel identity; the wire assertion now
  checks header lines without assuming the sliced last header retains CRLF.
  Neither correction changes production code or weakens the intended boundary.
  The earlier formatting-diff log is not claimed as a passing format check.
- Missing independent runtime rerun, exhaustive scheduling, supported-OS/native
  keychain, installed-data upgrade, auth-wide migration, redirect/proxy and
  release validation remain explicit limits, not newly accepted matrix items.

Only `reports/telemetry-snapshot-review.md` is authored by this reviewer.
**62/76 unchanged. Actual billed USD unknown.**

## Independent runtime addendum — 2026-10-15

**BOUNDED ACCEPT retained; no blocking finding.** This addendum supersedes the
initial no-reviewer-Cargo/no-independent-runtime-rerun statements above, preserving
their history. Other scope and acceptance limits remain unchanged.

- After explicit lead Cargo handoff, independently ran both focused suites with
  reused artifacts. The first run used the default Node 25 PATH and passed seven
  snapshot and six Panel-client tests. Repeated both with the required existing
  Node 24 directory prepended to PATH, verifying **Node 24.21.0** and **Rust
  1.98.1**, without installing or downloading tooling.
- `cargo test --locked -j1 -p velora-launcher --lib telemetry::snapshot_tests`:
  **7 passed / 0 failed**, exit 0. Node-24 independent log:
  `/tmp/velora-revitalization-20261014/telemetry-snapshot-node24-independent.log`.
- `cargo test --locked -j1 -p velora-launcher --lib panel_client::tests`:
  **6 passed / 0 failed**, exit 0. Node-24 independent log:
  `/tmp/velora-revitalization-20261014/telemetry-snapshot-panel-client-node24-independent.log`.
- Independently ran scoped `rustfmt --check --edition 2021 --config
  skip_children=true` on `telemetry.rs`, `panel_client.rs` and the new snapshot
  module: exit 0. Also inspected the completed lead non-test
  `telemetry-snapshot-check.log`: successful dev check, no warnings shown. Full
  host/feature results remain inspected lead evidence, not independent repeats.
- Recomputed all six SHA-256 values in the table after these runs; every value
  matches. In particular the three production review targets and new module
  remain `telemetry.rs` =
  `95bf64ba4386e9b5dfda5d3271258ae133bcec01ff1477b274d46cc54612e988`,
  `state.rs` =
  `4757be8daa4206d58923274d9d2f302f667c9ef9efe545e2ed0523d0b16366fc`,
  `panel_client.rs` =
  `d58a112be1f3bb00247441db9e186dce2c9a21d950add4e2d4921ba41674a464`,
  and `telemetry/snapshot_tests.rs` =
  `b82958f94a7c59ab2c3bd940c3a818937f5f2a6377709a1254e45444dfdb3116`.
- Reviewer-run tests used synthetic temporary stores/fallback keys and loopback
  only, with no real keychain, installed data or GUI repeat. Missing origin
  suppresses only optional telemetry bearer; no auth-wide migration, whole-privacy
  acceptance, matrix increment, deployment or release is inferred.

Reviewer edits still own only this report. **62/76 unchanged; actual billed USD
unknown.**
