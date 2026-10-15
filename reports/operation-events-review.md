# Independent launcher operation-event review

Date: October 12, 2026. Final source re-review of the controller, actual store, native host identity propagation, mock, permanent tests and source CI. Read `MONOREPO.md`, `docs/BOUNDARIES.md`, `docs/VALIDATION.md`, `docs/DEPLOYMENT_TARGETS.md` and the earlier preparation/cancellation reviews. Only the two requested review reports are authored in this review; production and tests are unchanged.

## Verdict

**The two previously identified event defects are resolved in the current source for their stated histories.** Bootstrap-known foreground live logs no longer require a recovered preparation token, and an observed exited run cannot be reinserted by a later `running` event during the same store lifetime. No new blocking defect was identified within that bounded fix scope. This supersedes the previous report's open P2 findings, not its native/recovery/process gates. It is source acceptance, not native application or release acceptance.

## Re-reviewed fixes

### Closed — Bootstrapped foreground run lost live logs

`launcher/src/lib/store.svelte.ts:87` restores `app.running` and selects its first run as `consoleRunId`. At `launcher/src/lib/store.svelte.ts:306`, live logs require a matching known **run and instance**, and must not belong to an exited run. When no admission/preparation is active, logs for that foreground run append directly; a local preparation request/UUID is no longer required. Unknown runs and wrong-instance payloads still fail. This recovers subsequent live output, not pre-reload log history or every running window's console.

During admission/preparation the alternate path still uses `PreparationEvents.event`: the request instance must match immediately, and the admitted UUID plus current request identity must match when the callback executes. A queued log additionally checks the foreground run at application time. The fix does not loosen preparation progress/state ownership or cancellation identity.

The permanent actual-store test at `launcher/tests/store-preparation.test.mjs:167` initializes with a nonempty bootstrap snapshot, sends a live log without a local request, and rejects wrong-instance and unknown-run logs. Unlike the original diagnostic probe, it tests `init()` itself.

### Closed — Late running resurrected an observed exited run

`launcher/src/lib/store.svelte.ts:200` maintains the module-lifetime `exitedRuns` set. `game://exit` records its run before removing that run from inventory (`:315`); the state listener rejects `running` for a tombstoned run **before** global insertion or controller dispatch (`:286`). The guard therefore also covers obsolete/current-owner and other-instance late running events with that run ID. Legitimate other-owner running inventory updates remain outside the single foreground preparation filter.

The permanent actual-store test at `launcher/tests/store-preparation.test.mjs:176` delivers running → identified exit → idle → late running and asserts both empty inventory and idle display. This establishes the intended injected history in source, not actual Tauri duplicate/reorder behavior. Tombstones are neither persisted nor bounded; their memory grows with distinct exit IDs during the store lifetime. Do not prune them casually while retaining an unlimited replay guarantee.

## Controller, host and mock conclusions

- `launcher/src/lib/preparation-events.ts:21` blocks cross-call admission, retains the invoke promise, and caps the pending event buffer at 128. Admission replays only callbacks for the current request and admitted UUID. Terminal phases reject further transitions; running cannot return to preparing. State floods can evict older state callbacks, so bounded buffering is not an unlimited lossless replay contract.
- `launcher/src/lib/store.svelte.ts:202` guards play/repair with controller busy state rather than visible UI. `cancel()` captures the request before awaiting its token; rejection sends no unidentified cancellation. Admission failure restores the prior controller request but does not fully restore the previous visible UI/console selection.
- `launcher/src-tauri/src/commands.rs:475` returns the manager-created operation UUID, supervises completion against ownership, and parses required cancellation UUIDs. `launcher/src-tauri/src/preparation.rs:33` matches both instance and operation and retains the slot through worker completion. There is no instance-only fallback.
- `launcher/src-tauri/src/game.rs:20` carries instance/operation identity in state and flattened progress; logs additionally carry run ID. Registered runs receive fresh UUIDs, running precedes watcher startup, and the watcher removes that exact run then emits exit followed by run-scoped idle. Exit retains its existing run identity contract without requiring an operation UUID. The close-after-launch branch exits before ordinary run registration/watcher setup; no process-ownership acceptance follows from these event changes.
- `launcher/src/lib/mock.ts:124` propagates operation identity, returns UUID admissions, models token-matched cancellation and repair without a run, and supplies bootstrap run IDs. It remains a single-running-game simulation, clears preparation immediately on cancel, and treats malformed/missing tokens differently from the native command. It cannot prove join/drain, multiple-window behavior or process ownership.

## Tests and CI evidence

Inspected **19 permanent frontend tests: 9 controller and 10 actual client-compiled store scenarios**. The store fixture transpiles and Svelte-compiles production `store.svelte.ts`, uses the real controller/client runtime, and substitutes IPC, theme/context, bootstrap and timers. Coverage includes early repair completion, stale same-instance and wrong-instance delivery, terminal protection, busy cross-call admission, captured/rejected cancellation, run-scoped exit, and both fixes above. These are stronger than copied-store fixtures but remain synthetic event delivery.

`launcher/package.json:12` exposes the VM-module test runner. `.github/workflows/ci.yml:61` runs launcher test/check/runes/build under Node 24 after lockfile install; the workflow is public-repository gated, read-only and has no deployment step. It does not run native Rust launcher acceptance. The user reports the Node 24 rerun underway and final native regression pending. **Neither completion nor passing results are independently observed here.** No duplicate suite/build or live store was opened. Scoped `git diff --check` passed; this is whitespace evidence only.

## Explicit remaining gates

1. **Native transport/bootstrap:** exercise an actual webview reload with a live game and subsequent output; bootstrap occurs before asynchronous listener registration, without an atomic snapshot/subscription barrier. Exit/running/log delivery in that gap can still be missed. Foreground selection is the first bootstrap run and is not automatically reassigned when it exits. No comprehensive reconnect reconciliation is implemented.
2. **Bootstrap active preparation:** bootstrap has run identities, not the active operation UUID/phase. A reloaded frontend cannot recover authoritative progress or a cancel target for an already-preparing host. Do not call restored-run console support active-preparation recovery.
3. **Native histories:** validate rapid running/exit before admission resolves, overlapping windows/owners, rejection recovery, stale terminal delivery and close/hide/minimize behavior. Logs for nonforeground runs during preparation are intentionally filtered; the 19 cases do not establish all these histories.
4. **Process/resource/filesystem:** abortion/join is not proof detached blocking work or child/watchers drained; close-after-spawn ownership and interprocess filesystem safety retain their earlier gates. No native subprocess, cancellation-drain, bootstrap-active-preparation, schema-upgrade or release proof is claimed.

## Reviewed snapshot

SHA-256 of the principal changed store/test and current CI; host/controller/mock were also inspected directly:

```text
a2486dd4d3460525e9b9b717e87bcedd3930c6c58a94dd45928dd6155a1e0a5d  launcher/src/lib/store.svelte.ts
2ebf30e70d35ef8e99ee5d9ab7ae797aa35cdea01aa93770a565035dd834c6c9  launcher/tests/store-preparation.test.mjs
fecd54018bcba3f878a843887aa92d2a7f1582b69114e8368a446c8d5d38db84  .github/workflows/ci.yml
```

This verdict applies to the inspected source, not later concurrent edits or pending test outcomes.
