# Independent telemetry consent/state review

Date: 2026-10-15. **Verdict: BOUNDED ACCEPT for the activity-reporting consent fix only.**
This is not native product acceptance, a release approval, a comprehensive privacy
audit, or a claim that opting out cancels requests. No new blocker found in the
shared-settings change within this scope. The limits below are part of acceptance.
**Additional UI disclosure verdict: HOLD on the current timing wording**, as
explained below; the native bounded acceptance is unchanged.

## Scope and evidence

- Reviewed `MONOREPO.md`, `docs/BOUNDARIES.md`, `docs/VALIDATION.md` and
  `docs/DEPLOYMENT_TARGETS.md`; inspected current `state.rs`, `telemetry.rs`,
  settings load/update, account token lookup, shutdown admission, Panel request
  construction, and activity call sites. Compared telemetry with HEAD, which had
  no `send_stats` gate. The surrounding checkout contains concurrent lead work;
  this verdict does not approve every change in its state/telemetry diff.
- Review owns only this report. No production edits, real stores, OS keychain,
  external service requests, commits, deployments, releases or subagents used.
  Executed tests use synthetic temporary stores and loopback HTTP.
- Source SHA-256 at review: `state.rs`
  `61c87456b1084d2c34e31587d1e9092874d4bac86411258390291fc76b8c0130`;
  `telemetry.rs`
  `e2fadd9cdb930a6707a1947e08740e27e015f07941994e60f596e0b67ab09f9a`.
  Paths are under `launcher/src-tauri/src/`. Re-review material later changes.

## Consent and lock ordering

- `telemetry.rs:19`: capture reads consent before resolving panel, token or active
  account. When false is observed it returns `None`, so public send/background
  do not reach those identity lookups or start an activity request.
- `telemetry.rs:58`: retained Recorder background calls check the shared current
  setting before obtaining a shutdown work permit, cloning payload strings or
  spawning. `telemetry.rs:43` checks again inside send before constructing a Panel
  client/request. A recorder captured while enabled therefore observes a later
  successful in-process opt-out; it does not retain an obsolete settings copy.
- Each consent read guard is a temporary dropped at the end of the `if`
  condition, before panel/account/secrets/shutdown access. No new settings guard
  spans HTTP `.await`, spawn, or shutdown admission. `panel_url()` also releases
  its settings read before account acquisition. Existing token lookup holds an
  account read while reading secrets, but the new gates add no nested settings
  acquisition there. No new lock-order cycle identified in the inspected paths.
- `state.rs:112` obtains a shutdown permit and then the settings write guard.
  `Shutdown::enter` releases its internal mutex before returning that permit;
  the permit is not a held shutdown mutex. Settings persistence is synchronous
  while holding the write guard, so telemetry reads may wait for disk work.
  Poisoned locks still panic via `unwrap`; no new poison-recovery guarantee.

## Persistence and identity compatibility

- `Arc<RwLock<Settings>>` changes internal ownership only. Initialization still
  loads `settings.json` and `Settings` remains the serialized value: no Arc,
  lock or new consent schema is written. Existing valid `send_stats: false`
  deserializes unchanged. `Settings::update` clones, saves, then publishes the
  candidate under the write guard; a failed save preserves previous memory.
  Consequently a failed opt-out save does **not** disable previously enabled
  telemetry. The caller must not present a failed save as successful consent
  revocation.
- Default consent remains `true` (`settings.rs:126`); missing/corrupt/wrong-typed
  settings can fall back to defaults. This fix is not explicit opt-in or
  fail-closed consent for unreadable settings. External edits to the file are
  not watched by retained recorders.
- The change does not alter stored account IDs, paths, credential service names,
  updater/signing or Minecraft identities. Recorder continues to own the captured
  panel string, optional token and username. Subsequent account/panel changes do
  not replace these fields at send time. The endpoint and event fields remain
  POST `/api/v1/launcher/events`, `kind`, `instance_id`, `username`; optional
  bearer handling remains in PanelClient. No native credential migration is
  certified by these tests.
- **Existing identity capture limitation:** panel resolution, token lookup and
  username capture are separate lock acquisitions. Token lookup itself resolves
  the panel again. Concurrent panel/account mutation during capture can therefore
  produce a mixed panel/token/username snapshot, potentially including a token
  selected against a different panel than the destination first captured.
  The new consent gates do not introduce or solve this pre-existing race. A
  coherent snapshot across concurrent identity changes is not accepted here;
  that stronger promise needs a separate fix and authenticated race fixtures.

## Race/privacy boundaries

The send-time consent read is the final consent check, **not** a transaction
with socket admission. Consent may change after the read and before request
construction or transmission; that request may still be sent, even after an
opt-out save returns. Already admitted/in-flight requests are not aborted or
retracted. Background admission can race with opt-out too; the send recheck
suppresses work only if it observes false. Capture can finish reading identity
after a concurrent opt-out, though send still rechecks. Rapid off/on transitions
do not permanently revoke old Recorder instances: if enabled again before their
send check, they may report using their original snapshot. There is no durable
event queue or revocation epoch added by this change.

Acceptable wording: activity capture/admission/send skip when their consent
checks observe off; retained recorders consult current settings before sending.
Not acceptable: “no requests after clicking opt-out,” instantaneous cancellation,
atomic identity capture, or a launcher-wide network disable switch. Authentication,
manifest, updater and other non-activity requests are outside this preference.

## Tests and remaining gates

- Inspected `/tmp/velora-revitalization-20261014/telemetry-consent-tests.log`:
  focused Cargo run completed, **3 passed / 0 failed**.
- Independently executed each of those three exact tests from the existing
  `target/debug/deps/velora_launcher_lib-410ded5001140d46` test binary:
  **3 passed / 0 failed**. No separate compilation claimed. A requested
  `cargo test --locked -j1 -p velora-launcher --lib telemetry::tests` with the
  prescribed Node 24 directory on PATH waited on the lead's build lock; terminated
  only that reviewer-owned waiting Cargo process rather than compete with it.
- Lead logs observed complete: `telemetry-host.log` **141 passed**, and
  `telemetry-feature-host.log` **148 passed**, under the same temporary log
  directory. These are inspected lead evidence, not independently rerun suites.
- After the lead confirmed Cargo was free, independently repeated
  `cargo test --locked -j1 -p velora-launcher --lib telemetry::tests` with
  `/Users/alyssasitler/.npm/_npx/387698761821791d/node_modules/node/bin` prepended
  to PATH: **3 passed / 0 failed**, exit 0, no compilation required. Log:
  `/tmp/velora-revitalization-20261014/telemetry-consent-independent.log`.
  Also inspected `telemetry-check.log`: default non-test check finished
  successfully. Source hashes above remain unchanged. The implementation report
  explicitly acknowledges the post-check race and no in-flight cancellation;
  its later-switch attribution statement must retain the during-capture caveat
  documented here. No native patched-binary rerun claimed.
- Off-path tests call actual public send/background and captured Recorder paths,
  not a duplicated consent helper. Persisted-opt-out test uses `update_settings`,
  reloads the JSON preference, and sends through a previously captured recorder.
  Enabled loopback test verifies method/path, exact synthetic body, absent bearer
  for an offline account, and retained username after mutation.
- Coverage limits: no fresh AppState restart from preseeded false settings; no
  authenticated token or changed-destination snapshot fixture; username mutation
  is not a full active-account switch; no deterministic queued-send/revocation
  interleaving, in-flight cancellation, concurrent capture race, failed-save
  consent integration or lock stress test. The no-request assertion samples
  `accept()` immediately; it alone cannot prove asynchronous silence. Static
  early-return inspection supports these particular disabled-background cases.
  Enabled success exercises Recorder send, not enabled background scheduling.
- Fixture lifecycle: each case owns a UUID-named temporary directory and a
  `127.0.0.1:0` listener. `Fixture::drop` removes its directory; the enabled
  case moves fixture ownership into the server thread and joins that thread on
  success. The Recorder owns its HTTP/settings/shutdown snapshot rather than
  needing files after capture, so server-side directory cleanup is safe for
  this request. Test-only secret initialization uses the temporary fallback
  file rather than the OS keychain. Cleanup is verified by code inspection,
  not an added post-join filesystem assertion.
- Fixture bounds are adequate for this controlled small request, not a general
  hardened HTTP server: accept has an eight-second monotonic deadline; each
  stream read has a five-second timeout; the production request has a
  five-second timeout. Reads have no total-body deadline or byte cap and the
  tiny response write has no explicit timeout. If client send panics via
  `unwrap`, explicit join is skipped and the thread may briefly outlive the
  test until accept/read timeout and Drop cleanup. Cleanup itself unwraps,
  so filesystem cleanup failure can panic, including during unwinding. These
  are test-harness limitations, not evidence of production leaks; do not
  describe all failure paths as synchronously joined or absolutely bounded.
- Native activity errors reported by the lead motivated this fix; this reviewer
  did not reproduce the desktop scenario. Native/UI consent persistence and
  supported-OS/release acceptance remain separate gates. Broader identity/privacy
  guarantees remain HOLD until separately demonstrated.

## Additional Settings disclosure/state review

- Inspected `launcher/src/pages/Settings.svelte:718`: the Launcher category
  binds the existing `s.send_stats` to the existing `saveSettings` callback.
  Disclosure identifies activity events, username, instance ID and configured
  panel. The separate paragraph distinguishes authentication, server connections
  and server-collected gameplay data. No new preference/schema is introduced.
- Settings imports the production `components/ui/SettingSwitch.svelte`; the SSR
  test loads that component rather than substituting a fake consent control.
  The new fixture checks both preference values, named switch, checked state,
  associated help via `aria-describedby`, data disclosure, and save binding.
- Independently ran Node 24
  `node --experimental-vm-modules --test launcher/tests/accessibility.test.mjs`:
  **10 passed / 0 failed**, exit 0. Log:
  `/tmp/velora-revitalization-20261014/telemetry-accessibility-independent.log`.
  This is rendered SSR/source-contract evidence, not a click/IPC/persistence or
  screen-reader/native end-to-end test. No patched native binary rerun.
- **Disclosure correction gate:** “Turning this off stops new reports, not
  requests already in flight” excludes only in-flight requests, but code also
  permits already-check-admitted requests that have not reached the wire.
  Furthermore `store.svelte.ts:226` debounces saves for 300 ms, then serializes
  them through SettingsDomain; native consent remains enabled until the save
  reaches and successfully commits in Rust. New reports may therefore be
  admitted while the UI already displays off, not just finish in flight.
  Save errors roll back matching fields and show an error toast; this is not
  an instantaneous native revocation control. A source regex for the current
  sentence cannot validate that privacy promise.
- Suggested bounded wording: “After this setting is saved, activity checks skip
  new reports while sharing is off. Reports already admitted or in flight may
  still be sent. A failed save does not change the saved preference.” Keep the
  data/recipient disclosure and required-networking distinction. Exposing pending
  or saved status beside consent would improve clarity but is not claimed here.
  Until wording acknowledges successful save and admitted-but-unsent requests,
  do not treat this UI disclosure as fully accepted.

Actual billed USD: **unknown**. Existing **62/76 unchanged**; no acceptance-count
or cost improvement inferred from this review.

## Independent final UI disposition — 2026-10-15

**UI disclosure HOLD resolved; BOUNDED ACCEPT for the revised disclosure and
existing preference binding.** This supersedes the initial UI wording disposition
above without removing its history. Native bounded acceptance remains unchanged.

- Rechecked `launcher/src/pages/Settings.svelte:718`: the disclosure retains
  activity/data/recipient identification and now explicitly qualifies enforcement
  with successful save, permits already-admitted or in-flight reports, and states
  that a failed save does not change the saved preference. The separate
  authentication/server/gameplay distinction remains. This matches the reviewed
  native pre-send policy without promising instantaneous toggle-time revocation.
- Rechecked `launcher/README.md:99` and the implementation report's disclosure
  follow-up: both acknowledge debounced/queued persistence and preservation of
  the old saved preference on failure. The mixed identity snapshot race remains
  a separate unresolved risk, not a claimed fix or a newly accepted guarantee.
- Rechecked the SSR fixture: both values still require a named switch, matching
  checked state, associated description and existing save binding. Assertions now
  require saved-setting qualification, already-admitted/in-flight caveat and
  failed-save text. Independently repeated Node 24
  `node --experimental-vm-modules --test launcher/tests/accessibility.test.mjs`:
  **10 passed / 0 failed**, exit 0. Independent log:
  `/tmp/velora-revitalization-20261014/telemetry-accessibility-final-independent.log`.
  Also inspected the lead's `telemetry-accessibility-final.log`: **10 passed**.
- Recomputed native source hashes; both match the values recorded above. No
  native changes or patched-binary native rerun inferred. All earlier limitations
  remain: SSR is not interaction/IPC/save-error/native acceptance; pending-state
  presentation and supported-OS/privacy race coverage are not certified.

Only this review report was edited by the reviewer. Actual billed USD remains
unknown; **62/76 unchanged**. No deployment, release or broader privacy approval.

## Independent strengthened-test disposition — 2026-10-15

**BOUNDED ACCEPT retained.** This addendum supersedes the earlier three-test
coverage inventory and telemetry source hash, not the privacy/race boundaries or
the preserved disclosure HOLD/resolution history.

- Current SHA-256: `launcher/src-tauri/src/telemetry.rs`
  `a606aac6e86df3b2441788f1bab9a6f83b72563aff900258aa71f50a704e11c6`.
  `state.rs` remains
  `61c87456b1084d2c34e31587d1e9092874d4bac86411258390291fc76b8c0130`.
  Inspected additions are under `#[cfg(test)]`; reviewed production consent
  checks and Recorder ownership are unchanged. Corrected UI wording unchanged.
- Inspected lead final logs and waited for their completion before running
  Cargo: `telemetry-host-final.log` **143 passed / 0 failed** and
  `telemetry-feature-host-final.log` **150 passed / 0 failed**. These remain
  inspected lead results, not independently rerun full suites.
- Independently ran with the prescribed Node 24 directory prepended to PATH:
  `cargo test --locked -j1 -p velora-launcher --lib telemetry::tests`.
  **5 passed / 0 failed**, exit 0, no recompilation required. Log:
  `/tmp/velora-revitalization-20261014/telemetry-consent-five-independent.log`.
  Lead `telemetry-consent-tests-final.log` also records five passing tests.
- `persisted_opt_out_survives_state_restart` successfully persists false and
  constructs a second AppState from that same temporary directory. It explicitly
  asserts loaded false, capture rejection and successful no-op public send.
  This closes the missing fresh-state preference-load coverage. It is an
  in-process reconstruction, not an OS-process restart: the original AppState
  remains alive, and the fixture account was not persisted, so the explicit
  loaded-setting assertion is essential; no-request alone could also follow
  from absence of a loaded active account. No native installed-store migration
  or process-crash claim follows.
- `failed_opt_out_save_preserves_enabled_preference` places a directory at
  `settings.json`, verifies update failure, enabled in-memory consent and
  `capture().is_some()`. This closes the missing consent-specific save-failure
  integration check for rejection of a nonregular destination. It deliberately
  does not send; the listener's silence is not evidence that enabled reporting
  was suppressed. It does not inject a mid-write/rename/fsync failure or prove
  preservation of a pre-existing valid on-disk settings document in that case.
- Enabled identity fixture now upserts a distinct offline account with a distinct
  UUID/ID after capture; `AccountsFile::upsert` selects it as active. The loopback
  request still carries the original username and no bearer. This supersedes
  the earlier “username mutation is not a full active-account switch” limitation
  for the offline path. Authenticated token/destination switching and concurrent
  mixed-snapshot capture remain untested and unresolved.
- Fixture cleanup/timeout limitations, missing deterministic revocation races,
  true-by-default/unreadable-setting fallback, no in-flight cancellation and
  SSR/native acceptance boundaries remain as previously documented. No patched
  native binary rerun or broader privacy guarantee claimed. Existing risk
  registration does not itself resolve the mixed-snapshot/default-consent risks.

Reviewer edits remain limited to this report. Actual billed USD unknown;
**62/76 unchanged**.
