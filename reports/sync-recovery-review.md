# Independent sync restart-recovery review

Date: 2026-10-13. Scope: working-tree `crates/core/src/sync.rs`,
`crates/core/src/sync/transaction.rs`, and `crates/core/src/sync/recovery_tests.rs`.
Read root AGENTS.md, MONOREPO.md, docs/BOUNDARIES.md, docs/VALIDATION.md, and
docs/DEPLOYMENT_TARGETS.md; inspected the adjacent sync safety helpers and fixtures.
This reviewer changes only this report. No production edits, dependencies,
commits, live stores, or deployments.

## Verdict

**One bounded restart-recovery blocker remains.** The independent scoped suite
passes, and the two earlier findings are resolved in the reviewed source, but
successful replay of interrupted recursive committed cleanup is not established.
This report supplies independent review evidence, not checkpoint or release
acceptance; do not credit complete restart-recovery coverage from the pass count.

## Remaining blocker: partially deleted directory backup cannot resume cleanup

- Locations: transaction.rs:238 (committed detection), transaction.rs:242
  (destination verification), transaction.rs:245 (missing-backup exception),
  transaction.rs:275 (cleanup), and transaction.rs:283 (recursive deletion).
- Legitimate sequence: a clean update journals and moves a multi-file
  `server-resource-packs` directory into `staged-sync/rollback/server-resource-packs`,
  publishes state, validates committed destinations, and begins recursive cleanup.
  A process exit after one child is removed but before the directory is removed
  leaves the committed state and journal intact, with a partial directory backup.
- On restart, the backup still exists, so the missing-backup exception does not
  apply. Its fingerprint differs from the complete recorded snapshot, and recovery
  returns `committed sync file changed; retain backup for review` before cleanup.
  Repeated recovery cannot progress, and ordinary sync admission is blocked by
  that retained workspace. This is an availability/idempotence failure, not a
  demonstrated destruction of player files; failing closed preserves the remainder.
- The existing `partial-cleanup` subprocess fixture removes the entire rollback
  directory. It exercises the absent-backup case, not this partial-subtree window.
- Required gate: add a committed directory-backup regression that removes only
  one child before restart. Persist cleanup authorization/progress before deletion,
  or otherwise make recursive deletion replayable without silently accepting
  externally changed backup bytes before cleanup is authorized. Continue validating
  committed state, published files, and retained quarantine entries.
- Evidence is source-level reasoning; this reviewer did not add or execute a new
  failure-injection fixture. The passing current suite does not refute this window.

## Resolved earlier findings

1. Rollback now rejects any occupied destination when the source already matches
   its recorded bytes (transaction.rs:162). The two-occupied-path regression
   retains both original and foreign backup bytes rather than permitting cleanup
   to discard the latter. Independently executed and passed.
2. Recovery quarantine validation now accepts direct `mods/*` paths matching the
   producer's existing regular-file enumeration, rather than only `.jar` names
   (transaction.rs:321). The non-JAR quarantine subprocess regression exercises
   precommit restoration and committed retention. Independently executed and passed.
   This intentionally preserves existing clean-update behavior, not a new promise
   that every unknown file in the mods directory remains in its original location.

## Bounded correctness observations

- Cooperative ownership uses a retained file handle and exclusive `try_lock`,
  acquired before recovery and held through sync and final cleanup. Unix
  multiple-link lock files and observed symlink lock/workspace paths are rejected.
  The same-process and subprocess ownership checks passed; this is not protection
  against a local actor replacing the lock pathname or concurrent hostile mutation.
- Forward directory/move intents are persisted before mutation. Journal writes
  use a create-new temporary file, file sync, rename, and Unix directory sync.
  Recovery discards interrupted temporary metadata after validating the retained
  journal. Reverse moves and directory removals precede removing their journal
  records, allowing replay after a process exit before the next journal update.
- State commit detection requires a recorded next-state fingerprint, absence of
  staged state, and matching published state. Precommit recovery checks original
  state and any recorded staged state. Changed original/staged state and changed
  published-file fixtures passed and retain evidence.
- `rollback_complete` is persisted after reverse operations and is checked with
  original state plus empty move/directory lists before cleanup. Finalization
  reuses recovery validation. Cleanup retains the journal until other workspace
  entries are removed; an empty workspace can be removed on the next attempt.
  These observations do not resolve the partial directory-backup blocker above.
- Existing `.scopenet/state.json`, revision/managed/clean_epoch representation,
  and `.scopenet/removed/<epoch>/mods/*` identities remain in use. Source and tests
  preserve synthetic saves/options/screenshots and unmanaged non-conflicting
  content; no installed-data or credential/updater identity migration was exercised.

## Independent validation and exact source identity

Lead explicitly declared the source stable before this reviewer executed:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher-core --lib sync::
```

Executed **once**, exit 0: **33 passed, 0 failed, 1 ignored, 94 filtered out**;
34 selected top-level tests. The recovery module declares 14 tests, including its
environment-controlled subprocess helper; child fixture invocations are not
additional independently counted top-level passes. The ignored test is the explicit
synthetic preparation timing profile. Existing artifacts were reused (Cargo reported
the test profile finished in 0.33s); suite duration was 6.95s. Available disk before
execution was approximately 5.6 GiB. No other Cargo command was run by this reviewer.

SHA-256 values matched the lead's stable-source declaration before execution:

| File | SHA-256 |
| --- | --- |
| crates/core/src/sync.rs | `8e3b67b2d6db468564b88d0e821603815bec788549c2befa013c6a23b3e5681a` |
| crates/core/src/sync/transaction.rs | `19a11c44d5e46ea10fd1bdbbf59bd4545c99c3cab4e522af1faa340d994a9338` |
| crates/core/src/sync/recovery_tests.rs | `9ba49fdc6b1bcd26ab5bc3d77448ff2bdcea0a36510ed4a857b797090afb6c18` |

## Limitations and remaining acceptance gates

Process fixtures use synthetic temporary files and explicit process exits; they
do not establish arbitrary interruption-point coverage, OS crash behavior, durable
power-loss recovery, or filesystem persistence guarantees. Directory syncing is
implemented only on Unix; no Windows/Linux acceptance, native launcher/game
acceptance, installed-store upgrade, or checkpoint acceptance is granted. Journals
are local owner-controlled metadata, not authenticated hostile inputs. The lock is
cooperative, and path checks are not an atomic defense against hostile races.
No comprehensive OS/filesystem matrix, release/signing checks, or live data tests
were performed. Review of this exact source is complete with the blocker retained;
any fix needs renewed source/fixture review before recovery completion is credited.

## Follow-up: authorized recursive cleanup (2026-10-13)

**Updated bounded source verdict: the remaining historical blocker is resolved;
no further correctness/safety blocker was identified within this review scope.**
The earlier findings, hashes, and test counts above are retained as historical
evidence, not the verdict for the revised source identified below. This follow-up
does not grant OS, power-loss, native, release, or checkpoint 4 acceptance.

### Fix inspection

- Journal `cleanup_started` defaults to false when absent (transaction.rs:29),
  preserving interpretation of existing version-1 journals. Production recovery
  validates committed state and every recorded destination before persisting
  cleanup authorization (transaction.rs:250, transaction.rs:271). Full backup
  equality is required before authorization; no deletion begins until the
  authorization journal write returns successfully.
- With authorization already persisted, only rollback-workspace backups may be
  absent or an unchanged subset of their original `(relative path, checksum/type)`
  snapshot (transaction.rs:258). A partial recursive deletion can therefore resume;
  added paths, changed bytes, and changed types fail the subset check. Live
  replacement and quarantine destinations still require exact fingerprints, and
  committed state must still match. Ownership and preserved path identities remain
  unchanged.
- Authorization with missing next-state evidence or completed rollback is rejected
  during journal validation (transaction.rs:341). Authorization outside the
  committed branch, including when staged state remains present, is rejected before
  rollback (transaction.rs:282). A mismatching current state also fails the committed
  comparison. These guards were inspected in source; the current fixtures do not
  separately exercise a forged precommit authorization or every invalid flag
  combination. Journals remain local owner-controlled, not hostile authenticated
  inputs. Snapshot-identical externally recreated content is indistinguishable
  from retained content; this is a content-level check, not inode provenance.
- The new subprocess fixture persists authorization before deleting one child of
  the backed-up resource-pack directory. Its unchanged remaining subtree resumes
  cleanup successfully. A companion case changes the remaining nested file and
  recovery rejects it while retaining those bytes. Both cases preserve committed
  revision, replacement bytes, and synthetic player save data. The existing
  entire-rollback-directory partial-cleanup fixture now persists authorization too.

### Independent follow-up validation

After explicit final-format stability confirmation, this reviewer executed the
same scoped command **once for this follow-up**:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher-core --lib sync::
```

Exit 0: **34 passed, 0 failed, 1 ignored, 94 filtered out**; 35 selected top-level
tests. Recovery now declares 15 tests including the subprocess helper. The recursive
cleanup test passed both unchanged-subtree and tampered-subtree cases. Child
invocations are not additional top-level passes. Existing artifacts were reused
(test profile finished in 0.29s); suite duration was 7.11s. Approximately 5.6 GiB
disk was available before execution. Across the initial stable review and this
separately authorized follow-up, this reviewer ran two scoped Cargo invocations;
the historical 33-pass result is not additional coverage of the revised source.

Revised SHA-256 values, captured before validation:

| File | SHA-256 |
| --- | --- |
| crates/core/src/sync.rs | `8e3b67b2d6db468564b88d0e821603815bec788549c2befa013c6a23b3e5681a` |
| crates/core/src/sync/transaction.rs | `a22350c32d4f0c84677827ed5555564c0abc30884c76ff5ef551ef686a040b5f` |
| crates/core/src/sync/recovery_tests.rs | `349cd8b4db53cd5e4f0957c6bb531b9e4d5b2fcbdb9130ad8c42be2592a980fb` |

Only this report was appended. All previously stated limitations remain: synthetic
process exits and cooperative locks, Unix-only directory fsync implementation,
no Windows/Linux acceptance, no arbitrary OS interruption or power-loss evidence,
no live stores, and no checkpoint 4 credit. Older unauthorised partial cleanup
cannot be retrospectively distinguished from external deletion; such ambiguous
evidence continues to require manual review rather than automatic deletion.

## Final optimization follow-up: read-only no-op (2026-10-13)

**Latest bounded verdict: one newly identified state-preservation blocker remains
in the declared final source.** The recursive-cleanup fix and its acceptance within
the bounded source scope remain intact. This section supersedes the previous
no-further-blocker verdict for the newly optimized sync.rs only.

### Exact guard and remaining blocker

The early return at sync.rs:177 follows ownership acquisition, existing-journal
recovery, state parsing, path/checksum/uniqueness checks, ownership collision checks,
and snapshot preflight. It requires an existing persisted state, equal revisions,
equal normalized managed sets, no triggered cleaning, and successful verification
of all requested files with `deep` honored. Ordinary unchanged cases therefore
avoid staging and state publication without bypassing pending recovery. Actual
publication still uses the unchanged transaction implementation. This is a
state/staging read-only optimization; lock acquisition and prior recovery may
still mutate metadata or recover an interrupted transaction.

However, `!cleaning` does **not** guarantee unchanged effective clean epoch:

- Persisted state: `revision = 0`, `managed = []`, `clean_epoch = 0`.
- Request: `revision = 0`, `files = []`, `clean_epoch = 5`.
- `cleaning` is false because it additionally requires `state.revision > 0`
  (sync.rs:128); every other early-return condition holds, including vacuous file
  verification. The optimized call succeeds without recording epoch 5.
- The pre-optimization publication path records
  `clean_epoch.max(state.clean_epoch)` (sync.rs:217), so this changes persisted
  state behavior for an input accepted by this API. There is no inspected guard
  establishing that an existing revision-zero state cannot be used here.
- Required fix: require unchanged effective epoch, for example
  `clean_epoch.max(state.clean_epoch) == state.clean_epoch`, or require the stated
  equal-epoch contract. Add a persisted revision-zero/epoch-advance regression.
  No demonstrated user-file loss is claimed; this is a bounded state-commit
  correctness edge case. It was reported before final stability and remains
  unchanged in the declared final hash. Evidence is source reasoning, not a new
  independently executed regression fixture.

### Fixture inspection and independent source validation

The new recovery_tests.rs:308 fixture checks ordinary and deep unchanged calls:
original state bytes and deliberately set mtime stay unchanged, staging is absent,
and the injected publisher panics if called. Its same-sized corruption case uses
deep verification and an unavailable origin; sync fails while retaining state,
tampered file bytes, and the synthetic save. It tests required repair admission
and failure preservation, not successful repair of that corruption. It does not
cover the revision-zero epoch advance above.

After the lead declared final source stable, the reviewer ran the scoped command
once for this optimization follow-up (no benchmark rerun):

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher-core --lib sync::
```

Exit 0: **35 passed, 0 failed, 1 ignored, 94 filtered out**; 36 selected top-level
tests. Recovery declares 16 tests including the subprocess helper. Existing
artifacts were reused (test profile finished in 0.43s); suite duration was 7.14s.
All existing cleanup and no-op fixtures passed. Total reviewer scoped Cargo runs
across the three separately authorized stable-source reviews: three. Earlier
counts remain historical and are not additive coverage of this final source.

Final source SHA-256 values, captured before execution:

| File | SHA-256 |
| --- | --- |
| crates/core/src/sync.rs | `b3c1085cb062a268c01f8debc5f318aa4c6d3b76e05b4eb5724b18c62ac6748c` |
| crates/core/src/sync/transaction.rs | `a22350c32d4f0c84677827ed5555564c0abc30884c76ff5ef551ef686a040b5f` |
| crates/core/src/sync/recovery_tests.rs | `89a8fb7d79636b91fce99b04c90ff6745c374c03459ba63dd23fcd5d7e7d8b18` |

### Profile attribution and retained limitations

The lead reports a passing explicit five-sample synthetic profile with median
milliseconds (cold/warm/deep/repair): historical pre-journal
229.703/14.409/16.365/20.579; journal before optimization
1092.460/55.962/57.692/100.577; final
1143.892/9.709/10.868/102.215. These numbers are lead-reported, not independently
rerun or independently audited benchmark evidence. They show the stated local
warm/deep improvement alongside roughly fivefold historical cold/repair cost;
they do not establish global or native performance gains. No performance
checkpoint, checkpoint 4, OS/power-loss, native, or release acceptance is awarded.
All earlier cooperative-lock, local-journal, platform, and synthetic-data
limitations remain. Only this report was appended; production source was not edited.

## Final accepted epoch fix (2026-10-13)

**Final bounded source verdict: all reported blockers are resolved; no additional
correctness/safety blocker was identified in this scope.** This verdict supersedes
the optimization section's historical unresolved epoch finding for the exact source
hashes below. Historical findings and validation results remain preserved.

The no-op guard now requires `clean_epoch <= state.clean_epoch` (sync.rs:177),
which is equivalent to an unchanged effective `max` epoch. An incoming epoch
advance cannot take the early return, even when persisted revision is zero.
Lower incoming epochs still preserve the already recorded epoch, matching the
existing publication semantics. Ownership, recovery, full preflight, matching
managed sets, and deep-aware file verification still precede the no-op return.
Transaction source is unchanged from the accepted recursive-cleanup follow-up.

The new `revision_zero_epoch_advance_is_persisted_without_cleaning_existing_files`
fixture (recovery_tests.rs:348) exercises persisted revision zero and epoch 2 to 3
with a matching managed file. It asserts recorded epoch 3, zero downloads and
cleaned entries, unchanged managed-file/save bytes, and no remaining staging.
No online origin is needed. This directly covers the reported guard edge case;
the unchanged/deep-corruption and recursive-cleanup fixtures also remain passing.

After the lead confirmed its focused result and source stability, this reviewer
executed the scoped command **once for this final epoch follow-up**:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher-core --lib sync::
```

Exit 0: **36 passed, 0 failed, 1 ignored, 94 filtered out**; 37 selected top-level
tests. Recovery declares 17 tests including the subprocess helper; child fixture
invocations are not extra top-level passes. The epoch regression passed. Existing
artifacts were reused (test profile finished in 0.34s); suite duration was 7.60s.
No other running Cargo job was interrupted. Total independent scoped Cargo runs
across the four separately authorized stable-source reviews: four, not additive
coverage of the final source. No benchmark or whole-suite rerun was performed.

Final SHA-256 values, captured before this validation:

| File | SHA-256 |
| --- | --- |
| crates/core/src/sync.rs | `654e7ec7b27e6230f0a8046039328492752dc4f64848e148c8b1733828dcf229` |
| crates/core/src/sync/transaction.rs | `a22350c32d4f0c84677827ed5555564c0abc30884c76ff5ef551ef686a040b5f` |
| crates/core/src/sync/recovery_tests.rs | `a7560a7e0b214ca7f7a6a36c7376bfcfb846da1fe9f449f4a7e0843e419cd723` |

Only this report was appended. All earlier limitations remain: cooperative locking,
owner-controlled journals, synthetic fixtures, and no native/Windows/Linux/OS
matrix, power-loss, installed-data, global performance, or release acceptance.
No performance checkpoint or checkpoint 4 credit is granted. Lead-reported profile
figures remain attributed to the earlier measured source; this final guard fix
was source-tested, not independently benchmarked.
