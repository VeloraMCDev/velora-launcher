# Independent staged-sync final source review

Date: October 12, 2026. Baseline HEAD: `4f639266a515c3cb05769b055a4772c4dcb89cb8`.

**Verdict: the three previous source/fixture blockers are resolved. VEL-004 checkpoints 1–3 are narrowly supported by the current implementation and regression assertions, subject to main's final Rust test results. Checkpoint 4 is not accepted.** This is an independent source review, not an independently observed green test run, native acceptance, or release approval. Do not mark the first three checkpoints execution-verified solely from this report.

## Scope and evidence

Reviewed this checkout from the repository root, following the supplied AGENTS.md instructions and reading MONOREPO.md, docs/BOUNDARIES.md, docs/VALIDATION.md and docs/DEPLOYMENT_TARGETS.md. Inspected the current sync implementation, safety helpers, publication/rollback helpers, both sync test modules, shared downloader and core HTTP regressions. Read the completion checklist and prior review/design evidence. Earlier test results do not certify these working-tree changes.

Only this report and `reports/downloader-response-review.md` are written. No production, test, snapshot, package or checklist edits; no duplicate Rust build/test run, native startup, live/world stores, external network fixtures, commit, PR or deployment. Main owns the next final Rust test run; its result was not supplied or observed during this review.

Reviewed SHA-256 identities:

| File | SHA-256 |
| --- | --- |
| crates/core/src/sync.rs | `699b7abca9f072636c31d31d9fdec5896258e286afec65c05a447f0e13a63d38` |
| crates/core/src/sync/safety.rs | `dd7743b633a6073947c64db0ac1fa60e5b39d881e1203a73b285ff880a631fd6` |
| crates/core/src/sync/transaction.rs | `ff88ce4bbc8e00690c36d07be3536b2cccae01855b2a137ca10f18d4ca81ef6d` |
| crates/core/src/sync/safety_tests.rs | `8a040c15b635bae9ffee6be75591f33f2d33ca76eadd0af029829e2dd85c31c0` |

## Previous blockers and checkpoint decisions

### Checkpoint 1 — All replacements verified before current-tree mutation

**Source/fixture acceptance: yes, narrowly.** `crates/core/src/sync.rs:96` normalizes manifest paths and requires SHA-1 to be exactly 40 bytes of ASCII hexadecimal before workspace creation. Empty, malformed and nonhex checksums cannot take the downloader's optional-digest path. Uppercase ASCII hex remains valid because verification is case-insensitive. Size zero remains unspecified; the required digest still verifies downloaded replacements.

`crates/core/src/sync.rs:174` stages all needed replacements in the claimed workspace, awaits the complete batch, deeply verifies each staged file, syncs its file contents and writes the candidate state before applying live moves. Batch/download/verification errors therefore precede managed removals, clean quarantine and replacement publication. Snapshot comparison checks that the observed live tree has not changed during staging.

Fixtures cover failed download preservation, a batch with one valid staged replacement and one failing download, failed clean staging, successful sorted ownership/idempotence, same-revision truncation repair and deep repair. `crates/core/src/sync/safety_tests.rs:6` closes the prior empty-digest blocker with invalid-checksum rejection and unchanged state/user-file/workspace assertions.

This is **not** unconditional integrity verification of every retained live file: the unchanged-revision, nondeep fast path intentionally checks existence and positive size, not hash. Same-size live corruption needs deep verification. Nor is this authenticated-manifest or adversarial SHA-1 security acceptance.

### Checkpoint 2 — Atomic state publication plus tested publication-error rollback

**Source/fixture acceptance: yes, for ordinary in-process errors.** The public `sync` arguments/result remain unchanged (`crates/core/src/sync.rs:67`). Its internal `sync_with_publication` hook injects only the final state publication operation; production supplies `std::fs::rename`. Earlier moves, deep verification and rollback use the real implementation, rather than a copied transaction fixture.

`crates/core/src/sync.rs:214` applies managed backups, clean-update quarantine/cache moves and staged replacements, then publishes state last. `crates/core/src/sync/safety_tests.rs:63` injects a final-state error after those live moves. Assertions require the exact previous state bytes, managed and dropped files, player mod and cache to return; the newly published config, quarantine epoch directory and staging workspace must disappear. This closes the prior absence of a publication-error rollback fixture; a failed download alone was not that evidence.

`crates/core/src/sync/transaction.rs:25` fingerprints a source immediately before its forward rename and records completed moves. Reverse execution checks that the inverse target is absent with symlink metadata and that the current inverse source matches the recorded content/tree fingerprint before renaming it back. It unwinds in reverse order and removes only tracked newly created empty directories.

`crates/core/src/sync/safety_tests.rs:83` occupies the reverse target and requires rollback error reporting, retention of original backup and foreign target bytes, unchanged persisted state and blocked retry. `crates/core/src/sync/safety_tests.rs:102` changes a published live file and requires its preservation plus retention of the original backup. On rollback failure, `crates/core/src/sync.rs:247` skips destructive workspace cleanup and returns manual-recovery context. A failed rollback may leave a mixed live tree; it is not a successful restoration and must not be launched as if sync succeeded.

The seam models rename failure before state replacement, as used by the production closure. It does not establish rollback for an arbitrary injected closure that replaces state and then reports an error. Not every intermediate move/parent-creation failure is separately injected, and successful-state-publication cleanup failure is reported as committed-but-cleanup-failed, not rolled back.

### Checkpoint 3 — User-file/path/collision/corruption safety fixtures

**Source/fixture acceptance: yes, within the represented ordinary filesystem cases.** `crates/core/src/sync/safety.rs:6` rejects traversal, rooted/drive paths, reserved state paths, unsafe Windows components and trailing dot/space names. Normalized slash aliases, duplicates, case-folded aliases and ancestor/descendant overlaps are rejected by `unique`; old ownership paths and the combined old/new set are validated too. `inspect` rejects symlinks and wrong entry types along instance-relative paths; fingerprints reject symlinks recursively.

`crates/core/src/sync/safety_tests.rs:6` exercises malformed checksums, traversal/root/drive/reserved/device paths, duplicates, case aliases, normalized separator aliases and overlapping paths. `crates/core/src/sync/safety_tests.rs:32` exercises unknown-user-file collision and corrupt persisted JSON without replacement/reset. `crates/core/src/sync/safety_tests.rs:48` supplies a Unix symlinked parent and asserts no outside-instance write. The rollback conflict tests additionally cover concurrent foreign bytes without deleting them.

Normal sync preserves unknown user files and refuses to overwrite a newly claimed user-file path. Explicit clean updates intentionally move unknown direct mod files into `.scopenet/removed/<epoch>/mods`; successful-clean tests preserve non-mod options/world/config fixtures and prove epoch idempotence. “User files are never touched” is not a literal description of this clean-update exception. Operational IDs, `.scopenet/state.json`, state schema and quarantine naming remain unchanged.

These are meaningful safety fixtures, not an exhaustive native path-corruption matrix. Unicode/filesystem equivalences, junctions/reparse points, all root/leaf/link combinations, parent replacement races and every malformed ownership-state variant remain outside the demonstrated test coverage.

## Explicit remaining gates — Checkpoint 4 stays open

- Publication records are in memory only: there is no persistent transaction journal, replay, durable commit marker or automatic interrupted-commit recovery. A leftover `.scopenet/staged-sync` blocks new sync and requires manual review; retaining evidence is not recovery.
- Cancellation while awaiting staging preserves the old live tree but leaves the claimed workspace/attempt temps. Current cancellation assertions prove old-state preservation, not cleanup or automatic retry. There are no awaits inside the synchronous live apply/rollback section, but process termination can interrupt it.
- Process kill, crash and power loss are not covered. Candidate files are synced, but directory rename ordering/durability is not established by a durable journal and parent-directory fsync protocol.
- Fingerprint/metadata checks and subsequent renames are distinct path-based operations, not handle-relative no-follow/no-replace transactions. Native TOCTOU, hostile ancestor replacement, hard-link identities and interprocess coordination remain gates; workspace claiming is not a complete cross-process lock protocol.
- Windows replacement semantics, open-handle failures, reparse points, filesystem aliases and the native restart/OS matrix need acceptance. Unix source fixtures do not establish Windows behavior.

## Handoff

Main should attach the final scoped Rust results for this reviewed source identity before crediting VEL-004 checkpoints 1–3. No new blocker was found for that bounded source/fixture scope. Preserve the fourth checkpoint and native/crash gates explicitly; do not describe this implementation as journaled, crash-safe, automatically recoverable or fully verified merely because ordinary rollback is now covered.
