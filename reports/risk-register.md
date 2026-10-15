# Risk register

## Design-review boundaries — October 15, 2026

`design-gate-followup.md` fixes observed normal-text contrast and Wallet motion
defects. Independent review additionally caught and prompted correction of a
light-branding/dark-HUD-preview regression. Repeated synthetic contrast matrices
still leave gradient/image/manual paint cases, input values/placeholders,
aria-hidden visible text, pseudo-elements, focus/border states, full key flows
and native/system-motion behavior unaccepted. Arithmetic/source checks and
zero failures among sampled DOM text groups must not be published as WCAG or
whole-VEL-015 acceptance. Fixed completion remains 62/76, not the requested 85%.

## Explicit Panel destination binding — October 15, 2026

The generic authenticated-call source gap described below is superseded by
`panel-context-implementation.md`: the unsafe ambient token helper is removed,
and prospective manifest fetch selects only a token matching its explicit
destination. Current request builders snapshot selected instance and token with
the Panel. Eleven direct regressions cover context/lookup, actual prospective
fetch, account requests and originless instance fetch. No observed external
credential disclosure is established; this is a source-confirmed binding defect.

Compatibility is intentionally tightened: unknown origins cannot authorize a
bearer or retain the old authenticated cache identity. Stores/IDs are unchanged,
but fresh scoped sign-in and refresh may be required. No inferred-origin or
ID-preserving legacy migration is certified. Installed upgrade/rollback remains
a release gate. Already-captured-token revocation, stale response publication,
post-response selection, map URL attribution, redirect/proxy behavior and whole
native/authentication coverage remain separate risks.

## Activity snapshot follow-up — October 15, 2026

The activity-specific mixed-snapshot risk described below has a production fix
and seven direct regressions: capture holds settings/accounts while selecting
the matching account's secret, using the actual request URL normalization for
credential scope. Missing legacy origin denies only the optional activity bearer;
it is not a migration or sign-out. See `telemetry-snapshot-implementation.md`.
This supersedes the activity-specific source gap, not every context race:
other `accounts::panel_token` callers still resolve destination and token
separately. Already captured tokens are not revoked on logout, HTTP redirect
behavior is not newly certified, unreadable-settings default-true behavior is
unchanged, and native/installed/OS/privacy acceptance remains separate.

## Optional activity preference — October 15, 2026

The native synthetic run reached activity-report failure paths despite
`send_stats: false`. This uncovered a real production preference-enforcement gap.
Capture/background/send now consult the shared current setting; retained
recorders observe later persisted opt-out without reattributing their identity.
The Settings switch discloses username/instance reporting and the distinction
from required server/gameplay traffic. Five host/loopback regressions and both
rendered switch states pass. A concurrent opt-out can still race after request
admission; already admitted/in-flight requests are not canceled or retracted.
No external delivery is established by the earlier route-only synthetic logs.
See `telemetry-consent-implementation.md` and the separate review. The corrected
binary has not been native-GUI replayed; whole privacy/release acceptance stays open.

The independent review also records a pre-existing identity-capture risk: Panel
URL, token and username are resolved through separate lock acquisitions, so
concurrent account/Panel changes are not a proven coherent snapshot. This fix
preserves sequential captured-event attribution but does not resolve that race.
Also, unreadable/malformed settings retain the existing default-true fallback;
the change is not fail-closed consent or an explicit opt-in migration.

## Returned-I/O and live-denial follow-up — October 14, 2026

Named synthetic transaction I/O errors now cover pre-marker rollback failure,
committed-marker synchronization and cleanup read/write fencing with real
encrypted restart fixtures. Static data-root symlinks fail before key access.
Separately, a process-local instance cache gate retains known denials after disk
invalidation failure and prevents older responses clearing a newer denial.
Both independent bounded reviews accept these slices; final host: 138 tests.
Actual OS/power-loss faults, durable offline authority after invalidation failure
and restart, credential epochs, already captured tokens, native keystore/upgrades
and complete release readiness remain risks. Ledger stays 62/76.

## Account restart protocol — October 14, 2026

The production account/credential pair now has prepared/committed recovery and
real encrypted interruption fixtures; unknown evidence is preserved rather than
overwritten. Native setup recovers before loading accounts, and pending journals
block live credential use/writes. See `account-transaction-implementation.md`.
No complete durability gate is awarded: malformed initial journal recovery needs
repair, a failed final directory sync retains evidence but is not durable-success
certification, and hostile ancestor races, remote-account rollback, Windows/OS
keyring/upgrade and power-loss acceptance remain. Independent review is separate.

## Final measurement audit — October 14, 2026

Independent acceptance of comparable final measurements brings the engineering
ledger to **62/76 (81.58%)**, not release readiness. Automatic cold and several
JDK-flow medians increase in the final repeat; sequential samples and uncontrolled
OS load/caches do not establish causality or wall-clock no-regression. The separate
measurement report and review retain all observations. Native launch/loader,
cross-store durability, cache authority/security and OS/upgrade/restore gates stay
open. No production change or new large build was needed for the artifact audit.

## Complete install/Java follow-up — October 13, 2026

Matched profiles cover the actual complete modern Vanilla installation body plus
real JDK detection and a harmless JVM, not actual Minecraft/native-library loads
or every loader/OS. Removing one duplicate metadata resolution retains both
discovery passes and 258 file checks; new actual-method fixtures prove second-pass
repair and final missing-library rejection. Child-loader paths can retain the
initial library metadata vector longer; no memory improvement is claimed. Timing
medians are mixed and uncontrolled OS caches prevent global speed claims. The
reviewed engineering ledger is now **61/76 (80.26%)**, not release readiness.
Cross-store durability, cache authority, native lifecycle and OS/upgrade/restore
gates remain. Disk about 1.8 GiB free (APFS 100%); no unrelated deletion/build tree.

## Shutdown follow-up — October 13, 2026

Owned-work admission/draining now covers preparation, blocking mutations, listed
background tasks, accepted account commits and updater downloads. Inline cache
and installer filesystem calls prevent detached Tokio writer lifetime but can
block async runtime workers. Stuck admitted work can delay exit indefinitely;
there is deliberately no unsafe forced-exit deadline. Native ExitRequested,
keystore, helpers/Java processes and supported-OS acceptance remain open.
Installer cleanup failure is best-effort, not an all-path removal guarantee.
Cross-store crash recovery, cache authority and release/restore gates remain.
About 2.1 GiB disk is free (APFS 99%); no new target tree was created. Actual billing
is unknown; the user removed the model cap, not infrastructure/release approval.

## Cold-sync follow-up — October 13, 2026

Batched independent replacement intents retain write-ahead journals and every
rename's parent flushes. Synthetic cold median improves to 694.028 ms from
1206.136 ms, still far above historical pre-journal 229.703 ms. New warm/deep/
repair medians are higher; no overall/native speed claim. Recovery/changed-evidence
fixtures and separate final review are recorded in `sync-batch-optimization.md`
and `sync-batch-review.md`. Only 1.8 GiB disk is now free (APFS 100%); actual
billing remains unknown. Fixed completion remains 59/76, not 90% or release-ready.

## Recovery continuation risks — October 13, 2026

Current-host staged-sync restart recovery now has a bounded journal/owner lock and
reviewed rollback/cleanup replay. Verified no-op work avoids state rewrites; actual
writes remain synchronized. Cold/repair synthetic medians are roughly five times
historical values, a significant pending performance risk. Cross-store durability,
shutdown and full OS/power-loss/failure matrices remain open. Isolated cache has
reviewed Unix-only reopen/process recovery, not HTTP resume/shared production
integration or hostile path-race safety. Online auth/status/config/decode/removal
no longer fall back to instance cache, but legacy cache lacks origin/account/lease
provenance and transport classification includes TLS failures. Native/release
gates remain despite frontend DOM and synthetic Panel restore acceptance. Current
59/76 is not the requested 90%; disk is 5.6 GiB free and actual spending unknown.

## Historical 58-checkpoint disposition — October 13, 2026

Java fetched-manifest verification and online warm corruption repair now pass
16 independently reviewed contracts and the final changed-core sweep. Warm
requires manifest availability and adds verification cost; index-failure fallback
still does not certify offline integrity. Artifact references/manifest shape,
symlink/path/permission/link handling, concurrent writes and whole-runtime durable
transactions remain open. Existing authenticated all-error cache fallback is not
auth-revocation acceptance. Broader durable sync/cross-store recovery and shutdown
still need implementation. Native/release/offsite restore gates remain despite
the synthetic Panel restore passing. About 2.0 GiB free (APFS 100%) and unknown
actual spend require disk/budget reconciliation before a large next batch.
58/76 is engineering credit, not safety or release certification.

## Historical 75% risks — October 13, 2026

Source fixes now cover A1 Home action contrast, A2 modal/inert shortcut isolation,
Settings roving tabs and system-motion casino exclusion. Final tests and separate
source/fixture re-review pass; six final matched rasters close the bounded snapshot
gate, not every-theme/native acceptance. Earlier stale captures remain excluded. Cache requires
exclusive new roots and is not downloader-integrated; persisted/interprocess/
hostile-root/durable recovery acceptance is absent. Deferred route imports can
fail persistently despite retry. Home action intent is view-local. Diagnostics
intentionally omits unknown signatures rather than promising arbitrary log redaction.
Initial static JS decreases while total JS increases. Dollar balance is unknown.
Earlier runtime/durability/identity/upgrade risks are not closed by 57/76 progress.

## Historical 50% risk disposition — October 12, 2026

Earlier risk rows are historical findings, not claims that their exact defects
still exist. Local source fixes now cover cancellation partial cleanup, bounded
unknown-size/redirect downloads, account/profile fail-closed persistence and
exact credential rollback, redacted typed Panel response handling, and frontend
stale/failed-selection reconciliation. Independent reviews plus final synthetic
suites support these bounded outcomes; see `fifty-percent-validation.md`.

Open high-risk gates: durable interrupted-sync/cross-store journal recovery,
directory fsync/power loss, native shutdown/background/child-process quiescence,
interprocess writers/handle-bound containment, reload preparation identity,
future schemas/ACLs/keystore and supported OS/installed upgrade/restore acceptance.
Synchronous download file operations require slow-storage/runtime profiling;
custom clients own intermediate redirect policy. Remaining Value endpoint families
and all-error cache fallback prevent universal API/auth-revocation claims.
Percentage credit does not close these gates or establish dollar-budget compliance.

## Historical 25% risk disposition — October 12, 2026

Earlier entries remain historical root-cause evidence. Same-instance stale cancel,
uncorrelated scoped events, root downloader response/temp/declared-size handling,
pre-verification sync mutation and settings save-before-memory divergence now have
bounded source fixes and passing synthetic regressions. Their broad risk areas
are **not closed**: cancellation partial cleanup, process/background quiescence,
reload identity recovery, durable sync journal/restart recovery, filesystem TOCTOU,
interprocess writers, native Windows/keystore/upgrade and settings future-schema/ACL
continuity remain open. Rollback deliberately retains backups when external changes
make reversal unsafe; interrupted workspaces block retries pending manual recovery.
See latest independent reviews and `milestone-validation.md`; no release acceptance
or warm-path performance equivalence follows from passing local tests.

| ID / priority | Evidence / reproduction | Owner / required acceptance | Rollback |
| --- | --- | --- | --- |
| R1 P0: remaining cancellation correlation / quiescence | Initial global `AppState.task` ownership gap is fixed; same-instance delayed cancel, uncorrelated events and independent/background writers remain. Preparation implementation report | Operation worker; real command and terminal-event tests, correlated cancel and process/I/O quiescence acceptance | Revert scoped job-manager changes; no persisted migration |
| R2 P0: pre-download sync mutations | `sync.rs` cleans/prunes before all replacement files succeed; direct revision-state writes. Controlled failure report | Core worker; synthetic snapshot invariants and commit/recovery interruption matrix | Do not touch real installs; future journal/backups need approved recovery semantics |
| R3 P0: download integrity / interrupted temp files | Shared HTTP primitive, predictable partial path and validation paths. HTTP regressions | Core worker + independent reviewer; correct size/digest validation, old destination preservation, bounded recovery | Revert narrowly; retain compatible hashes and existing destination identities |
| R4 P1: stale UI event/state | Global store and asynchronous feature responses; operation/frontend reports | Frontend + operation owners; identified jobs/runs and stale response rejection | Preserve IPC names/shapes until consumers migrated |
| R5 P1: credential/settings continuity | Existing keyring/encrypted store, rename/write behavior and corruption fallbacks | Security reviewer; native restore and failure tests before persistence changes | No key rotations, app ID changes or replacement of unreadable encrypted files |
| R6 P1: validation overclaim | Ignored online tests, browser mock timing, no signed/native upgrade run | Lead; explicit pending acceptance ledger | Block release rather than infer support |
| R7 P2: oversized modules / bundle | commands/pages sizes and >500 kB main chunk | Frontend/host owners; contracts and measured scoped extraction | Separate refactor/behavior commits in future reviewed PRs |
| R8 P0: filesystem containment / aliases | Independent review E4: lexical joining does not reject existing parent symlinks, reserved metadata paths or destination aliases | Core + security reviewer; preflight normalized manifest/persisted paths, native case/reparse tests, unique owned temporaries | No new writes/deletions until validated containment; no metadata rename |

Operation ownership also needs to govern delete/cache commands and child spawn/registration (independent review E6), not only preparation admission. A cancellation request is not proof of stopped writes or terminated children.

At the initial audit, severity was provisional and no production fix had been approved or deployed. See `independent-review.md` for reviewed ordering and additional risks; the scoped continuation below has its own final patch review.

October 12 continuation: the declared-positive-size publication gap in R3 is fixed in the root utility and its regression is enabled. Incorrect sizes preserve the previous destination and reject before rename; zero/omitted sizes retain their established meaning. This does not close R3's cancellation, retry-progress, temporary isolation or streaming-limit concerns, nor the separate pack-utility snapshot's analogous gap. Independent review of this scoped patch is recorded in `download-size-review.md`; no deployment occurred.

Latest continuation: R1's wrong-instance cancellation and early registry reopening
are fixed with actual manager/join tests and exact-ID finish. R1 remains open for
same-instance delayed cancel, public event correlation and independent/background
I/O/process quiescence. In-process mutation leases now guard preparation versus
delete/cache/username admission; close-after-spawn, interprocess/path aliases and
background writers remain unresolved. Terminal publication is lock-free while
the registry stays occupied, resolving the reentrant Rust-listener risk found
during review. Native mutation/event acceptance remains a gate.
