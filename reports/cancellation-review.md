# Cancellation-token review

October 12, 2026. Independent source review only; writes limited to this report. Root boundary/validation/deployment instructions and prior `preparation-review.md` apply. No production edits, duplicated tests, live stores, commits, branches, PRs or native acceptance.

## Verdict

Backend token ownership and frontend snapshot-before-await prevent stale cancellation from aborting a replacement, including the same instance. **The identified P2 admitted-token-loss gap is addressed by the current store changes; no new scoped blocker found in this re-review.** This is source acceptance of the cancellation-token contract, not authoritative frontend state/event or native acceptance. Main's reported validation and remaining final frontend rerun are distinguished below.

## Resolved finding: rejected overlap loses the admitted token (P2)

Previously a rejected overlapping B request replaced admitted A's cancellation target permanently. Current `play` and `repair` first return when `app.launch.state === 'preparing'`, before changing UI, logs or request and before issuing IPC. This guards every store caller, including Repair/CrashDialog paths, rather than relying on the PlayButton's disabled state.

For admission attempts still possible after stale idle/error UI, both functions capture `previousRequest` before installing the new request. Their rejection catch restores it only if the rejected request is still current, retaining an admitted A token for subsequent cancel; an obsolete rejection cannot replace a newer request or clobber its UI. `cancel` still snapshots before awaiting, so this recovery does not retarget an already pending cancel. No implementation change is made by this reviewer.

## Confirmed source behavior

- `PreparationManager::cancel` requires both instance equality and active UUID equality before requesting abort. Wrong/stale UUID, wrong instance, repeated cancellation and finished worker are no-ops; ownership still remains until joined/finalized. Existing lease and lock-free supervisor publication remain unchanged.
- `launch`/`repair_instance` return the UUID of the actual admitted manager slot, not a frontend-generated token. `cancel_launch` parses the required `operationId`: malformed UUID returns error; missing required IPC field is rejected; a valid nonmatching token returns unit success without changing another owner. A fast completed operation can return an already-inactive token safely.
- `cancel()` snapshots the complete request object before awaiting its admission promise. A later request cannot change either captured instance or token. Rejected admission does not issue cancel. If the captured worker completes and a replacement starts while this await is pending, backend UUID matching makes the delayed cancel harmless. Repeated matching cancel is idempotent.
- Direct `stale_cancel_cannot_abort_a_replacement_for_the_same_instance` starts/aborts/joins/finalizes an old alpha owner, starts a live replacement alpha, and rejects both the old UUID and a random UUID while asserting the new handle is unfinished and uncancelled. It exercises actual manager identity checking, not a copied command fixture. It does not prove frontend or native integration.

## Compatibility and remaining gates

- **Intentional IPC contract change:** command names remain, but launch/repair success now returns a string instead of unit and cancel requires a new argument. Older callers must migrate together with the bundled frontend; retain no instance-only fallback. README documents this correctly. Persisted IDs, schemas, credentials, signing identities and dependencies are not changed by this token contract.
- Mock models global busy admission, UUID returns, instance+token matching, stale no-op and repair completion without spawning a run. It does **not** reproduce native join/finalization timing: cancel clears its slot immediately while `fakeLaunch` may still be sleeping; missing/malformed tokens are also no-ops rather than host errors. Mock evidence cannot establish those host contracts or subprocess ownership. Its existing single `running` boolean also does not model all native repair/run exclusions.
- Progress, terminal game state and logs remain uncorrelated with preparation tokens. Old-game idle/log delivery and frontend reload/reconnection token recovery remain gates; the module-local request is not restored by bootstrap. The new guard relies on UI state, not an authoritative backend snapshot. Rejection restores the request but still sets UI error and leaves the attempted `launchingId`; it does not restore an active preparation display or ensure a visible cancel control. A restored historical/rejected request is not proven active. These are broader state/recovery gates, not a claim that this token fix redesigns events or solves arbitrary overlapping event histories.
- Abort/join still does not prove background filesystem work drained. Close-after-spawn ownership, detached game watchers/readers, native close/hide/minimize and interprocess filesystem safety retain the prior review's explicit gates. No native acceptance is claimed.
- Existing `tauri.ts` reports `launch_cancelled` after any successful cancel IPC, including valid stale no-ops. That telemetry is a request signal, not proof an operation was actually cancelled; unchanged wrapper behavior should not be used as acceptance evidence.

## Review evidence

Inspected current manager, commands, direct tests, production store, mock, IPC wrapper, PlayButton/InstanceSettings/CrashDialog callers and README against prior review; re-inspected the exact updated store diff and both guarded/recovery blocks. Initial scoped whitespace check passed; `git diff --check -- launcher/src/lib/store.svelte.ts` also passed after this fix.

Main reports eight client-compiled production-store scenarios passed with mocked IPC: empty cancel; delayed pending same-instance cancellation for play and repair with explicit idle allowing replacement; rejected admission without active owner; obsolete failure without UI clobber; play/repair cross-call guard; and denied-overlap recovery retaining the admitted target. These exercise real store code, not native IPC/event delivery. Main reports Rust **28 launcher passed / 1 ignored**, **43 core passed / 4 ignored**, and **7 online ignored**, plus initial frontend check/runes/build passed. Final frontend rerun after the store fix remains pending at this report update. No suite or harness was independently rerun by this reviewer; ignored tests are not acceptance evidence. This is a bounded source verdict, not release approval.
