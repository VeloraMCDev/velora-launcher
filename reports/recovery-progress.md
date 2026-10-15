# Recovery follow-up and 90% gate audit

## Final measurement audit — October 14, 2026

The separate independent final measurement audit accepts VEL-014 checkpoint 3:
**62/76 (81.58%)**. This is a distinct comparable-measurements deliverable, not
another award for the optimization or a recovery/native acceptance claim.
Fourteen checkpoints remain; seven additional verified outcomes are needed for
at least 90%. See `install-final-measurements-review.md` and `90-percent-gates.md`.
Earlier totals below remain historical snapshots. No production code changed.

## Current 80% milestone — October 13, 2026

Independent full-install/Java review accepts VEL-014 checkpoints 1–2, bringing
the fixed ledger to **61/76 (80.26%)**. Earlier shutdown/recovery totals below are
historical snapshots, not the latest aggregate. Matched install/automatic-runtime
and real JDK fixture profiles now accompany a counted duplicate-resolution
optimization that retains all library checks. Stable scoped Rust tests pass 260;
independent four-test/profile reruns pass separately, not additive test credit.
See `install-java-profile.md`, `install-profile-review.md` and the updated ledger.
No native-game/OS/keystore/release/restore gate is inferred; eight further verified
checkpoints would still be required to reach 90%.

## Shutdown implementation follow-up

The user explicitly removed the model cap as a continuation gate. Owned-work
admission/draining and preparation shutdown now have actual host integration,
not only a design proposal. Blocking mutation workers retain their resource/work
ownership even if the awaiting wrapper is cancelled. Transitive cache filesystem
work is inline, accepted rename commits retain their existing lease, and owned
installer partials are closed/removed on cancellation. Lead host check, 92 host
library tests and four preserved IPC-surface tests pass; separate final review
is in `shutdown-review.md`. Full normal-exit/native/helper/process-tree acceptance
is not inferred. See `shutdown-implementation.md`; fixed credit remains 59/76.

## Actual-method fault follow-up

`sync-fault-coverage.md` records new deterministic tests at actual rollback
reverse/pop/persist, move execution rechecks and exclusive combined-journal
write failure. Test hooks are `cfg(test)` only. Lead focused sync suite passes
40 tests with one explicit timing profile ignored. Separate final review belongs
to `sync-fault-review.md`; prior source hashes are historical. Current-host
fault coverage does not close the supported-OS/native gates: **59/76 (77.63%)**
still requires ten more checkpoints for 90%. Disk measured 2.6 GiB free at the
start of this slice; metered billing remains unknown.

Final stable full core library suite passes **132 tests**, zero failures, three
explicit timing profiles ignored. Final independent recovery rerun passes 21;
the non-test library check passes. Prior unaffected-segment counts remain prior
evidence, not a fresh aggregate sweep. `90-percent-gates.md` maps all 17 open
checkpoints and the ten needed to reach the unchanged target.

## Subsequent cold-sync follow-up

Independent replacement intents now share one durable write-ahead journal
publication; every rename still flushes both parents. Pending/partial/committed
batch recovery and changed-evidence preservation have new process fixtures.
Synthetic cold median falls from 1206.136 to 694.028 ms, while warm/deep/repair
are higher in the new uncontrolled run. `sync-batch-optimization.md` records
exact samples and limits; `sync-batch-review.md` owns changed-source review.
No additional checkpoint is granted: **59/76 (77.63%)**, ten short of 90%.
Disk now measures only **1.8 GiB free (APFS 100%)**, not the earlier 5.6 GiB;
no unrelated files were removed. Provider billing remains unknown. The earlier
snapshot's review hashes and counts refer to its predecessor source.

Latest stable checks pass **307 Rust** (75 host, 131 core, 10 utility, 19 shared,
72 Panel) and **98 launcher Node** tests; Svelte zero errors/warnings, runes,
production build, boundary/identity/workflow/publication/secrets/links/scoped
format/whitespace checks pass. Separate batch review accepts final source with
20 focused recovery passes and explicitly retains fault-injection gaps.
No new native Panel restore/OS/installed-store validation is implied.

October 13, 2026. Base HEAD `4f639266a515c3cb05769b055a4772c4dcb89cb8`,
uncommitted working-tree source. Fresh origin fetch: zero commits on either side
of `HEAD...origin/main`. No branch, commit, PR, release, migration or deployment.

## Delivered in this continuation

1. Staged-sync process ownership, bounded write-ahead move/parent journal,
   interrupted precommit rollback, authorized replayable committed cleanup and
   fail-preserve handling for changed/unknown evidence. Independent review caught
   four gaps; all are fixed with regressions. Verified read-only no-op
   sync avoids redundant journal/state writes without bypassing deep repair.
2. Explicit persistent reopening of the isolated Unix cache, stable owner lock,
   verified/canonical bounded inventory and process-kill recovery. Lead took over
   after the worker's expired-auth failure, corrected two fixture errors and added
   more integrity tests. Separate final review passes the bounded source suite;
   no downloader/HTTP resume or broad platform/security acceptance is implied.
3. Instance-manifest fallback restricted to typed transport failures. Online
   auth/status/config/decode/removal failures do not consult installed cache.
   Existing file format/path and successful online writes remain unchanged; the
   network fallback does not rewrite cache. Separate review accepts the narrow
   policy, not legacy offline authority or cross-panel/account provenance.
4. Production frontend Home/Settings/Stats/Market render/navigation and synthetic
   disconnect/reconnect observed in the shared browser with a test-only IPC
   facade. The facade is absent from the production build. Snapshot/click/frame
   automation failures are disclosed; accepted evidence is state plus actual DOM
   handler/render observations, not new screenshots or native interaction.

Evidence: `sync-recovery-implementation.md`, `sync-recovery-review.md`,
`content-cache-recovery.md`, `content-cache-recovery-review.md`,
`manifest-cache-policy.md`, `manifest-cache-policy-review.md` and
`frontend-domain-browser.md`. Review source hashes match final working-tree files.

## Current validation

| Run | Passing evidence |
| --- | --- |
| Locked launcher/core/utility/shared tests | 75 + 128 + 10 + 19 = 232; doc tests pass |
| Separate locked Panel library tests | 72; combined scoped Rust total **304** |
| Launcher Node/Svelte/runes/build | **98** tests; zero Svelte errors/warnings; runes/build pass |
| Independent focused reviews | Sync 36, cache 29, manifest policy 5 passes; reruns are not added to primary totals |
| Explicit timing fixtures | Updated cache and sync five-sample profiles pass; synthetic only |
| Final native Panel build/restore | Build passes; four synthetic backup/restart/restore groups, four backed-up files; **2026-10-13T13:26:39.365Z** |

Rust command:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher -p velora-launcher-core -p velora-platform-utils@0.5.0 -p velora-shared@0.5.0
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-panel --lib
```

Three core timing fixtures stay ignored by default; seven preexisting online
tests remain ignored, not acceptance. The final Panel binary retains its two
preexisting dead-code warnings. The previous Operations/Docs/Infra/signing/mobile
71 Node tests and Panel web checks remain earlier unchanged-segment evidence,
not newly rerun counts for this continuation. Source checks are rerun after
report reconciliation. No test or review count is completion credit by itself.

### Performance cost is not hidden

In the same synthetic 64-file profile, journal-before-no-op warm/deep medians
55.962/57.692 ms become 9.709/10.868 ms after avoiding verified no-op writes.
Final synchronized cold/repair medians 1143.892/102.215 ms remain roughly five
times the historical 229.703/20.579 ms. This is a significant pending cold/repair
optimization risk. OS caches/load are uncontrolled; no native/global speedup,
significance, Java spawn, actual game or throughput claim is established.

## Fixed ledger and requested target

Only VEL-008 checkpoint 4 is newly awarded for the defined frontend rendering/
navigation/reconnection outcome: **59/76 = 77.63%**. At least **69/76 = 90.79%**
is needed to satisfy a 90% target under the unchanged equal-weight ledger.
Thus **ten additional verified checkpoints are still required**. The current
continuation does **not** claim that 90% was achieved or that the plan is complete.

VEL-004 checkpoint 4 remains open: current-host process exits are not the full
restart/OS matrix. VEL-012 checkpoint 4 remains open: bounded Unix recovery review
is not complete recovery/integrity/security/platform acceptance. VEL-014 remains
0/4: these synthetic profiles and no-op changes do not replace comparable native
install/Java/final-loader acceptance. Final independent whole-change security,
supported-OS upgrades and exact-image/offsite restore are not closed by scoped
reviews or one synthetic Panel restore. **Not release-ready.**

Remaining work includes actual implementation, not only externally run tests:
cross-store durable account/profile transactions, shutdown/background I/O/process
quiescence, full cache recovery/failure matrix and integration, manifest-cache
origin/account authority and cold/repair performance. Supported-OS keystore/
installer/game/loader, native UI/feature/startup and signed updater acceptance,
final independent security/data/compatibility sweep and release rollback/offsite
restore still need approved fixtures/platforms and operator decisions. Completing
source code alone cannot honestly manufacture those checkpoints.

## Resource and authority gates

Free disk now measures about **5.6 GiB** (APFS 98%); no unrelated user files were
deleted to expand capacity. Actual provider spending/remaining balance are still
unavailable. The original $1,000 hard cap and $500/$750/$900 alert/approval rules
remain; renewed continuation is not proof that billing is under those thresholds.
Reconcile actual billing before another open-ended/high-risk batch, and arrange
the missing native platforms/approved synthetic installation fixtures. No paid
infrastructure, automatic deployment, real credential/item mutation or live
operator/player store was introduced. Further unbounded work is not justified
merely to force the percentage to a requested value.
