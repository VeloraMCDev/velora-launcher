# Independent bounded sync fault review

Date: 2026-10-13. Scope: the new test-only publication hooks in
`crates/core/src/sync/transaction.rs` and actual-method fault fixtures in
`crates/core/src/sync/recovery_tests.rs`, compared with the three coverage gaps
in `reports/sync-batch-review.md`. Read root `AGENTS.md`, `MONOREPO.md`,
`docs/BOUNDARIES.md`, `docs/VALIDATION.md`, and `docs/DEPLOYMENT_TARGETS.md`;
no additional scoped `AGENTS.md` files were found under crates or reports.
Adjacent sync module gating, recovery, persistence, and safety helpers were
consulted to interpret the scoped calls, not to review the accumulated changes.
Only this new report is authored by this review; no production edits, commits,
branches, worktrees, deployments, or live-store access.

## Disposition

**Acceptable as a bounded source/test change. No blocking defect identified in
the reviewed hooks or new cases.** The previous report's three specific coverage
requests are materially addressed through actual publication methods, with the
assertion and fault-boundary limits below. This is not master-plan completion or
release acceptance.

The lead supplied **sync:: 40 passed, 0 failed, 1 ignored**. That broader result
is lead-reported, not independently rerun or certified here. This reviewer ran
exactly one focused recovery-suite command and one non-test library check.

## Coverage against the preceding gaps

1. **Actual rollback interruption:** `recovery_tests.rs:106` installs the hook,
   successfully calls `Publication::move_paths` for all three replacements, and
   calls actual `Publication::rollback`. The hook exits with code 77 on staged
   source `2`, the first reversed replacement. At `transaction.rs:233` the
   reverse rename and both directory flushes have completed, but neither the
   in-memory pop nor its persist has occurred. At `transaction.rs:245` the pop
   and persist have completed. Restart therefore exercises respectively an
   already-reversed record still on disk and a shortened persisted journal.
   The explicit panic if rollback returns prevents a missing exit hook from
   silently passing. The parent verifies exact original state bytes, original
   managed bytes, synthetic player bytes, removal of staging and new parents,
   and a second successful recovery. `batch-two` separately adds a manually
   applied two-of-three prefix; it is not an actual-method execution hook.

2. **Actual execution recheck:** the hook at `transaction.rs:201` runs immediately
   before the execution guard. On staged source `1` it changes that source,
   occupies its destination, or removes its source while its destination is
   still absent. These cases call actual `move_paths`, after the first actual
   replacement rename succeeds. Child assertions establish first replacement
   bytes and an unexecuted third replacement. The parent observes recovery
   refusal, unchanged original state, retained first replacement and original
   backup bytes, preserved player bytes, and exact foreign bytes or absence of
   both second paths. These are fail-preservation cases, not successful rollback
   of untrusted or missing evidence.

3. **Actual combined persist failure:** `before_plan_persist` creates a regular
   synthetic `journal.tmp` after replacement records have been appended in
   memory and destination parents have been prepared. Actual `persist` fails
   its exclusive `create_new` open at `transaction.rs:111`; the hook itself
   returns success. The child verifies all three staged sources exist, all
   replacement destinations remain absent, the durable journal has only the
   earlier backup move, and that backup retains exact original bytes. The
   parent then successfully recovers that prior backup and directory intents,
   removes new parents/staging, and preserves state/player bytes. This tests
   temporary-file creation failure, not failure of the final journal rename.

## Production exclusion and synthetic isolation

- `PublicationHook` (`transaction.rs:12`), the `Publication::hook` field
  (`transaction.rs:73`), all four hook call sites, the rollback path clones used
  only by the hook, and the explicit recovery initializer are individually
  `#[cfg(test)]`. `sync.rs:18` gates the entire recovery-test module. The hook is
  not a serialized journal field; persisted version and record schema are
  unchanged by this instrumentation. In ordinary non-test compilation the hook
  type, field, calls, and test helper are excluded by those source gates.
- The successful non-test `cargo check --lib` corroborates that the ordinary
  library compiles without relying on the gated field/type. It is not a release
  binary inspection, packaging test, or cross-target compilation matrix.
- Parent fixtures use `tempfile::tempdir()` and synthetic state, replacement,
  backup, foreign, and player-owned bytes. Only the child command receives the
  synthetic root/point environment variables; there is no parent-process
  environment mutation or global hook registry. Each hook belongs to its own
  publication, and restarted recovery initializes the hook to `None`.
- The test helper accepts a root from its environment; it is not a sandbox
  enforcing temporary-root provenance. In this controlled run the spawning
  parents supply temporary roots. Child process exit bypasses child destructors,
  but parent-owned temporary directories clean up the fixture roots on drop.
  No real credentials, player stores, operator addresses, or native services
  were used or written by these new cases.

## Remaining non-blocking assertion and coverage limits

1. Execution and persistence failures assert `is_err()`, not the error category
   or message. In particular, both-paths-missing would also fail at `rename` if
   the fingerprint guard were absent. Its current assertions do not alone prove
   guard rejection before the syscall. Source inspection locates that rejection;
   a specific error assertion would strengthen regression sensitivity.
2. Rollback fixtures select the correct boundary in source and verify complete
   restart results, but do not explicitly assert the intermediate on-disk move
   count (four before reverse persist versus three after it), restored third
   staged bytes, or the other two still-published replacements before recovery.
   Checking those milestones would make the two interruption windows more
   independently observable. The two-of-three prefix remains manually applied.
3. The persistence fixture checks source existence and a one-record journal,
   not exact bytes for every staged source, unchanged durable journal bytes, or
   explicit directory-intent contents. Successful removal of new parents is
   useful indirect evidence. No write, file-sync, journal-replacement rename,
   directory-sync, or in-process rollback-after-persist-error fault is injected;
   this case exits and exercises restarted recovery instead.
4. Refused-recovery cases check selected retained bytes, but do not snapshot the
   whole workspace or retry refusal to prove idempotent preservation of every
   record/file. Cases do not explore mutations during rollback or after the
   execution recheck. Existing check/fingerprint followed by ordinary rename
   still has a concurrent-writer race; the hooks do not supply atomic no-replace
   or atomic verified-byte binding.

These limits do not negate the observed coverage improvements and are not
reported as production defects introduced by the hooks.

## Independent local validation

Initial disk availability was 2.6 GiB. Existing host `target/` artifacts were
reused; no alternate target/platform, clean build, or additional Cargo test run
was requested. Cargo commands were serial, with the lead holding concurrent
Cargo work. The check rechecked host dependency metadata; it was not wholly
cached. Disk was 2.5 GiB available after validation.

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher-core --lib sync::recovery_tests
```

Exit 0: **21 passed, 0 failed, 0 ignored, 0 measured, 114 filtered out**.
Cargo reported test-profile readiness in 0.45 seconds and test execution in
6.70 seconds. Child fixtures also ran via the same test executable and their
required exit code 77 was checked by their parents.

```sh
CARGO_PROFILE_DEV_DEBUG=0 cargo check --locked -j1 -p velora-launcher-core --lib
```

Exit 0: non-test host library check completed in 16.09 seconds. These timings
describe this invocation only and are not performance comparisons.

Reviewed SHA256 values matched before and after both commands:

| Reviewed file | SHA256 |
| --- | --- |
| `crates/core/src/sync/transaction.rs` | `ba140d6288709bd0edf08f5bc1fad38d3a42c2a3fadeb7b6124e1b39638e92b3` |
| `crates/core/src/sync/recovery_tests.rs` | `fd0806c9f8996d91aeb82496bca896f6dc04a326420ab25ccb819dc5cea97865` |
| `reports/sync-batch-review.md` | `fbf63b618a9ca46805add441b6427de0629e447055eaf3059169f8cf490e11ce` |

## Acceptance boundary

Acceptance is limited to this source snapshot, local non-test compilation, and
the focused synthetic recovery run. Process exit occurs after the real reverse
rename and directory flushes, not during either operation. It does not model
storage power loss, arbitrary instruction boundaries, or all external writers.
No native launcher, supported-OS matrix, installed-store upgrade, full workspace,
release packaging/signing, performance-target/90% improvement, or billing
acceptance is claimed. No paid infrastructure or automatic deployment is
introduced or authorized.

## Final fixture assertion acceptance — 2026-10-13

After the lead's stable-patch notification, reviewed the strengthened assertions
in `recovery_tests.rs` and ran exactly one additional recovery-only command.
This addendum supersedes the earlier assertion weaknesses only to the extent
specified here; the original snapshot, results, and broader limits remain
historical evidence. No production file was edited by this reviewer.

**Accept the final fixtures as bounded synthetic source-test evidence; no
blocking assertion defect identified.** The transaction SHA256 is identical to
the original reviewed snapshot, so the prior source-gating review and non-test
check remain applicable. No new hook or production behavior was introduced by
this fixture follow-up; no whole-suite or second non-test check was run.

- `recovery_tests.rs:131` now requires the exact planned-path guard error for
  all three execution mutations. In particular, the missing-source case cannot
  pass merely because a later rename returns an I/O error. The persist case at
  line 116 downcasts to `std::io::Error` and requires `AlreadyExists`, consistent
  with the actual exclusive open of the injected `journal.tmp`.
- The persist-failure fixture checks exact bytes for all three staged payloads,
  absence of every replacement destination, the one durable backup move, exact
  original backup bytes, and presence of both `new` and `new/nested` directory
  intents in the durable journal. These directory checks require membership;
  they do not assert the complete directory vector, its order, or absence of
  duplicates/extra intents. Durable journal byte equality is still not asserted.
- Before restarted rollback, the parent checks four on-disk move records before
  reverse persist and three after persist, exact restored third staged bytes,
  absence of the third published destination, and exact still-published first
  and second payloads. This directly observes the two previously inferred
  interruption milestones before the existing full-restoration assertions.
- For the three execution-mutation cases, the parent takes a whole-workspace
  fingerprint after the first refused recovery, then requires another refusal
  and the same fingerprint after the second. This strengthens repeated-refusal
  preservation for these cases. It is not a pre-first-recovery comparison or an
  extension to every other refused-recovery fixture. The fingerprint covers
  recursive paths, directory entries, and file-content hashes, not timestamps,
  permissions, inode identities, or other filesystem metadata.

Validation command, exit 0:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher-core --lib sync::recovery_tests
```

**21 passed, 0 failed, 0 ignored, 0 measured, 114 filtered out**. Cargo reported
4.05 seconds for test-profile compilation/readiness and 6.73 seconds for test
execution. This is the sole Cargo invocation for this follow-up; the lead held
concurrent Cargo work. It reused the existing host target and temporary synthetic
fixtures, without accessing live stores or building additional platforms.

Final source SHA256 values matched before and after the rerun:

| Final reviewed source | SHA256 |
| --- | --- |
| `crates/core/src/sync/transaction.rs` (unchanged) | `ba140d6288709bd0edf08f5bc1fad38d3a42c2a3fadeb7b6124e1b39638e92b3` |
| `crates/core/src/sync/recovery_tests.rs` | `d1d2c03e0e3acfbd090a4ddf2d1e3de7c32cd480aaa28b8ea019799d1f1bd097` |

Broader limits remain: the two-of-three prefix is manual; persistence injection
is an exclusive temporary-file open failure, not a write/sync/final-rename fault;
in-process rollback after persist failure is not tested; process exits occur
after directory flushes rather than modeling power loss; concurrent-writer
check/rename races and arbitrary rollback mutations are not resolved. The
lead-reported broader sync result is not recertified on this final fixture hash.
No native, supported-OS, installed-store, packaging/release, performance-target,
90% improvement, billing, or master-plan completion acceptance is added.
