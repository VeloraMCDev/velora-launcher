# Release readiness

## Final measurement audit — October 14, 2026

**Not release-ready; 62/76 (81.58%) engineering checkpoints verified.** Separate
independent acceptance of VEL-014 checkpoint 3 covers comparable final measurements,
not native launch/loader no-regression. Final-repeat regressions are reported;
no general performance improvement is certified. Fourteen checkpoints remain,
including durability/cache security and native/keystore/OS/upgrade/restore gates.
See `install-final-measurements-review.md` and `90-percent-gates.md`.
No package, signing, release, store migration or deployment occurred.

## 80% engineering milestone — October 13, 2026

**Not release-ready; 61/76 (80.26%) engineering checkpoints verified.** The user
requested 80% and that milestone is reached through independently accepted
install/Java profiling and counted duplicate-resolution removal, not redefined
native gates. Actual JDK 21 fixture launch is not Minecraft/native-loader, signed
upgrade or full supported-OS acceptance. Wall-clock observations are mixed;
all library checks and final presence guard remain. Remaining durability/cache
authority/security/OS/upgrade/restore gates are mapped in `90-percent-gates.md`.
No package, signing, release, store migration or deployment occurred.

## Shutdown follow-up — October 13, 2026

**Still not release-ready; 59/76 (77.63%), not 90%.** Owned normal-exit draining,
transitive cache I/O ownership, admitted rename completion and updater partial
cleanup now have bounded reviewed source and synthetic tests. Locked scoped
Rust sweep: 256 passes; post-format host rerun: 92 passes. Native exit/window,
keystore, helper/process, installer handoff and supported-OS/upgrade/restore
acceptance remain required. Inline filesystem responsiveness, cross-store crash
durability and manifest-cache authority are still risks. See
`shutdown-implementation.md` and `shutdown-review.md`; no native/release gate is
credited from unit tests. No signing, execution of an installer, release or
deployment occurred.

## Cold-sync follow-up — October 13, 2026

**Still not release-ready; 59/76 (77.63%), not 90%.** Batched replacement-intent
optimization reduces descriptive cold-sync overhead without dropping directory
flushes. Supported-OS/native/power-loss/installed-store/release and broader
durability/authority gates remain. See `sync-batch-optimization.md` for all four
timing medians rather than inferring a global improvement from the cold result.

## Recovery continuation decision — October 13, 2026

**Not release-ready.** 59/76 (77.63%) engineering checkpoints are verified;
90% was not reached. Current-host reviewed sync/cache process recovery, typed
manifest fallback, rendered frontend fixtures and synthetic Panel restore pass.
These do not close cross-store durability/shutdown implementation, full cache/
manifest authority, supported-OS keystore/installers/Java/game/updater, final
whole-change security or exact-image/offsite rollback/restore. Cold/repair sync
has substantial measured synchronization overhead. See `recovery-progress.md`.
No packaging, signing, migration, release or deployment is authorized by progress.

## Historical 58-checkpoint decision — October 13, 2026

**Not release-ready.** 58/76 (76.32%) fixed engineering checkpoints are verified.
Final scoped source/build and synthetic native Panel restore checks pass; the
Java change has separate bounded integrity review. This does not establish
durable interrupted-sync/cross-store recovery, shutdown quiescence, supported-OS
keystore/installer/Java/game/updater acceptance, final blanket security review or
exact-image/offsite restore. Recovery remains implementation work. No release,
signing, packaging or deployment was performed. `final-local-sweep.md` records
the next gates; scarce disk and unknown billing must be resolved before expansion.

## Historical 75% decision — October 13, 2026

**Not release-ready.** 57/76 (75%) engineering checkpoints are verified.
Isolated cache is not wired into downloads and has no durable reopen/resume. Safe
diagnostics and typed UI retries have local fixtures, not native/E2E acceptance.
Final independent source/fixture accessibility review and bounded synthetic raster
snapshots pass. Broader design-system/native acceptance remains open, along with
durable sync/cross-store recovery, shutdown, OS/keystore/installer and restore
gates. No packaging, signing, release or deployment is authorized by progress.

## Historical 50% decision — October 12, 2026

**Not release-ready.** The 38/76 engineering milestone does not close Phase 1,
certify all endpoint families or authorize packaging, signing or deployment.
Evidence: `fifty-percent-validation.md` and the bounded implementation/review
pairs for extraction, Panel client, schemas, persistence, frontend domains, core
boundaries and downloads. Earlier decisions below remain historical.

Cancellation partial cleanup now passes local synthetic acceptance; it is no
longer an ignored desired invariant. Release blockers remain durable interrupted
sync/cross-store recovery, directory fsync/power loss, shutdown/background/child
process quiescence, cross-process/handle-bound containment, reload preparation
identity, future schemas/ACLs/keystore, installed upgrade/restore and supported-OS/
native loader matrices. No warm/slow-storage performance equivalence is established.
The existing all-error instance cache fallback is not authorization revocation.
No application/game, release or persistent-data migration was performed.

## Historical 25% decision — October 12, 2026

**Not release-ready.** The measured 26.32% engineering checklist milestone does
not close Phase 1 or authorize packaging/deployment. Correlated operations,
downloader hardening, staged sync rollback and atomic settings now have passing
local synthetic regressions and independent bounded source reviews. These replace
the earlier open source findings only within their implemented scope. Exact
evidence: `milestone-validation.md` and the four latest implementation/review pairs.

Release blockers still include durable interrupted-sync/restart recovery,
cancellation cleanup, native shutdown/background I/O/child-process quiescence,
cross-process/handle-bound path containment, supported OS filesystem/keystore and
installed-data upgrade/rollback tests. Active preparation recovery after UI reload,
settings schema/ACL continuity and warm-sync profiling are also open. No native
launch, signing, installer or live-store acceptance was run. All changes remain
local and uncommitted; dollar accounting remains unavailable.

## Historical gate decisions

**Not release-ready on this evidence.** Initial audit/regression/design is followed by one scoped declared-download-size fix, not a product release or complete revitalization.

- Gate 0: user approved source-work continuation on October 12; native gaps and dollar telemetry are still explicitly unresolved.
- Gate 1: independent review plus actual operation boundary tests and transactional interruption/data preservation acceptance pending.
- No implementation PR, merge, deployment, package publication, signing, schema migration or live backup/restore was performed.
- Source checks and local unit tests do not replace exact-image Docker acceptance, native desktop/game install and upgrade testing, server integration joins, offline recovery or offsite restore drills.
- Packaging/releases remain manual. Preserve account/credential/updater identities, operational SQL migrations, persistent paths/volumes and segment licenses/attribution.

Implementation evidence: root utility validates known sizes before publication;
the native host owns preparations by instance/internal UUID, retains cancellation
through join/finalization and shares mutation admission leases. Direct regressions
and independent source reviews pass. This is not full request correlation, I/O or
process quiescence, transactional sync, or native acceptance. Separate pack-utility
snapshot is unchanged. Next delivery: correlated IPC/event and staged sync/recovery
contracts, reviewed implementation patches, then native acceptance. No broad
architecture/UX/feature work before the relevant gates.
