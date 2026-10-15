# Velora revitalization status

## Rendered contrast and motion fixes — October 15, 2026

`design-gate-followup.md` records browser-discovered contrast fixes across shared
launcher controls, Settings, Social, Market, Player Guide and Casino navigation,
plus actual system/manual preference handling in Guild Wallet's JavaScript
balance animation. Final source checks: frontend 118, shared experience 8;
launcher check/runes/build, Panel consumer check/build and boundaries pass.
Two synthetic ten-accent matrices have no failures among 8180 checked text
groups, with 730 additional complex groups explicitly left for manual review.
A separate 24-cell HUD preview matrix verifies the review-requested local-white
correction against both light/dark branding text, accent extremes and opacity
samples. They are not native or whole-design acceptance. Independent feasibility audit
in `85-percent-acceptance-audit.md` finds no honest three-gate shortcut to 85%.
The unchanged ledger remains **62/76 (81.58%)**; no partial work is rounded up.

## Destination-bound Panel requests — October 15, 2026

The remaining generic token helper is removed. Manifest discovery now selects
credentials against its explicit requested destination: connecting to a new
Panel no longer attaches a token merely because it belongs to the previously
configured Panel. Current account/leaderboard/public-server requests capture
destination, token and selected-instance header together; telemetry shares the
same strict account-origin lookup. Instance manifests also deny bearers for
missing stored origins. See `panel-context-implementation.md` and its review.
Nash independently accepted this bounded source change and repeated 19 tests
across context, real manifest fetch, account helpers, game fetch and telemetry.

This intentionally fails closed for legacy originless accounts without changing
their IDs or encrypted stores. They may require a new scoped sign-in, and old
authenticated offline caches may no longer match their now-unauthenticated
authority. Installed legacy-upgrade/recovery acceptance remains open; no
transparent migration or whole-authentication certification is claimed.

Eleven new direct/loopback regressions pass. Final default host **161 passed**,
feature host **168 passed** (overlapping), non-test check passed; frontend
**102 passed**. An initial existing telemetry fixture `WouldBlock` failure is
retained in the logs; its accepted socket now explicitly uses blocking reads
and bounded write timeout. No patched-native GUI run or release follows.
The completion ledger remains **62/76 (81.58%)**.

## Stable activity identity snapshot — October 15, 2026

The prior review's mixed destination/token/username race is now addressed in
activity capture: settings/account read guards remain held through lookup, and
bearers require an explicit stored origin matching the request client's validated
base, including case-sensitive tenant paths. Missing/invalid/mismatched origins
fail closed for this optional bearer without modifying stored account data.
Other generic authenticated callers are not migrated or certified by this fix.
See `telemetry-snapshot-implementation.md` and its separate review.
Huygens independently accepted this bounded change, repeated seven snapshot and
six Panel-client tests, and verified scoped formatting and source fingerprints.

Seven direct regressions pass, including a deterministic destination-lock check,
concurrent tuple stress and actual authenticated loopback delivery after a Panel
and account switch. Final default host **150 passed**, feature host **157 passed**
(overlapping suites), non-test check passed. There is no patched-native GUI,
real-account or supported-OS acceptance claim. The ledger stays **62/76 (81.58%)**.

## Native evidence audit and activity preference — October 15, 2026

Fetched `origin`: HEAD remains `4f639266a515c3cb05769b055a4772c4dcb89cb8`,
with zero ahead/behind divergence. Existing uncommitted work is preserved.
The guarded real macOS/Tauri run now has an independent documentary evidence
audit in `native-current-host-observations.md`; it is not an independent GUI
replay. Actual failure/retry, unconfirmed cancellation, modal focus and busy-close
observations are partial evidence. Quit exit status is unknown; no game/helper,
keystore, successful install or supported-OS gate is certified. Owned native,
fixture and Vite processes are absent at this follow-up check.

The run exposed ignored `send_stats: false`. Production capture/send/background
now honor that preference, including recorders captured before a persisted
opt-out. Settings exposes the switch with accurate account/instance disclosure;
already admitted/in-flight requests are not canceled. See
`telemetry-consent-implementation.md` and the separate privacy/state review.
Aristotle independently accepts this bounded fix and corrected disclosure,
repeating five host tests and ten accessibility tests. The initial disclosure
HOLD and its resolution remain recorded in `telemetry-consent-review.md`.
Default host **143 passed**; feature host **150 passed** (overlapping suites);
default non-test check passed. Node 24 frontend **102 passed**, Svelte zero
errors/warnings, runes/build passed. Source changes have not been GUI-replayed.

The fixed completion ledger remains **62/76 (81.58%)**. Seven whole checkpoints
are still needed for 69/76 (90.79%); neither harness creation nor a narrower
privacy fix substitutes for their unchanged acceptance criteria. No commit,
branch, paid infrastructure, package, release, signing or deployment.

## Transaction I/O and live cache-denial follow-up — October 14, 2026

Final host sweep: **138 tests**, zero failures; non-test host check passes.
Independent transaction/root-order follow-up accepts the bounded slice, with
23 module and nine real-encryption integration tests independently repeated.
The actual instance-manifest caller now fences same-process offline access after
known-denial invalidation failure, and older in-flight successes cannot clear a
newer denial. Focused cache/gate suite: 19 tests. See
`account-transaction-io-review.md`, `manifest-denial-fence.md` and the separate
`manifest-denial-review.md`. Both bounded independent reviews accept their slices.
Durable offline authorization, native/OS/power-loss and complete
checkpoint gates remain open. Ledger stays **62/76 (81.58%)**, not 90%.

## Account restart transaction — October 14, 2026

Production sign-in/rename/logout now journal the existing account and encrypted
credential files as one recoverable pair. Startup recovers before loading
accounts; changed/corrupt/locked evidence blocks initialization and live pending
credentials/mutations fail closed. The transaction/I/O host sweep passes **134**, including
child-process and real encrypted restart fixtures; non-test host and four IPC
contracts pass. Named returned-I/O failures now cover rollback, commit/cleanup
fences and real-encryption restart behavior; they are not power-loss/OS fault
certification. Static data-root symlinks are rejected before key access. See
`account-transaction-implementation.md`. Independent review
is separate; native keystore/upgrade/OS/power-loss gates are not inferred.
The fixed ledger remains **62/76 (81.58%)**.

## Rendered accessibility fixes — October 14, 2026

Actual synthetic browser checks found and fixed pale-accent selection marks,
shared filled-action foreground/gradient contrast and clipped Settings categories
at 130% interface scaling. End/Home now expose and focus the last/first tabs;
all nine preset swatches and manual reduced motion were exercised. See
`rendered-accessibility-followup.md` for limits. Complete defined/native acceptance
is still open; the fixed ledger remains **62/76 (81.58%)**, not 90%.

## Cache authority implementation — October 14, 2026

The actual instance-manifest caller now binds offline reuse to Panel/account/
authentication/instance identity, atomically publishes compatible-shape cache
files, and invalidates matching authority on known 401/403/404 denial. Fifteen
focused tests include production-caller loops with synthetic `AppState` stores;
the final whole-host suite passes 102 and the non-test host check passes.
See `manifest-cache-authority.md`. The ledger remains **62/76 (81.58%)**:
no independent full-cache/native gate is awarded for this bounded source work.

## Final measurement audit — October 14, 2026

**62/76 = 81.58% verified checkpoints.** Separate independent review accepts
VEL-014 checkpoint 3 after reconciling all three retained profile records,
120 flow samples, 15 successful JVM outcomes and every published timing value.
See `install-final-measurements.md`, `.json` and its separate review. The final
repeat includes regressions; no global speedup or wall-clock no-regression claim.
No production code changed for this audit. Earlier 260-pass scoped validation
remains applicable, not a new test count. Fourteen checkpoints remain, including
native launch/loader acceptance; seven more are needed for at least 90%.
Historical milestones below are not the latest aggregate. No commit or deployment.

## 80% milestone — October 13, 2026

**61/76 = 80.26% verified checkpoints.** User's requested 80% target is met
without changing the ledger or native acceptance definitions. VEL-014 checkpoints
1–2 now have a matched full Vanilla/Java profile and independently accepted
duplicate-resolution optimization. Real local JDK 21 detection and harmless JVM
execution supplement synthetic automatic-runtime/native-archive fixtures, not
Minecraft or OS/release acceptance. All 258 library discovery checks remain;
actual second-pass repair/final-presence regressions pass. Matched timings are
mixed, so no global faster-install claim. See `install-java-profile.md` and
`install-profile-review.md`.

Stable locked scoped sweep: **260 Rust passes** (92 host, 136 core, 13 utility,
19 shared), four explicit profiles and seven online cases ignored by default.
Both lead profiles and the independent rerun pass. Non-test host, four IPC
contracts and source/security checks pass. No new frontend/full-OS/release claim.
Origin remains 0/0 divergent; changes stay local and uncommitted. No paid capacity,
signing, release or deployment. Disk about 1.8 GiB free; model cap superseded,
actual billed USD unknown. Fifteen checkpoints remain; see `90-percent-gates.md`.

## Shutdown implementation — October 13, 2026

User superseded the model spending cap; no infrastructure purchase/deployment.
Implemented owned-work admission, normal exit-request draining, preparation
cancellation/finalization, tracked texture/telemetry/session tasks, and blocking
filesystem ownership that survives awaiting-wrapper cancellation. Lead host
check passes, **92 launcher library tests** and four IPC-surface tests pass.
Transitive utility cache I/O no longer detaches from its cancelled owner; accepted
rename responses commit under their original lease; updater partials have exclusive
creation and close-before-cleanup ownership. Stable core132/utility13/shared19
checks bring this scoped Rust sweep to **256 passes**. Separate review accepts the
bounded source fixes, not a native shutdown or release matrix.
`shutdown-implementation.md` records preserved game-detach policy and uncovered
native/helper boundaries; separate review belongs to `shutdown-review.md`.
**59/76 (77.63%) remains the verified ledger**, not 90% or native acceptance.

## Actual-method fault follow-up — October 13, 2026

Added actual rollback-method reverse-before/after-persist interruptions, actual
execution-time changed/occupied/missing evidence and combined-journal write
failure fixtures. The earlier review's three coverage requests now have bounded
tests, not claims of full failure/OS acceptance. Focused sync: 40 passes/zero
failures/one timing profile ignored. Exact scope and final separate review:
`sync-fault-coverage.md`, `sync-fault-review.md`. **59/76 = 77.63%, not 90% or
release-ready.** Disk measured 2.6 GiB free at batch start; actual USD unknown.

Final core library validation passes **132 tests**, zero failures, three explicit
profiles ignored. Final independent recovery rerun: 21 passes; non-test library
check passes. Source/identity/workflow/publication/secrets/links/scoped-format
checks pass. Other segment runs remain prior evidence, not newly rerun totals.
See `90-percent-gates.md` for the remaining 17 checkpoints; ten must close for 90%.

## Cold-sync follow-up — October 13, 2026

**59/76 = 77.63%, not 90% and not release-ready.** Implemented independently
reviewed write-ahead replacement-intent batching with per-rename parent flushes
retained. Synthetic cold median falls from 1206.136 to 694.028 ms; other measured
medians are higher, so no overall/native speed claim. Final scoped **307 Rust /
98 launcher Node** tests, Svelte/runes/build and source checks pass. Final review
accepts bounded source and lists remaining crash/failure injection gaps.
`sync-batch-optimization.md` records exact scope/evidence. Ten further checkpoints
are needed for 90%; OS/keystore/installer/Java/game, cross-store/shutdown/cache
authority, full security and exact-image/offsite gates remain. Disk is **1.8 GiB
free (APFS 100%)**, billing unknown. No release/deployment or manufactured credit.
The following recovery snapshot describes the predecessor source/evidence.

## Recovery continuation — October 13, 2026

**59/76 = 77.63%**, not 90% and **not release-ready**. Added reviewed staged-sync
restart journaling, verified no-op writes avoidance, isolated Unix cache reopening/
process recovery and typed network-only instance-manifest fallback. Production
frontend render/navigation/synthetic reconnection closes VEL-008 checkpoint 4.
Final scoped runs: 304 Rust / 98 launcher Node tests pass; build/source checks and
updated native Panel synthetic restore pass. Cold/repair synchronization costs
remain a major measured risk; no broad performance or native acceptance claim.
At least ten more checkpoints are required for 90%. Current-host fixtures do not
close supported-OS/keystore/installer/game, full security or exact-image/offsite
restore. Further implementation remains as well. Details: `recovery-progress.md`.
Disk is now about 5.6 GiB free; actual billed USD remains unknown. All edits are
uncommitted; no release/deployment or percentage-driven gate relaxation.

## Historical 58-checkpoint local sweep — October 13, 2026

**58/76 = 76.32%**, fixed engineering checkpoints; **not release-ready**.
Current changed-source suites pass 272 Rust and 169 Node tests; relevant frontend
checks/builds and final synthetic native Panel backup/restart/restore pass.
Java manifest bytes are bounded and verified, and online warm corruption is
repaired; separate independent review passes 16 contracts. This costs an extra
manifest fetch/deep verification, not a claimed performance improvement.
Evidence and exact remaining gates: `final-local-sweep.md`. Fresh origin fetch
reports zero HEAD divergence. Changes remain uncommitted; no deployment/release.
Durable recovery and shutdown still need implementation; supported-OS/game/
keystore/upgrades, final blanket security and exact-image/offsite restore gates
remain. Disk has about 2.0 GiB free, APFS reports 100%, and billed USD is unknown.
Reconcile disk and the $1,000 budget before expanding into another large batch.

## Post-milestone bounded work — October 13, 2026

Added an isolated real-sync preparation profile and a normal three-file fixture;
five explicit samples measure cold/warm/deep/repair behavior while checking
unmanaged-file preservation and cleanup. Updated core: 85 passed, zero failed,
two explicit timing benchmarks ignored by default. No runtime optimization or
Java/native-speed claim. `preparation-path-profile.md` documents results and next
gates. Ledger remains 57/76; no new checkpoint is awarded for partial profiling.

## Historical 75% progress — October 13, 2026

**57/76 = 75%**, meeting the requested engineering milestone. Fixed checkpoint definitions remain
unchanged. See `completion-checklist.md` and `progress-validation.md`.

Implemented: isolated bounded cache, lazy secondary routes, semantic components,
focus/Settings keyboard and motion fixtures, installation-state UX and allowlisted
diagnostics. Local suites pass 183 Rust / 98 frontend tests; Svelte has zero errors
or warnings. Initial static JS is 42.25% smaller; all JS grows 33320 raw bytes.

Final visual gate now passes with six DOM-matched, individually inspected synthetic
captures. Earlier stale PNGs remain excluded. A bounded reviewer retry completed
with 28 focused tests; final independent accessibility source/fixture review passes.
Do not infer readiness from the percentage. Durable recovery, native upgrades,
process quiescence and release/restore gates remain open. Fresh origin fetch has
zero HEAD divergence; changes remain uncommitted with no releases/deployments.
Actual dollar spend/remaining budget is unknown.

## Historical 50% milestone — October 12, 2026

Measured checklist: **38/76 = 50%**, with unchanged definitions and denominator.
This supersedes earlier snapshots below; it is engineering deliverable credit,
not effort weighting, release readiness or budget compliance. See
`completion-checklist.md` and `fifty-percent-validation.md`.

Local main remains at `4f639266a515c3cb05769b055a4772c4dcb89cb8` with
uncommitted changes. A fresh read-only origin fetch confirms HEAD and origin/main
have zero divergent commits. No branch, commit, PR, deployment, signing, paid
infrastructure or live-store migration occurred.

This batch extracts 124 native IPC handlers into 16 domain modules, introduces
bounded/redacted Panel transport and shared Discord/config contracts, makes
account/profile persistence failure-safe with exact credential rollback, and
separates frontend account/context/settings ownership with stale-response recovery.
Unused core dependency edges are removed; loader merge fixtures preserve runtime
and native-classifier behavior. Unknown-size downloads have bounded attempts,
restricted production redirects and tested cancellation partial cleanup.

Final local evidence: 164 Rust tests (70 host, 65 core, 10 utility, 19 shared),
54 frontend tests, zero Svelte errors/warnings, runes/build and Panel compilation.
Seven preexisting online tests remain ignored; no desired-gap contracts are ignored.
Main chunk is 814.45 kB / 224.60 kB gzip; no startup speedup is claimed.

Phase 1/release gates remain open: durable sync/cross-store journal and restart
recovery, directory durability, shutdown/process quiescence, cross-process and
handle-bound containment, active preparation recovery after reload, future-schema/
ACL/keystore continuity, installed upgrade/restore and supported-OS/native loader
acceptance. Legacy Value endpoint families are not comprehensively modeled.
Actual billed USD and remaining balance remain unknown.

## Historical 25% milestone — October 12, 2026

This snapshot supersedes the historical batch descriptions below. Local `main`
remains at `4f639266a515c3cb05769b055a4772c4dcb89cb8` plus uncommitted changes.
It was fast-forward synchronized with canonical upstream at the start of work;
no branch, commit, PR, deployment, signing or live-store migration was performed.

Operation UUID cancellation and correlated host/store/mock events are implemented;
the preparation domain and actual store have stale-event/admission/run regressions.
Root downloader full-response/declared-size/owned-temp hardening, staged verified
sync with in-process rollback, and atomic settings with save-before-memory-commit
are implemented and independently source-reviewed. Source CI now runs launcher
frontend tests/check/runes/build without release/deployment changes.

Measured checklist: **20/76 checkpoints = 26.32%**, satisfying the requested 25%
engineering milestone. Definitions/evidence are in `completion-checklist.md` and
`milestone-validation.md`. This is not percentage release readiness or equal effort.
Final validation: host 45 pass, core 57 pass / one known gap ignored, utility 6 pass;
seven existing online tests ignored. Frontend 19 pass, zero check errors/warnings,
runes and build pass. Existing bundle warning remains; main chunk is 811.30 kB
(223.63 kB gzip), not a measured startup speedup.

Gate 1 stays open: durable restart recovery, cancellation partial cleanup,
background/process quiescence, cross-process/native containment, reload identity
recovery, keystore/upgrade/OS acceptance and warm-sync profiling remain incomplete.
Actual billed USD and remaining balance are unknown. Next work should close these
correctness gates before broad extraction, performance or visual revitalization.

## Historical batches

- Baseline HEAD: `4f639266a515c3cb05769b055a4772c4dcb89cb8`.
- Current integration branch: local `main`, fast-forward synced with canonical `origin/main`; first-batch changes uncommitted. No worktree, branch or remote write.
- Phase/gate: Phase 1 source work continues after user authorization. Declared-download-size validation and instance-owned preparation/admission are implemented. Gate 1 remains open: full request/event correlation, transaction recovery, background I/O/process ownership and native acceptance are incomplete.
- Budget spent / remaining: unknown; provider dollar telemetry unavailable. Planning envelopes: audit $80, download-size $20, preparation ownership $35; these are not measured spend against the $1,000 maximum.
- PRs open / reviewed / merged: none created; no open canonical PRs/issues at initial inspection.
- Top reliability risks: same-instance stale cancellation and uncorrelated events; independent/background I/O and child-process quiescence; sync mutations before successful replacement; insufficient filesystem containment/download integrity. Native destructive-command acceptance remains pending.
- Metrics: production main JS 809.49 kB (223.09 kB gzip), existing chunk warning; synthetic browser observations recorded, no improvement/regression claimed.
- Current local tests: native host 27 pass / one stale-event contract ignored; core 43 pass / four known-gap contracts ignored; seven existing online tests ignored. Native total includes 13 direct production-manager/lease regressions and replaces eight old copied-fixture tests. Utility six-pass, Node 24 frontend/build and Infra results remain earlier-batch evidence; no frontend changed here. Source/secret/link checks are rerun per batch. Signed upgrades/game launch/OS matrix and exact-image restore acceptance remain pending.
- Human decisions: reconcile budget telemetry and approve typed operation/transaction contracts before their wider changes. No deployment approval requested. User continuation authorizes this bounded source fix, not release or implementation of every proposed contract.
- Next three actionable tasks: correlate public operation/run events and prove supervisor/native request ordering; establish staged sync commit/recovery and background/process quiescence; add bounded stale-response/UI ownership tests before frontend extraction.

Download-size-stage history: the initial batch added 19 passing tests and reproduced eight known-gap contracts; production remained unchanged during that batch. The download-size fix converted one contract into a passing regression and added six compatibility/mismatch tests, leaving seven known gaps at that stage. Root downloader now rejects wrong positive declared size before rename even with a valid SHA-1; size zero/omitted remains unspecified. Cleanup failure returns an error; cancellation cleanup and transactional sync are not fixed. The later preparation stage leaves five known-gap contracts, as recorded below.

All changes remain local and uncommitted on `main`; no branch, PR, merge, deployment, signing, dependency upgrade or persistent-data migration. The separately versioned `packages/packs-platform-utils` snapshot is unchanged, so this is not a fix for every Panel pack consumer. See `download-size-fix.md` and `download-size-review.md` for exact evidence. Initial synthetic preview servers remain stopped.

Final scoped review: approved by the independent reviewer after inspection of the exact production/test diff; no source-review blocker. Gzip/chunked fixtures, cleanup-failure fault injection, native overwrites and transaction acceptance remain explicit follow-ups, not claimed test results. Root boundary/identity/workflow/publication/secret/link and diff checks are repeated for this fix. No performance improvement is claimed.

See baseline, worker reports, regression summary and independent review for evidence and limits. Dollar usage remains unknown; reconcile provider billing before expanding the batch.

## Latest continuation: preparation ownership

Wrong-instance cancellation no longer aborts another owner or publishes its idle
state. Cancellation retains the busy registry through actual join and supervisor
finalization. Internal UUIDs protect completion identity; same-instance stale
cancel requests remain unresolved without a correlated IPC contract. Preparation,
delete, cache clear and username mutation share an in-process admission lease.
The legacy copied command fixture was replaced by direct production-manager tests,
not left passing against obsolete behavior. Five known-gap contracts now remain
ignored; native/background filesystem/subprocess and transaction gates are open.

Evidence: `preparation-implementation.md`, `preparation-regressions.md`, and
`preparation-review.md`. No production frontend/game changes, persistent migrations,
identity changes, dependencies, commits, PRs, merges or deployments.

Final verification: `CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher -p velora-launcher-core` exited 0 after the strengthened stale-finish test and scoped formatting. Results: launcher 27 passed / one ignored; core 43 passed / four ignored; seven existing online tests ignored. Log: `/tmp/velora-revitalization-preparation/tests-final.log`.
