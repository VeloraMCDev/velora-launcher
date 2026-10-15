# Independent bounded replacement-intent batch review

Date: 2026-10-13. Reviewed the current working-tree replacement-intent batching
change only in `crates/core/src/sync.rs`,
`crates/core/src/sync/transaction.rs`, and
`crates/core/src/sync/recovery_tests.rs`. The accumulated working-tree diff is
not the review scope. Read applicable root `AGENTS.md`, `MONOREPO.md`,
`docs/BOUNDARIES.md`, `docs/VALIDATION.md`, and `docs/DEPLOYMENT_TARGETS.md`;
consulted adjacent safety helpers only to understand the scoped calls.
No implementation edits, branches, commits, deployments, or live-store access.

## Findings

**No concrete batching regression or blocking defect identified in this source
snapshot.** This is bounded source-review and synthetic-test evidence, not an
assertion that every crash or external mutation is covered.

Non-blocking validation gaps:

1. **Actual reverse-rollback interruption is not directly injected.**
   `recovery_tests.rs:95` implements `batch-rollback` by applying only the first
   replacement and manually moving it back, leaving all journal records intact.
   This usefully checks replay of an already-restored record, but does not run
   `Publication::rollback` to a crash between its rename and persist, or after
   persisting removal of one of several applied records. Add a deterministic
   interruption after reversing the last of three applied replacements and
   separately after its journal pop is persisted; restart and verify complete
   original bytes/state, player data, and removal of new parents. A two-of-three
   applied prefix is also absent from the batch fixture matrix.
2. **The new execution recheck lacks deterministic mutation/failure coverage.**
   `transaction.rs:192` checks both destination occupancy and source fingerprint
   immediately before each rename. The batch tamper/occupancy fixtures instead
   mutate evidence after manual publication and exercise recovery, not this
   `move_paths` guard. Add an execution-boundary hook to change a later source or
   occupy a later destination after intent persistence/one successful rename;
   verify refusal, retention of foreign bytes, and safe recovery. Missing source
   and both-paths-missing pending records deserve explicit batch cases too.
3. **Batch journal-persist failure is not directly exercised.** Verify that a
   failure writing/replacing the combined journal occurs before any replacement
   rename, and that directory intents and prior backup moves remain recoverable.
   Existing interrupted temporary-journal coverage is useful but is not an
   injected failure of this new combined persist. These are coverage requests,
   not observed implementation failures.

## Source acceptance evidence

- `sync.rs:244` retains serial backup/quarantine/cache moves, then
  `sync.rs:266` batches only the staged replacements. State publication still
  follows successful completion of all replacement renames.
- `transaction.rs:151` collects source snapshots and checks destination absence
  before recording moves. `safety::unique` is applied to all source/destination
  paths in the batch before creating parents, rejecting duplicates, case aliases,
  and ancestor overlap. The updated overlap test at `recovery_tests.rs:203` uses
  two distinct staged sources, so its duplicate/nested/case-aliased destinations
  actually exercise destination collisions rather than duplicate-source rejection.
  A separate repeated-source/distinct-destination case also checks source alias
  rejection with unchanged journal bytes and no new parents.
- Parent creation retains its individual intent persist, mkdir, and parent
  directory fsync (`transaction.rs:128`). Only after these parents are prepared
  are all move records appended and persisted together (`transaction.rs:171`).
  No replacement rename occurs before that persist returns successfully.
- `transaction.rs:189` rechecks each planned source against its recorded snapshot
  and rejects an occupied destination before rename; both rename parents still
  receive fsync. Single moves delegate to the same helper at
  `transaction.rs:147`. Empty batches do not add a move-intent persist.
- Reverse rollback at `transaction.rs:202` handles each record independently:
  matching source plus absent destination is an unexecuted/already-reversed
  intent; absent source plus matching destination is reversed. Changed source,
  two occupied paths, missing destination evidence, or changed destination stops
  recovery rather than overwriting/deleting that evidence. Each successful pop
  is persisted before the next record. Independence within the batch allows
  reverse replay of pending suffixes followed by executed prefixes, then earlier
  backup restoration; recorded directories are removed only after the moves.
- The serialized journal remains version 1 with the existing ordered move
  records and snapshots, not a new batch marker or completion bitmap. Existing
  validation and commit detection remain in use. There is no new persisted path,
  credential, protocol, signing, or updater identity. Compatibility assessment
  is structural and current-recovery behavioral evidence, not an old-binary
  upgrade/downgrade test.
- Batch recovery fixtures cover all pending, first applied, all applied,
  already-reversed first move, committed publication, altered pending source,
  and occupied pending destination. Unknown pending evidence deliberately blocks
  earlier restoration and retains the workspace/backup for review. This may
  leave a partially published tree until manual resolution; it is not a promise
  that every refusal restores the old tree immediately. The final batch successor
  state lists all three published paths, with an explicit committed managed-list
  assertion, rather than retaining the old singleton fixture ownership list.

## Validation and reviewed hashes

After explicit authorization, ran the following focused suite on the first
reviewed snapshot, reusing existing artifacts without a full build or profile run:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher-core --lib sync::
```

Exit 0: **39 passed, 0 failed, 1 ignored, 0 measured, 94 filtered out**;
test execution 7.47 seconds. The ignored test is the explicit synthetic profile.
Nested child-fixture output is not an additional top-level pass count.
The first snapshot hashes matched before and after that run. Its recovery-test
SHA256 was `d79655cc6d8a641b92eced543aef5c31816763c809d131980cba60cf1e55c7aa`;
the two production-source hashes were the same as those below.

The lead then changed only the fixture ownership list/committed assertion and
added the repeated-source alias case. After reviewing these final fixture changes
and receiving separate authorization, ran exactly one narrower rerun:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher-core --lib sync::recovery_tests
```

Exit 0: **20 passed, 0 failed, 0 ignored, 0 measured, 114 filtered out**;
test execution 6.66 seconds, incremental compilation 3.48 seconds. The full
`sync::` suite was not rerun on the final fixture snapshot. Final reviewed SHA256
values matched immediately before and after the recovery-only rerun:

| Reviewed source | SHA256 |
| --- | --- |
| `crates/core/src/sync.rs` | `b612b75337687b98bd26d736917636361d6efc9a22e7963e713eec8b4a59ca37` |
| `crates/core/src/sync/transaction.rs` | `8f5cbcd4626be3f92f649562093a74c27fcb31d02bf511ff92e4066b31bd3bf8` |
| `crates/core/src/sync/recovery_tests.rs` | `3070b22110cddfa2dd291ddbeffcf0fd6347c8315862054e7ecd8aaa575bc8bb` |

## Acceptance boundary and limits

Acceptable as a bounded source change on the evidence above; the coverage gaps
remain open. This report does not grant master-plan checkpoint, native launcher,
supported-OS, power-loss, performance, or release acceptance. Child process exit
fixtures do not simulate storage power loss or every instruction boundary.

The pre-rename absence check followed by ordinary `std::fs::rename` is not an
atomic no-replace primitive against an uncooperative concurrent filesystem writer.
The source fingerprint check likewise is not an atomic binding of verified bytes
to the rename. These existing check/rename race limits are not established as new
batching defects; neither the source review nor passing tests proves protection
against all concurrent external changes or path-ancestor replacement.

No performance/profile test was run by this reviewer. The lead's separately
reported synthetic timings are not independently verified here and establish no
global improvement, native-performance acceptance, or billing claim. No paid
infrastructure or automatic deployment is introduced or authorized by this review.
