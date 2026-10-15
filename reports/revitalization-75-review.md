# Independent 75% batch review

## Status and authority — October 12, 2026

**Initial lazy-page review only; final batch review pending the lead's ready message.**
This is not a 75% certification, release approval, native/OS acceptance, runtime
performance measurement or budget claim. Only this report was written; production
and test files were not modified. No broad build, commit, branch or deployment ran.

Read repository instructions, MONOREPO.md, docs/BOUNDARIES.md,
docs/VALIDATION.md, docs/DEPLOYMENT_TARGETS.md, completion-checklist.md,
revitalization-status.md and the baseline/agent ledger. The current checklist
still reports 38/76 (50%). Its 19 items and four fixed checkpoints per item remain
unchanged. The original attached master-plan text is not present in this review
conversation; the checklist is the available fixed-definition reference. Lead
should supply any additional batch instructions before final acceptance.

Final scope will include App/LazyPage/page-loads, operation presentation/Home,
tokens/Modal/CommandPalette/focus/Settings/primitives, crash diagnostics and
isolated core content_cache. Workers' assertions and earlier validation totals
are not counted as independently passing this evolving batch.

## Initial verdict

No confirmed application-cache ownership or rejection-eviction blocker in the
inspected lazy-page source. The fixed local import registry preserves page names;
Home/Login/Setup stay eager. App retains the selected-instance/view keyed boundary
for gameplay pages, and Settings retains its separate navigation branch. Loaded
module objects, not page instances/account snapshots, are cached. No new IPC,
persisted identity, URL-derived import, credential handling or install mutation
appears in this change.

PageLoads attaches its rejection handler before returning the request, deletes
only the matching rejected request, defers synchronous loader execution and
coalesces concurrent requests. Installed Svelte await-block source guards settled
callbacks with an effect-destruction flag. This supports stale-render safety by
inspection, but does not substitute for rendering App during navigation/context
changes, and does not cancel imports or module evaluation.

### L1 — Medium: Retry is not a universal module-recovery path

Locations: launcher/src/components/LazyPage.svelte:19,
launcher/src/lib/page-loads.ts:9, launcher/src/lib/pages.ts:5.

Evicting the application promise does not invalidate the JavaScript module
loader's failed evaluation cache. Independently executed synthetic dynamic imports
of a module that throws during evaluation: two PageLoads attempts both reject,
but the module evaluates only once. The real loaders reuse the same specifier.
Therefore the Retry button can repeatedly return the same failure for this class
of error. This does not disprove recovery for transient fetch failures; those
must be exercised against the actual packaged/browser asset-loading path.

Recommendation: treat Retry as best effort, add a bounded persistent-failure
explanation/route home, and avoid promising successful recovery. Any reload/update
escape hatch must explicitly handle active preparation/game state; do not
automatically reload or add cache-busting URLs as a blind workaround. A module
render/setup exception after a successful import is also outside this await
rejection UI; decide/document whether a separate render boundary is required.

### L2 — Acceptance gap: rendered stale-state and retry-focus behavior untested

Locations: launcher/tests/page-loads.test.mjs:1,
launcher/src/components/LazyPage.svelte:12,
launcher/src/App.svelte:64.

The four current tests exercise PageLoads and source-text wiring, not mounted
LazyPage/App. Their out-of-order test proves cache separation, not suppression of
an obsolete page mounting, child effects, or a stale failure alert. Retry removes
its focused button when the pending branch mounts; this component defines no
post-retry focus destination. Actual resulting focus and keyboard continuity need
rendered acceptance, not a passing cache test. Generic loading/status text also
does not name the destination; verify announcements with synthetic page names.

## Concrete acceptance tests for the lead

1. Mount the production LazyPage with controllable module promises. Request page
   A, switch to B, resolve B then A; assert only B mounts, A effects never run,
   and no A alert/loading content returns. Repeat with A rejecting late.
2. Through App, select instance A while a gameplay page is importing, switch to
   instance B, then resolve the import. Assert one current child mount and its
   instance-derived requests use B. Repeat logout/account removal, Setup entry,
   and page-capability revocation while the import is pending; obsolete pages
   must not mount or issue IPC. Include leaving Settings during its pending import.
3. Reject, activate Retry with Enter/Space, show pending, then resolve. Verify
   only one retry import despite rapid activation and another consumer; maintain
   selected account/instance, preparation owner/status and existing persisted data.
   Repeat consecutive rejection then success and navigation away during retry.
4. Mount/unmount/remount a successfully imported page. Assert its module is reused
   but local component state is fresh; gameplay context effects and listeners
   initialize/clean up exactly as before. Confirm Settings tab selected from the
   update action still opens About after delayed loading, including without login.
5. Exercise a real chunk fetch failure and a real module-evaluation failure in a
   disposable synthetic browser fixture. Document which Retry cases recover and
   show a usable escape path for persistent failures. Test render/setup throw
   separately so it is not confused with import rejection.
6. Keyboard-only navigation through loading, failure, Retry, success and Home:
   focus remains visible, does not become trapped/disappear into the document,
   and failure/status announcements occur once for the current destination.
   Test 320px-width scrolling and prefers-reduced-motion after final tokens land.

## Independently executed evidence

- `node --experimental-vm-modules --test launcher/tests/page-loads.test.mjs`:
  **4 passed, 0 failed**, no skips, initial snapshot only.
- Read-only inline Node assertions: immediate concurrent retry after rejection
  coalesces and calls the successful loader once; **passed**.
- Read-only inline Node module-evaluation failure experiment: application eviction
  does not force module re-evaluation; **confirmed limitation**, not browser/native
  recovery acceptance. The synthetic data URL was not written into product source.

## Initial source identity

HEAD: `4f639266a515c3cb05769b055a4772c4dcb89cb8`, with uncommitted evolving changes.
SHA-256 values below identify this initial inspection, not final batch approval.

| File | SHA-256 |
| --- | --- |
| launcher/src/App.svelte | `940e1541b4c5c065dd294788b6f40c59b18171a0ce5c3488fd717f8c19c129df` |
| launcher/src/components/LazyPage.svelte | `11c88351d20a1073e4c24b141bf8dfee7fbce248c863176251142ccebfa4e346` |
| launcher/src/lib/pages.ts | `207e6f6313f08e4fad6a5246dddfea819d384d0c32befe120fe91a952f724a86` |
| launcher/src/lib/page-loads.ts | `d6898e5a8cc73d267739673c82ddf1901657dcebd347673bc8a9696ed77f5866` |
| launcher/tests/page-loads.test.mjs | `c3a0d87c8e84117b3e563adf2c8c6c9859c389311401cfa0e7c614378a49db85` |

## Remaining gates before final review

- Lead ready message, final diff and final hashes; revalidate changed source rather
  than carrying this snapshot forward as passing.
- Independent operation-state/retry/cancellation ownership and Home review.
- Independent fail-closed/cache containment and diagnostics redaction review,
  including authorization failures, corrupt entries and hostile synthetic values.
- Independent design/keyboard/focus/contrast/reduced-motion review of final tokens
  and all changed primitives/dialogs; rendered synthetic evidence where available.
- Focused final tests and recorded raw outcomes. Concurrent broad builds remain
  prohibited; unavailable native/OS/runtime/budget evidence remains explicitly open.
- No final checkpoint credit is granted by this preliminary report.

## Additional preliminary scope — bundle graph and lazy inputs

Lead supplied bundle metrics and requested this additional review; workers are
not all ready. Final review still waits for a further lead message.

Independently reran the four loader tests and three bundle-metrics tests together:
**7 passed, 0 failed/skipped**. These are cache/script/source-wiring tests, not
rendered instance-state acceptance. The inspected lazy implementation is unchanged
from the initial hashes above; L1/L2 and acceptance tests remain applicable.

### Metric graph verdict

The current six-file static closure is correct for the inspected build. An
independent TypeScript AST traversal of actual emitted import/export declarations
found exactly six static modules totaling **468244 raw bytes**, matching the
manifest metric. The same independent traversal of the saved baseline found one
static module totaling **814453 raw bytes**. Thus the baseline's missing manifest
does not conceal static dependencies in this particular artifact. Do not generalize
that HTML-only fallback to arbitrary builds: modulepreloads need not exhaust their
transitive static dependencies.

Recomputed both artifacts using the existing Node **24.21.0**, zlib
**1.3.2.1-motley-8002e91**, and independently reproduced the lead's saved values:

| Metric | Saved baseline | Inspected lazy build |
| --- | ---: | ---: |
| Entry raw / gzip bytes | 814453 / 222833 | 246725 / 71686 |
| Static closure files | 1 | 6 |
| Static closure raw / summed gzip bytes | 814453 / 222833 | 468244 / 124913 |
| All adjacent JS files | 9 | 24 |
| All adjacent JS raw / summed gzip bytes | 881396 / 245651 | 910975 / 267571 |

Static closure decreases by **346209 raw bytes**, while total adjacent JS increases
by **29579 raw bytes**. Splitting therefore does not establish total-download savings.
Summed per-file gzip is an estimate using local compression, not wire transfer size,
native startup speed, or elapsed loading/parse time. It excludes CSS/fonts and runtime
dynamic imports; mock boot in particular can fetch assets outside the static closure.
Hidden-preview FCP evidence is discarded, as requested; no paint/startup credit.

### M1 — Reproducibility/robustness follow-up, not a current graph blocker

Location: launcher/scripts/bundle-metrics.mjs:1.

The JSON omits Node/zlib version and compression settings. Running unchanged script
and unchanged artifacts with default Node **25.6.1**, zlib **1.2.12**, produced baseline
gzip **222632** and final summed closure gzip **124950**, rather than the lead's
**222833** and **124913**. Raw bytes and closure membership matched. Record runtime
and compression settings with final metrics to avoid misclassifying this as source
drift or mixing unlike gzip baselines.

The helper unions every manifest `isEntry`, not just entries referenced by index.html,
and totals only `.js` files immediately beside the first HTML entry. That is adequate
for this inspected single-entry, flat-assets build, not a generic multi-entry or
nested-assets inventory. HTML parsing is double-quote/attribute-order specific.
Path checks reject lexical traversal but do not establish symlink containment. This
is a read-only local build-analysis tool, not a cache/security containment boundary.
No current-build miscount was observed. Suggested future fixtures: unrelated second
manifest entry, nested JS, single-quoted/reordered HTML, missing physical imported
file, and entry/static-preload deduplication. Scope output honestly if these formats
remain unsupported rather than claiming universal graph coverage.

### Additional inspected artifact identity (not final source)

| File | SHA-256 |
| --- | --- |
| launcher/scripts/bundle-metrics.mjs | `d492d441ec5532fc423b4817803ab3a925173464adb6c36c46a8caaba7f8228f` |
| launcher/tests/bundle-metrics.test.mjs | `80563a9a7444d9c2dc6c18360eeeb41728b75e00d3ca2fa6f2e6fb5b681a9f11` |
| launcher/vite.config.ts | `4aa109cca2d424f43908f18fa71a4ff76edb4aa9730106971e6062bc0f9985d3` |
| launcher/dist/.vite/manifest.json | `5ac52a40d4620d230163830b295002729f6043e91e0aebe952f94c101b2054e4` |
| saved baseline-dist/index.html | `4c4b1c3115edd4e05a3bf11c03f5e703db6839a3fde0694723e823c643a9e50f` |
| launcher/dist/index.html | `cff9f2e6d990960930fc2feee985ed70b96d16ae631d80de55fc311303ca4aee` |

`manifest: true` adds build-analysis metadata; by inspection it does not change
persisted identities or authorize deployments. These dist snapshots predate final
worker integration and cannot establish the final source-to-artifact correspondence.

## Evolving implementation review — diagnostics ready, other workers pending

This section independently inspects actual source, not worker assertions. Final
batch certification still waits for settled source hashes and the lead's ready
message. No production/tests were edited. No Rust build was launched concurrently.

### Diagnostics: bounded source acceptance

Inspected createDiagnosticReport and CrashDialog, and independently ran their
nine tests. Report generation emits fixed allowlisted signature summaries rather
than attempting regex masking of arbitrary original text. The recent tail, line
prefix, state/stage enums, integer exit code and final output have explicit bounds.
Unknown extension fields are not serialized. No original labels, URLs, paths,
tokens, account/instance/run IDs or crash-report filename are echoed in the shared
text. Categories/advice are qualified and current preparation context is explicitly
uncorrelated with the crashed run. Copy awaits completion; Export uses a fixed
filename and revokes the object URL; neither uploads automatically. Existing native
open/retry actions remain separate. No new redaction leak was found in this scope.

Combined independent command:
`node --experimental-vm-modules --test launcher/tests/diagnostics.test.mjs launcher/tests/operation-presentation.test.mjs launcher/tests/store-preparation.test.mjs`:
**34 passed, 0 failed/skipped** after the operation worker's additional cases landed.
This does not establish native clipboard/download/file-opening/game retry acceptance.

### Operation state: object-reference concern checked against actual store

At store.svelte.ts:372, stage/progress handlers mutate the current `app.launch`
object's properties; they do not replace it on each emission. Replacement occurs
at operation start and terminal running/error/idle transitions. Consequently
OperationPresentation's reference check does not presently erase kind/pending/
cancellation on normal progress ticks. The helper tests cover mutated progress,
repair-specific retry, stale replacement and cancellation races. Fixed phase labels
and bounded counters avoid raw engine-error exposure. Cancellation-ended wording
does not claim native cancellation confirmation.

The controller is view-local: navigating away and remounting Home loses its action
kind, so externally started operations decline typed retry. That is an explicit
limitation, not persisted/reload recovery. Keep actual-store identity regression
coverage if future progress handling switches to immutable launch replacements.

### A1 — Medium, confirmed: new Play text fails default contrast

Location: launcher/src/components/OperationStatus.svelte (local `.primary` rule).
Independent DOM inspection in synthetic `?mock=main` at 960×600 found Play text
`rgb(236,237,241)` on solid `rgb(109,106,245)`, 13.5px, no gradient. Computed sRGB
contrast is **3.5684:1**, below the 4.5:1 normal-text criterion. Merely switching to
white is not a general solution: allowed custom cyan/light-grey accent examples
against existing light text yield approximately 1.54:1 and 1.06:1. Use semantic
on-accent foreground/background with contrast-safe treatment, and cover supported
custom/server themes. This blocks an unqualified contrast/accessibility pass.
Default muted text on opaque default surface computes 5.4865:1; this does not certify
translucent, image-backed, hovered, disabled or custom-theme states.

### A2 — Medium, source-confirmed: background launch shortcut bypasses modal scope

Location: launcher/src/components/OperationStatus.svelte:56.
The window Ctrl/Meta+Enter handler guards busy/account/Home/repeat but not
`defaultPrevented`, composition, editable targets, or an active dialog/inert Home.
Inert isolation does not suppress a window listener. CommandPalette's input Enter
handler prevents default but does not stop propagation; a modified Enter can select
a command and then reach the background launch listener. Guard consumed/editable/
composing events and inactive/inert modal context; test that no `play` request is
issued while any dialog or palette owns keyboard focus. This hazard existed in
PlayButton's earlier shortcut too; it remains relevant to the new scoped keyboard
acceptance, not a falsely claimed newly introduced regression. No native launch
was triggered to test this observation.

### Focus/design rendered evidence and limits

Used DOM evaluation and keyboard operations in lead-provided tab_3, synthetic
mock data, at 960×600. Ctrl+K focused the palette combobox with an existing active
option; Tab moved to Close, next Tab wrapped to input, Escape removed the dialog
and restored inert attributes (zero remaining). Original focus was body in that
exercise, so it is not proof of restoring a focused opener. Nested-dialog order,
Shift+Tab, disappearing/disabled opener, no-focusable dialog and screen-reader
behavior still require explicit acceptance. Source uses a per-document top-dialog
stack and restores preexisting inert values, which is appropriate in principle.

Temporarily added then restored the user reduce-motion class in the synthetic DOM:
orb/hero/actions computed animation and transition durations became 0.000001s and
delays zero. This validates those sampled rules only. System reduced-motion CSS
explicitly excludes `.casino-motion`; confirm/document that exemption and test its
user-preference behavior before comprehensive motion acceptance. No OS setting was
changed or certified.

At a diagnostic 320×600 viewport, Home repair extended past the right edge; native
tauri.conf.json explicitly sets minimum 960×600. Thus this is a below-supported-
minimum observation, not a claimed native small-window regression. Restored 960×600.
No screenshot was obtained or credited; DOM rectangles/colors are not visual snapshots.

### Isolated content_cache: security and contract findings by inspection

- Fixed namespace/digest/size filenames cannot inject paths. Missing/malformed digest
  declines rather than caching unverified input; sha1 is explicitly compatibility.
  Root creation is exclusive, unknown entries/corruption fail closed, object/disk/entry
  limits precede population, and retrieval rechecks size/digest through scratch.
  No downloader/installed-store integration or automatic eviction/recovery is present.
- Existing-root `create` returns Collision, including after process restart. This is
  an ephemeral isolated prototype, **not persistent cache reopening or resume**.
  State inventory is memory-only; do not grant native restart/recovery acceptance.
- Retrieve's capacity check deliberately includes existing retained bytes plus a
  full verification scratch copy. The source test `bounds_include_staging_and_entry_limit`
  explicitly expects a 3-byte published entry under a 5-byte disk quota to be
  unreadable, but readable under 6 bytes. This is real scratch reservation, not an
  accidental arithmetic double count. It is nevertheless a usability limitation:
  accepted populations can later be impossible to retrieve when full. Document a
  reserved-read budget or change admission/retrieval policy before integrating it.
- Scratch Drop unlinks before its File field is dropped, and successful retrieval/
  population likewise unlink with the handle live. Close-before-unlink is the more
  portable design and should get fault tests. This review does **not** assert a
  proven Windows failure: actual share modes and filesystem behavior need acceptance.
  Cleanup ignores errors in Drop; leftover scratch causes subsequent inspect to
  fail closed rather than silently adopt it. Post-link unlink error can return an
  error even though state/publication already succeeded; define/test that outcome.
- Path inspection is lexical/metadata-based, not descriptor-relative handle-bound
  containment. Parent/root/scratch may race with external mutation between checks
  and open/hash/read/link/unlink. Private Unix permissions reduce exposure but do
  not certify hostile-process/Windows ACL/reparse containment. Keep these explicit
  gates; no production authorization or native security claims from prototype tests.

No independent Rust cache test execution is credited in this section; worker test/
benchmark assertions remain awaiting focused final execution and matching hashes.

### Lazy retry remediation and mounted-harness blocker

The newer LazyPage adds a stable named focus region, named loading announcement,
and explicit best-effort retry explanation. This addresses L1's misleading recovery
expectation by source inspection and provides the requested focus destination; L2
still needs mounted evidence. Optional `load` defaults to the fixed registry and
is not populated from external/user data in production.

**L3 — Confirmed fixture blocker:** on the dev server at
`http://localhost:1420/tests/lazy-browser.html`, the dedicated reviewer tab_5 remained
blank and waiting for fixture text timed out. Inspecting its actual HTML module
import returned `state_unsafe_mutation` with stack pointing to `load` in
LazyPageHarness.svelte: reactive `requestCount++` runs synchronously from production
LazyPage's `$derived`. Svelte forbids that reactive mutation. Defer the fixture's
loader bookkeeping to a microtask, mirroring PageLoads, or otherwise move it outside
the derived reaction. This is not a production fixed-loader failure; it prevents
the new harness from establishing any mounted lifecycle acceptance as written.
Review and rerun stale resolve/reject, unmount-before-settle, retry focus and child
mount/unmount after repair. The fixture alone does not exercise actual App selected-
instance/account/capability gates. A separately attempted URL on preview port 1422
served the launcher fallback, not the harness; that attempt earns no fixture credit.

### Inspected identities for this evolving snapshot

Diagnostics ready hashes:
- diagnostics.ts: `666e1787d0f8dada68f85593d6431f5f65788ef09864410f240271ac71356e49`
- CrashDialog.svelte: `3cd6030baed5ecf5852a83551d597aa70c67ba806ce711f56fa90a9641df4fcd`
- diagnostics.test.mjs: `fc593d9d246611803839bb675fdd7f6e44be298f0ee05d8204bd563f13bb9a6e`

New lazy snapshot hashes:
- LazyPage.svelte: `6630eeeacf124fd60d1736c27fecc4e62f9375b3a50319a94fe763ad49535236`
- LazyPageHarness.svelte: `f8e8e36d1cc7ea44f2dc9fbedb5705a72a20b03b678ee2c58ea8468070c75354`
- LazyFixturePage.svelte: `ab0d2f01886e9d9ee0021b232d62735bd7219f1d9593c184cd1a8910d7c2087d`
- lazy-browser.html: `61f1ea635d5bf5de244696da60bc1869f46e032bfc281ffdc991568e236e6e3a`

Other source remains evolving; its inspected hashes must be recaptured/reconciled
after workers finish. A1/A2/L3 and explicit source/native gates prevent a blanket
75% or accessibility/security pass. No fixed checklist definitions are relaxed.

## Independent follow-up after initial all-ready message

**Superseded readiness:** lead subsequently advised that A1/A2, Settings tab
semantics and system-motion rules are being repaired. Final review is withheld
until the imminent new ready message and matching test/source hashes.

### Provenance correction: shared browser observations

Earlier tab_3 observations are historical/non-final and must not be combined with
concurrent lead harness activity as acceptance evidence. No further operation on
tab_3 is permitted. Reviewer created separate explicit tab_5 for the dev harness
and tab_6 for the synthetic production UI. Only their new independent results are
eligible below. No raster screenshot exists or earns visual-snapshot credit.

### Lazy fix independently verified in reviewer tab_5

Ready-message LazyPage captures `name`/`load`, then invokes its callback from
`Promise.resolve().then(...)`, outside the derived reaction. L3 is **resolved**:
the unchanged harness now mounts on dev port 1420 without unsafe reactive mutation.
The production loader still defaults to the fixed registry; synchronous custom-
loader throws also become awaited rejections rather than escaping the derivation.

Programmatic DOM clicks in isolated tab_5 independently established:
- Stats pending → Market pending → latest Market resolve → original Stats resolve:
  lifecycle is **only `mount:market`**; stale Stats never mounts.
- Switch to Stats, reject latest, focus/click Retry: active element is the stable
  region named Stats & leaderboards during pending and after success. Lifecycle is
  `mount:market, unmount:market, mount:stats` with no duplicate child.
- Switch to Market, unmount pending page, settle resolve; then remount, unmount,
  settle rejection: no new mount, no obsolete failure alert (zero alerts).

These are **programmatic synthetic DOM tests**, not semantic preview-click,
physical keyboard, screen-reader, native webview or App account/instance E2E.
L1's best-effort recovery limitation is appropriately disclosed. L2's tested
component-level stale/focus/unmount subset is resolved; App instance/account/
capability changes and real chunk-fetch/evaluation fault paths remain acceptance
gates. The extra wrapper does not cache application state or alter IPC identities.

### Independently executed final-candidate focused tests

- Prescribed Node 24: loader/metrics/diagnostics/operation/preparation-events/
  store-preparation suites: **50 passed, 0 failed/skipped**.
- Separately executed accessibility fixtures: **8 passed, 0 failed/skipped**,
  before the subsequent Settings navigation import was added. These results are
  not certified against that newer Settings source. The attempted combined command
  had mistakenly named nonexistent `design-accessibility.test.mjs`; only its actual
  50 listed tests are credited, not additional assumed design coverage.
- `CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher-core --lib content_cache::tests:: -- --skip controlled_local_benchmark`:
  **19 passed, 0 failed/ignored**, 66 filtered out. Cargo initially waited for the
  existing build lock and then completed; no broad workspace build was requested.
- Explicit already-built focused benchmark runner: **1 passed**, five iterations,
  each retaining 1 MiB committed payload, zero interruption scratch and full 1 MiB
  retry after 256 KiB interruption. Independently observed microseconds:
  cold `12693,10115,10324,10596,11345`; warm `4695,5162,4498,5183,4649`;
  interruption `1553,1915,1857,2176,1527`; retry `9755,30825,11071,10297,30817`.
  This is bounded artificial-reader/debug local fixture evidence, not install,
  networking, native desktop, physical-disk or resume performance.

### Cache follow-up disposition

Actual Scratch source now stores `Option<File>`, takes/drops it before unlink and
marks successful removal, preventing a second unlink after successful cleanup.
The new close/unlink-failure and committed-plus-scratch quota tests passed in the
independent 19-case run. The former handle-order concern is **resolved at source/
fixture level**, not universal Windows acceptance. Quota is intentionally retained
once plus a distinct scratch reservation. Its no-headroom retrieval limitation,
post-publication cleanup error outcome and fail-closed residue are explicitly
documented. Contract accurately declines persisted reopen/resume/shared-root/
hostile-writer/durable recovery claims. No further blocker to the isolated bounded
prototype was found under its exclusive-writer precondition. Native integration
and handle-bound path containment remain open; VEL-012 checkpoint 4 is not awarded.

### Settings warning decision, subject to final source

A named scrollable region's `tabindex=0` is intentional keyboard reachability,
not automatically an accessibility defect solely because a compiler classifier
warns. The subsequently inspected source now uses proper tablist/roving tabs and
labelled tabpanel `tabindex=0`; that is a stronger semantic implementation and must
be tested with the added navigation helper. Do not suppress warnings to manufacture
a zero-warning result. Older region markup/tests cannot certify this later change.

### Conservative checkpoint recommendation, not a modified ledger

- VEL-012 checkpoints 1–3: supported for the explicitly bounded isolated contract,
  prototype and independently reproduced synthetic cold/warm/interruption fixture.
  Checkpoint 4 stays open; no full master-plan shared persistent cache acceptance.
- VEL-016 checkpoints 1–3: supported for Home safe-state presentation and tested
  typed retry/cancel/failed-state fixtures. Native/E2E checkpoint 4 stays open.
- VEL-018 checkpoints 1–3: supported for approved bounded diagnostics contract,
  actual implementation and nine independently passing helper/handler tests.
  Native feature checkpoint 4 stays open.
- VEL-013: static chunk baseline/implementation evidence is supported; final
  comparable measurement credit requires final build-to-source correspondence.
  Runtime startup/no-regression checkpoint 4 stays open. No install/Java performance
  evidence is supplied for VEL-014.
- VEL-015: semantic-token vocabulary exists. No raster/visual-snapshot credit;
  accessible-components/contrast/keyboard acceptance awaits A1/A2 fixes and final
  evidence. Review findings do not automatically constitute a passing checkpoint 4.
- VEL-017: focused keyboard and compact-layout source/fixture evidence is useful;
  final Settings/system-motion changes must be rechecked before final recommendation.
  An independent review with unresolved findings is not blanket accessibility credit.
- VEL-019: no full final sweep or broad security/data/compatibility acceptance is
  certified by focused runs. Existing durable/native/rollback/restore gates persist.

No 57/76 total is certified or reverse-engineered from a target. Definitions and
denominator remain fixed; the lead must record the accepted bounded evidence and
leave unverified outcomes unchecked.
