# Bounded installation UX — VEL-016 checkpoints 1–3

Date: October 12, 2026. Starting checkout HEAD: `4f63926`. Implementation only;
no milestone percentage, native acceptance, release or deployment is awarded.

## Instructions and scope

Read root `AGENTS.md`, `MONOREPO.md`, `docs/BOUNDARIES.md`,
`docs/VALIDATION.md`, `docs/DEPLOYMENT_TARGETS.md`, launcher README, current
store/event contracts and the supplied local
`VELORA_REVITALIZATION_MASTER_PLAN.md`. No deeper AGENTS applies to the five
owned paths. The plan's VEL-016 Home/instance/launch/repair work and installation
UX tasks guide this bounded implementation; its usability/E2E gate remains open.
Existing unrelated checkout edits are not owned or changed by this task.

Changed paths:

- `launcher/src/pages/Home.svelte`
- `launcher/src/components/OperationStatus.svelte` (new)
- `launcher/src/lib/operation-presentation.ts` (new)
- `launcher/tests/operation-presentation.test.mjs` (new)
- `reports/installation-ux-implementation.md` (new)

No store API, IPC, Settings, App, Modal, global style, identity, persistence,
dependency or engine policy changes. No branch, worktree, commit or deployment.

## Implemented behavior

1. Home uses the new production operation component instead of PlayButton.
   Play, verify/repair, cancel, action-specific retry and individual running-window
   Stop controls remain usable through existing store functions. Ctrl/Meta+Enter
   launches only when allowed. Global preparation disables new starts across
   instances; another-instance notice directs the user to select the owner.
   Instance selection does not key-remount the operation component; current
   operation presentation survives switching instances while Home remains mounted.
2. Stage-level status uses fixed safe labels, not arbitrary engine text.
   Positive totals produce bounded phase-local progress. Zero/invalid totals
   remain indeterminate, not 100%, including work reported without a total.
   File counts are separate. A failed operation retains the last observed safe
   phase; unknown externally initiated action types never receive an invented retry.
3. Production presentation helpers own action eligibility, local attempt scopes,
   retry intent and cancellation request tracking. They reject stale selection,
   replaced same-instance attempts and duplicate actions. Launch retries require
   an account; repair retries preserve repair intent. Store cancellation is called
   immediately after validation so its existing admission/token snapshot supplies
   the actual native operation identity. Late cancellation responses do not alter
   replacement presentation. Existing engine admission/event ownership remains
   the authority, not the UI's local sequence.

Safe expandable diagnostics contain only action type, last observed phase and
bounded outcome. Raw messages, labels, engine logs, URLs, paths and tokens are
not reflected by this component; generic guidance suggests connection, disk and
permission checks without pretending to classify the engine failure. Stop errors
are also fixed safe text. Repair follows existing managed-file rules and may
update managed files; this is not a new cleanup/reset/data-retention guarantee.

The polite atomic live region announces phase/state changes, not every byte/file
tick. Native progress semantics, named controls, disclosure keyboard behavior and
local focus-visible outlines are provided. Home's action region scrolls when tall;
no new progress animation or global style changes are introduced.

## Cancellation and persistence limits

`LaunchState` has no operation token or cancelled outcome. The store discards the
native `Cancelled` message and turns it into idle (or running when windows exist).
The UI therefore records only a local cancellation request. It remains preparing
and blocks retry while the store remains preparing, even after cancel IPC resolves.
Idle following the request is labelled **ended after cancellation request**, with
an explicit unconfirmed-outcome explanation, not confirmed native cancellation.
Running or error wins a cancellation race. A cancel transport failure allows
another cancel and never invents success.

Same-instance scope replacement is detected from the store's new LaunchState
object when preparing starts. Native operation IDs and stale events are validated
by existing store/host code; they are not exposed or duplicated here. Very short
external operations that start and end between observations cannot have their
missing phase/action reconstructed. Presentation memory does not survive leaving
Home, app restart or a missing manifest; the store still owns current operation
progress, but terminal retry intent and cancellation request history are local.
There is no persisted installation-health model or installation-complete claim.
No ETA or overall installation percentage is inferred from phase-local byte totals.
Existing console/toast/other-page diagnostics are outside this exclusive scope;
this is not a launcher-wide raw-diagnostic redaction claim.

## Validation and evidence

- Production-helper tests import the actual TypeScript module, not copied logic.
  Fifteen synthetic cases cover positive/zero/invalid totals, safe labels and
  errors, failed repair/launch retries, account eligibility, admission cancellation,
  duplicate cancel/start, cancellation failure and terminal races, foreign selection,
  same-instance stale scopes, late responses and coarse announcements.
- Targeted combined run includes existing preparation-event and actual-store
  fixtures. These retain native-token snapshot and stale-event contract coverage
  but run with synthetic admissions/IPC, not a native webview. Command:
  `node --experimental-vm-modules --test tests/operation-presentation.test.mjs tests/preparation-events.test.mjs tests/store-preparation.test.mjs`
  from launcher; final result: 34 passed, zero failed (15 new helper cases).
- `npm run check`: zero errors; the initial run had two existing warnings in
  Settings (region tabIndex) and CommandPalette (missing aria-selected). Final
  `npm run check -- --output machine` found zero errors and only the Settings
  warning; unrelated concurrent edits changed the warning count. No owned-file diagnostics.
- `npm run check:runes`: passed. No full build run, no concurrent full builds.
- Isolated browser preview `tab_2` at 960×600 uses `?mock=progress`. Actual UI
  interaction exercised Verify/repair → determinate progress → Cancel → ended
  after request, retaining repair retry. DOM observation showed phase-local 95%,
  byte/file counts and blocked start buttons during preparation. Synthetic
  store injection exercised zero-total indeterminate state and failed state with
  token/URL/path-bearing inputs; expanded safe details reflected none of them.
  Document-width checks found no horizontal overflow in those fixtures.
- Screenshot requests through preview_snapshot failed repeatedly with
  PreviewAutomationExecutionError, including save mode. No screenshot artifact
  exists; DOM observations are not screenshot/visual acceptance. Initial browser
  interaction used shared `tab_1` before the lead identified the conflict; subsequent
  actions use only explicit `tab_2`. Do not use the disturbed shared-tab interval
  as production measurement evidence.

## Remaining gates and rollback

PENDING — independent UX/state review; native Windows/macOS/Linux admission,
cancellation, repair, offline/error/disk/permission scenarios; cross-instance and
multi-window E2E; screen-reader and keyboard-only testing; rendered screenshots,
contrast/system scaling and small-window/reduced-motion review; restart behavior
and existing installed-data compatibility. Synthetic browser work is not native
installation, process termination, file preservation or release acceptance.

Rollback is limited to the five owned paths: restore Home's previous PlayButton
wiring and remove the new component/helper/tests/report as appropriate. No stores,
schemas, persisted IDs, installed files or deployment state need migration.
