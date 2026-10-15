# Performance after

## Batched replacement intents — October 13, 2026

Latest same-fixture five-sample medians: cold 694.028 ms versus the final
pre-batch epoch-fixed 1206.136 ms; warm 12.082 versus 9.738, deep 12.430 versus
9.523, repair 107.871 versus 99.949 ms. Only cold improves in this descriptive
run. All per-rename parent flushes remain. Historical pre-journal cold/repair
229.703/20.579 ms are still much lower. No physical-cold/native/JVM/game,
significance or overall improvement claim. Exact samples, scope and crash tests:
`sync-batch-optimization.md`. Earlier figures below remain historical snapshots.

## Recovery/no-op measurements — October 13, 2026

New synchronization has a real cost in the same synthetic sync fixture. Final
cold/repair medians are 1143.892/102.215 ms versus historical 229.703/20.579 ms.
These roughly fivefold costs remain a risk, not a claimed optimization. A verified
no-op guard removes redundant journal/state writes for unchanged warm/deep work:
medians 55.962/57.692 ms before that guard become 9.709/10.868 ms in the next
five-sample run. State bytes/mtime remain unchanged and deep corruption still
requires repair. No controlled physical cold cache, significance, JVM/game/native
or overall speedup claim. Exact comparison/scope: `sync-recovery-implementation.md`.
The cache's explicit five-sample fixture also passes with its new synchronization;
it still restarts interrupted populations from byte zero, not HTTP resume.

## Final Java integrity tradeoff — October 13, 2026

Matched five-sample synthetic runtime fixture: cold median 1.319 ms; warm median
0.568 ms versus historical marker-only 0.271 ms. Warm adds a verified manifest
request and declared-artifact deep checking; the observed 0.297 ms extra work is
not hidden or claimed as faster native preparation. Marker timestamps remain
unchanged on verified warm ensures. No JVM/game/spawn timing or physical cold
cache/significance is established. `java-runtime-profile.md` retains raw samples,
historical scope and current request counts. Final core passes 101 ordinary tests;
three timing profiles remain explicit-only. VEL-014 stays 0/4.

## Post-milestone profiling — October 13, 2026

No runtime optimization is introduced. `preparation-path-profile.md` measures
current synthetic sync paths: medians 229.703 ms cold logical instance, 14.409 ms
warm, 16.365 ms deep and 20.579 ms one-file repair. These are different flows, not
before/after gains. Java, native startup and representative install metrics remain
open. Updated core tests pass 85 cases; both timing benchmarks are explicit-only.

## Historical 75% chunk measurements — October 13, 2026

Comparable Node 24 accounting: initial static JS **814453 → 470361 raw bytes**
(42.25% reduction), gzip **222833 → 125795** (43.55% reduction). Total JS grows
**881396 → 914716 raw / 245651 → 268970 gzip**. Entry-only reduction must not be
mistaken for full boot payload. Details/toolchain/hashes are in
`frontend-bundle-implementation.md`; no native startup/install speedup is claimed.
Controlled cache benchmark is an isolated artificial-reader fixture, not a
production download/install measurement. Runtime and VEL-014 gates stay open.

## Historical 50% measurements — October 12, 2026

Final Node 24 build: main JS **814.45 kB / 224.60 kB gzip**. Compared with initial
809.49 / 223.09, this is **+4.96 kB raw / +1.51 kB gzip**, not a startup improvement.
The existing >500 kB chunk warning remains. No native startup/install/launch,
loader or warm-sync speedup/no-regression claim is supported by these measurements.
Download writes/rename/cleanup are synchronous to make cancellation ownership
explicit; slow-storage runtime latency requires profiling. Staged verification
hashing likewise requires warm-path measurements. Earlier results below are history.

## Historical 25% measurements

No native startup/install/launch speedup is measured or claimed. The October 12
final event-controller build reports a main chunk of 811.30 kB (223.63 kB gzip), versus
the earlier 809.49 kB (223.09 kB gzip): +1.81 kB raw / +0.54 kB gzip. This is bundle
accounting from local Node 24 builds, not a startup benchmark. The existing >500 kB
chunk warning remains. Staged sync adds snapshot/verification hashing that needs
warm-path profiling before any no-regression claim. Repeat controlled native
baselines on the same environment; browser fixtures are not native measurements.
