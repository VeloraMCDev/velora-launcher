# Actual-method sync fault coverage

October 13, 2026. Bounded follow-up to the three nonblocking requests in
`sync-batch-review.md`. Changes are limited to test-only instrumentation in
`crates/core/src/sync/transaction.rs` and synthetic recovery fixtures in
`crates/core/src/sync/recovery_tests.rs`. No dependency, journal-format, persisted
path, protocol, credential, signing or updater identity change.

## Covered boundaries

- Actual `Publication::rollback` reverses the last of three applied replacement
  moves, flushes its parents and exits before popping/persisting that move record.
  A second case exits after the actual pop/persist. Restart completes rollback,
  restores prior managed bytes/state, preserves synthetic saves and removes new
  empty parents and staging. A two-of-three applied prefix is also covered.
- Actual `move_paths` persists the combined intent journal, applies the first
  replacement, then observes changed later staged bytes, an occupied later
  destination or a missing later source with absent destination. Its execution
  guard refuses the later rename. Restart does not silently repair or discard
  unknown evidence: previous backup/state and foreign bytes remain preserved,
  while the earlier replacement can remain applied until manual resolution.
- Actual combined journal persist encounters an existing exclusive `journal.tmp`
  after parent intents and prior backup publication. It fails before any
  replacement rename. The prior on-disk journal has only the original backup
  move; all three staged sources survive. Restart cleans interrupted temporary
  journal evidence, restores the original tree/state and removes empty new parents.

All hooks, the hook type/field and hook invocations use `cfg(test)`. They are
unavailable in the ordinary library build. Environment-selected behavior exists
only in the child-test fixture, not in production sync. Child processes and all
files use temporary synthetic roots; no installed/player/operator data is used.

## Evidence and limits

Final assertion refinements make the observed boundaries explicit: rollback
children leave four versus three durable move records before restart, exact
restored third staged bytes and still-published first/second replacements.
Execution refusal requires the specific planned-path guard error; combined
persist refusal requires the actual `AlreadyExists` I/O category. That case also
checks every staged payload and both new-parent directory intents. A second
refused recovery preserves the entire workspace fingerprint, not only selected
foreign files. These strengthen the fixtures without adding production behavior.

Lead focused suite: **40 passed, zero failed, one explicit timing profile ignored**;
log `/tmp/velora-revitalization-final/sync-fault-final.log`. Counts are top-level
tests, not additional credit for each child exit. Separate final independent
acceptance and reviewed hashes belong to `sync-fault-review.md`; the earlier
batch review's hashes refer to its predecessor snapshot.

Final stable full core library suite: **132 passed, zero failed, three explicit
timing profiles ignored**, log
`/tmp/velora-revitalization-final/sync-fault-core-final.log`. Independent final
recovery-only rerun: **21 passed, zero failed**, with matching final hashes.
The non-test core check also passes on unchanged transaction source. Earlier
host/shared/utility/Panel/frontend results remain prior unchanged-segment
evidence, not newly rerun totals for this test-only slice. Boundary/identity,
all 16 workflow policies, publication inventory, tracked/addition secret scans,
local links, scoped formatting and whitespace checks pass. No native binary,
restore, whole workspace or timing run is newly claimed.

These are deterministic interruption/guard/exclusive-temp-write fixtures, not
power-loss, disk-full, failed-fsync, journal-rename denial, every instruction
boundary, noncooperative filesystem race or supported-OS certification. They do
not establish shutdown quiescence, cross-store credential/account durability,
native game/keystore/installer acceptance or release readiness. Earlier timing
measurements are not rerun or reattributed to this test-instrumented snapshot.

The fixed ledger remains **59/76 = 77.63%**; 90% needs at least 69/76, ten more
verified checkpoints. Current disk measured 2.6 GiB free at the start of this
batch. Actual metered USD remains unavailable, not zero; original budget limits
remain unchanged. One independent reviewer is authorized for this bounded slice;
no open-ended implementation batch, purchase, release or deployment.
