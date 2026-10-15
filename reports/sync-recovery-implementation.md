# Staged-sync restart recovery implementation

## Subsequent bounded optimization

`sync-batch-optimization.md` records the later independent replacement-intent
batch and final pre-batch/batched timing comparison. The original move/recovery
format, per-rename directory flushes and cleanup authorization remain; final
changed-source review is in `sync-batch-review.md`, not the earlier review below.

October 13, 2026. Base HEAD `4f639266a515c3cb05769b055a4772c4dcb89cb8` with
uncommitted working-tree changes. This closes an implementation gap in interrupted
sync on the current host, not the complete supported-OS recovery matrix or
cross-store account/settings journaling. Default protocol, managed state location,
revision/clean epoch semantics and quarantine paths are preserved.

## Implemented behavior

- A stable `.scopenet/sync.lock` file holds an exclusive advisory process lock
  through staging, publication, recovery and cleanup. Competing cooperating syncs
  fail before recovery/mutation; process exit releases the OS lock. The lock file
  remains in place to avoid unlink/recreate split ownership.
- `.scopenet/staged-sync/journal.json` records bounded versioned relative-path
  move intents and snapshots, created-parent intents, original/next-state
  fingerprints and rollback completion. Journal updates use exclusive temporary
  creation, file synchronization, rename and Unix directory synchronization
  before the associated live-tree mutation.
- A restarted sync recovers under ownership before reading state/snapshotting new
  work. Precommit replay restores moves in reverse; pending/already-reversed moves
  are recognized by exact source snapshots and absent destinations. Two occupied
  paths, changed artifacts/states, unsafe paths or unknown journals retain evidence
  for review rather than deleting user changes.
- Verified next-state publication with absent staged-state source identifies a
  committed transaction, including identical old/new state bytes. Committed live
  artifact/quarantine bytes are checked before deleting backups. Partial committed
  cleanup tolerates already-deleted backup entries while preserving live finals.
- Cleanup records completed rollback, removes other scratch only after validation,
  and removes the journal last. Empty final directories are safe to finish after
  interruption. Missing nonempty legacy journals are not auto-adopted.

No new dependencies, lockfiles, alternate default data directories or native
keystore/signing changes. Added files are only internal recovery metadata under
the existing instance metadata directory, not replacement state identities.

## Focused validation and review

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher-core --lib sync::
rustfmt --edition 2021 crates/core/src/sync/transaction.rs crates/core/src/sync/recovery_tests.rs
```

Final lead scoped run: **36 passed, zero failed, one explicit timing profile
ignored**. Actual child test processes exit at staged intent, original move,
replacement move, rollback and state-publication boundaries. Tests also cover
partial committed cleanup, non-JAR clean-update quarantine, interprocess
exclusion/release, original/staged state tampering, pending temporary journal,
two occupied paths, malformed/oversized/legacy journals and static symlinked
lock/workspace. Managed old/new bytes and synthetic saves are asserted exactly.
All earlier sync safety/publication/clean-update regressions pass.

The first run exposed a preflight regression: claiming metadata before inspecting
a symlinked destination created `.scopenet`. Lead moved destination inspection
before ownership acquisition; the unchanged preservation assertion now passes.
Independent preliminary review identified overly narrow JAR-only quarantine
validation and ambiguous occupied-source rollback; both were fixed with targeted
process regressions. A further review found partial recursive backup cleanup was
not replayable. Cleanup now persists authorization before deletion; restarted
cleanup accepts only an unchanged remaining subset of the authorized snapshot,
while changed/new backup bytes and live/quarantine changes still fail closed.
Subprocess tests remove one directory child before restart and separately tamper
with the remaining child. The separate follow-up accepts this bounded fix in
`sync-recovery-review.md`; supported-OS/power-loss acceptance stays open.

Final review also caught a no-op epoch edge: persisted revision-zero state does
not require cleaning, but still must publish an advancing clean epoch. The guard
now explicitly requires no effective epoch advance, and a regression checks
revision 0/epoch 2→3 persistence with zero cleaning/downloads and preserved bytes.

## Verified no-op optimization and measured tradeoff

A comparable five-sample synthetic profile exposed journal overhead even when
state/files were unchanged. After ownership, recovery and normal safety preflight,
sync now returns read-only only when persisted state exists, revision/clean epoch
require no change, ownership sets match and every file passes the requested
size/digest policy (including deep verification). No staging or state rewrite is
needed for that no-op; every actual write still uses the journal. A new regression
checks unchanged state bytes/mtime, no staging/publisher callback, and repair
admission on same-size deep corruption.

| Median ms | Historical pre-journal | Journal before no-op optimization | Final no-op optimization |
| --- | ---: | ---: | ---: |
| Cold logical instance | 229.703 | 1092.460 | 1143.892 |
| Unchanged warm | 14.409 | 55.962 | 9.709 |
| Deep verification | 16.365 | 57.692 | 10.868 |
| One-file repair | 20.579 | 100.577 | 102.215 |

Same fixture: 64×16,384-byte files, 1 MiB payload, concurrency four, loopback
HTTP, debug build, five samples; OS cache and system load remain uncontrolled.
Warm/deep avoid redundant writes, but synchronized cold/repair remain roughly
five times the historical synthetic medians. This substantial cost is a remaining
performance risk, not a general improvement or native/runtime claim. Profile
invariants and the explicit cache timing fixture pass; VEL-014 remains open.

## Explicit limits

Only Darwin arm64 process-interruption behavior is exercised, not supported-OS
or power-loss acceptance. Directory synchronization is implemented on Unix;
non-Unix directory durability remains unverified and is not silently credited.
Locks coordinate cooperating callers, not same-user hostile path replacement;
path helpers do not provide race-free no-follow traversal. Journal files are
local caller-owned metadata, not authenticated hostile inputs. External mutation
or unknown legacy staging can still require manual review. Device/fsync/rename
failure injection and full OS permission matrix remain gates. Cross-store
account/profile transaction recovery and process/I/O shutdown quiescence remain
separate implementation work. VEL-004 checkpoint 4 stays open until the complete
restart/OS matrix and independent scope acceptance are satisfied.
