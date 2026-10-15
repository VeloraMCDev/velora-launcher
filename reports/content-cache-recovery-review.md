# Independent bounded content-cache recovery review

Date: October 13, 2026, America/New_York. Review completed locally around
08:55 EDT. Baseline HEAD: `4f639266a515c3cb05769b055a4772c4dcb89cb8`.
This is a review of the dirty working-tree snapshot, not certification of HEAD
or reliance on Harvey's missing final report. **Blocked: the requested narrow
suite fails. No checklist credit or production/security acceptance is assigned.**

## Ownership and scope

Only this report is authored by this reviewer. No production code, tests, original
reports, lockfiles or sync files are edited. Lead handles sync separately. No
commit, branch, worktree, deployment or paid infrastructure is created. Root
`AGENTS.md`, `MONOREPO.md`, `docs/BOUNDARIES.md`, `docs/VALIDATION.md` and
`docs/DEPLOYMENT_TARGETS.md` were read; no nested instructions were found under
core or reports. The supplied review authorization is the scope authority; the
master-plan attachment referenced by the original report was not found in this
checkout and is not independently certified here.

Primary review: `crates/core/src/content_cache.rs`,
`crates/core/src/content_cache/recovery.rs`, and
`crates/core/src/content_cache/tests/recovery_tests.rs`. Supporting review:
the original contract/implementation reports, existing cache tests, and the
reused SHA-256/SHA-1 file helpers. The original implementation report describes
an earlier disposable prototype; it is not evidence for these new recovery files.

## Concrete blockers

1. **Failing canonical-inventory assertion** —
   `crates/core/src/content_cache/tests/recovery_tests.rs:65` appends `0` to a
   complete canonical filename whose size is `3`, creating a canonical filename
   with size `30`. `Key::from_filename` is a syntax parser, not a file-content
   verifier, and correctly accepts this different identity. The assertion fails
   at line 66. This is not evidence that noncanonical keys are accepted. The
   owner should use an actual noncanonical size alias (for example a leading
   zero), or exercise reopen with a canonical size-30 filename and a size-3 file
   to test the separate integrity check. Do not weaken the parser to reject a
   legitimate size-30 identity.
2. **Failing hostile-substitution error-category assertion** —
   `crates/core/src/content_cache/tests/recovery_tests.rs:104` writes a 24-byte
   owner-marker sentinel for every hostile object, including a committed key
   that declares three bytes. For the hardlinked final-file case,
   `recovery.rs:148` rejects the length as `Integrity` before the subsequent
   link-count check at line 164 can return `UnsafePath`; line 114 insists on
   `UnsafePath`. Source analysis explains a fail-closed error-precedence mismatch,
   not accepted tampered content. Use a correctly sized sentinel to isolate the
   hardlink check, or separately assert rejection and preserved bytes. Because
   this assertion aborts the test, its later root-replacement assertions at
   lines 118–123 are not executed in this run.
3. **Stale contract/evidence handoff** — the original contract says the root is
   never reopened, no interprocess lock or fsync exists, and Windows remains an
   execution gate. Current code adds reopen, locking and synchronization and
   deliberately rejects non-Unix platforms. Original implementation counts
   (19 passing tests) and its no-process-kill statement do not describe this
   snapshot. The code owner/lead must reconcile those documents before treating
   them as the current contract. This reviewer does not edit their owned files.

The first two failures block a green validation handoff. Neither is, by itself,
proof of an unsafe successful reopen. Remaining failure-injection and platform
gates below also prevent broader recovery acceptance.

## Behavior confirmed within the bounded prototype

| Concern | Source and observed evidence | Qualification |
| --- | --- | --- |
| Persistent reopen and exclusivity | `recovery.rs:56` reconstructs an explicitly named existing root. `open_owned` validates the fixed owner marker, acquires nonblocking `File::try_lock`, and retains the file handle for the cache lifetime. Live same-process reopen returns `Collision`; the killed-child fixture also verifies interprocess exclusion and release after kill. Reopen preserves the marker inode and survives repeated reopen. | Advisory coordination for cooperating cache objects, not protection against an external writer replacing the marker. No shared simultaneous cache access. |
| Bounded reconstructed capacity | Recovery validates limits; canonical committed filenames supply exact sizes; actual lengths must match. Checked addition rebuilds bytes and entry count, including empty entries. A single unlinked scratch counts separately; a linked scratch is the same inode and counts once. Per-object, aggregate, entry and retrieval-headroom fixtures pass. | Payload bounds, not a physical filesystem quota. Directory/owner metadata and block overhead are excluded; no eviction or reference tracking. |
| Canonical identities | `recovery.rs:6` bounds filename length, parses numeric namespace IDs, exact algorithm tags, digest and size, then requires round-trip equality with `filename()`. Uppercase digests and numeric aliases cannot survive round-trip canonicalization. | Key syntax is not trusted provenance. The failing size-30 assertion is a test error. The reopen fixture uses one namespace; preexisting tests cover namespace separation, not a multi-namespace reopen fixture. |
| SHA-256/SHA-1 integrity | Recovery hashes every committed file before deleting scratch. Stage/get verify exact length and digest. Both algorithms and exact-empty objects survive reopen; same-size and oversized SHA-256 corruption reject reopen while preserving final and scratch bytes. Existing warm-tamper and known SHA-256-vector tests pass. | SHA-1 is compatibility-only, not collision-resistant authentication. No dedicated SHA-1-corrupt-reopen fixture. SHA-256 helper reads a whole bounded object; SHA-1 streams. Path-reopening helpers are not race-free handle verification. |
| Interrupted scratch/link recovery | At most one canonical nonzero `staging-N` is accepted. Only an inode-linked committed file may have two links; other entries require one. Final integrity is checked before scratch removal. Sequence is recovered from surviving scratch; successful recovery removes scratch and synchronizes the directory. | Unlinked partial or fully verified scratch is discarded, never resumed or promoted. Linked verified final is retained. More than one scratch, unknown names or extra hardlinks fail closed. Sequence is not a durable monotonic journal when no scratch survives. |
| Publication ordering | `content_cache.rs:249` stages and synchronizes the payload before hashing; publication hardlinks without clobber, updates memory, syncs the root, unlinks scratch, then syncs again (`:319`). Owner marker and new-root parent also receive synchronization. | An error after final-link creation can still leave a committed final. No transactional all-or-nothing success receipt; sync syscall ordering is not power-loss proof. |
| Tamper and failure preservation | Static symlink/nonregular objects, owner/root inode substitution, malformed owner marker, unknown inventory and unexpected link counts reject operations. Recovery does not delete committed entries. Pre-validation integrity failures preserve scratch; existing reader-error/cancellation/collision fixtures preserve finals. | Same-user hostile races are outside the caller-owned/exclusive-writer assumption. The failing hostile fixture limits its own coverage; other original tamper tests pass. |
| Unix-only behavior | Both `create` and `open_owned` return `UnsupportedPlatform` off Unix. Unix inode/link checks and restrictive creation modes are explicit. | Tests import Unix APIs without non-Unix gating; Windows test compilation/runtime support is not established. No native Windows or Linux acceptance. |

## Exact local validation

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher-core --lib content_cache::
```

Run once against the existing `target` directory; no target override, cleaning,
large benchmark fixture or broad build. Host: Darwin 24.6.0 arm64; rustc
`1.98.1 (48a229cea 2026-09-01)`; cargo
`1.98.1 (797e8a9bc 2026-08-05)`. Incremental test profile finished in 0.46 s;
test execution took 0.69 s. Exit **101**. Result: **25 passed, 2 failed,
1 ignored, 94 filtered out**, 28 selected tests. The ignored test is
`controlled_local_benchmark`; it was not run. The selected standalone
`process_worker` test returns immediately without its environment variables;
its pass alone is not subprocess evidence.

Failing tests:

- `content_cache::tests::recovery_tests::recovery_rejects_unbounded_ambiguous_and_noncanonical_inventory`
  — panic at `recovery_tests.rs:66`.
- `content_cache::tests::recovery_tests::hostile_owner_root_and_scratch_substitutions_fail_closed`
  — panic at `recovery_tests.rs:114`.

The passing `killed_process_releases_ownership_and_recovers_interrupted_boundaries`
test launches and kills a child at six synthetic boundaries: owned, partial,
verified, linked-unsynced, linked and published. Partial and published use actual
population paths; verified/link boundaries manually compose internal staging and
link operations. Previously committed `retained` bytes remain readable, linked
finals survive, unlinked scratch is discarded and subsequent reopen works. This
is local process-kill/reopen evidence on the same running OS, not power-loss,
filesystem remount, disk-full, permission-denial or full application evidence.

## Remaining gates and limits

Failed recovery before cleanup preserves inventory; a blanket claim that **every
failed reopen preserves every file** would be incorrect. At `recovery.rs:191`,
scratch can already be removed when directory synchronization fails; the final
sync at line 97 can also fail after cleanup. No new fault-injection fixture tests
remove/sync/owner-creation failures. Committed files are not intentionally deleted,
but error receipts do not imply no filesystem changes. The original best-effort
RAII cleanup and live-object fail-closed behavior after leftover scratch remain.

Static metadata checks and advisory locking do not establish hostile path-race
resistance. Ownership checks verify inode/length, not marker bytes on each live
operation; trusted exclusive writers remain required. Accepted in-memory inventory
is reconstructed from filenames, not an authenticated manifest or journal. Root
creation interrupted before a complete marker is not adopted/repaired. The owner
should add targeted failure injection and complete repaired tests before a new
independent acceptance decision.

No HTTP range/validator/resume, downloader integration, native Windows/Linux,
power-loss durability, real install/upgrade, production performance, full security
acceptance or VEL-012/master-plan checklist completion is claimed. No benchmark
numbers from the old report are rerun or endorsed here.

## Snapshot SHA-256 provenance

Primary source hashes were captured during review and rechecked unchanged after
the test and source review. Supporting hashes pin the exact context read, not ownership of
those files. Future source changes require a new review; HEAD alone is inadequate
because cache files are untracked and other agents' changes are present.

| Path | SHA-256 |
| --- | --- |
| `crates/core/src/content_cache.rs` | `c3357b7cb6218370fe88be0f65de95c0399b8f6dc0896827a335abdcf66d65ca` |
| `crates/core/src/content_cache/recovery.rs` | `9157f57c5b9cb831df2784e8d4d66f97f81b839e8abaf13783472b34be559e97` |
| `crates/core/src/content_cache/tests/recovery_tests.rs` | `a27c9a27ffc55b7d0aa830bf92561bcdd59b6f97024456fb606d3cac8eefc4f1` |
| `crates/core/src/content_cache/tests.rs` | `0850b876a823ffc65a9e4c26c79b4ecb14f193ed2eec3a842dac1fee4426037d` |
| `crates/platform-utils/src/authlib.rs` | `6560819f3c67941f4194b73c889277f606dbf9c0ac91f300683369dd41a2863d` |
| `crates/platform-utils/src/http.rs` | `406cca7cbffa9005ee41c55b9942afe4aa5d42adccccd26406064a250300e253` |
| `crates/core/src/lib.rs` | `40c400a8e9f0e0cdeee8d8871312759ea9d4394dcfb13e3ba05c525e7bd73aae` |
| `crates/core/Cargo.toml` | `20284cd750f187bd7eaa862de1dde37736fb371bb45ee73f76cbf878911aa002` |
| `Cargo.lock` | `a47e070cd2fcbf55cdf21339d11a57c8f7ac65a3f07ee5ea433dfa1fa688a8ab` |
| `reports/content-cache-contract.md` | `0442c28060d4c6745dfb8c5b8bb00948e39574a28b49ea5c2f627070b748552b` |
| `reports/content-cache-implementation.md` | `061b9dcf0924477677ce8c1fc6f2c04febea59a0fe37ddbbbc7ba8e18692b3e9` |

## Final follow-up review — October 13, 2026, 09:05 EDT

This append-only follow-up supersedes the initial failed-suite verdict for the
snapshot below; the initial run, findings and hashes remain historical evidence.
Only this report is changed by this reviewer. Root instructions were rechecked;
the same working-tree HEAD and Darwin/Rust/Cargo environment apply. Current cache
source, recovery code and repaired/new recovery tests were reviewed. Original
base tests and recovery implementation retain their initial hashes.

**Verdict: the two fixture blockers are resolved; the bounded local source/test
review is clear of those blockers, and the required narrow suite is green. No
additional production-code blocker was identified within this review's scope.**
This is not production cache, full security or checklist acceptance. The original
contract/implementation reconciliation and missing worker handoff remain pending
with the lead; their files still had their original hashes during this follow-up.
Those handoff/documentation gates are not silently closed by a green test run.

### Repairs and additional evidence

- `recovery_tests.rs:65` now tests a genuine leading-zero size alias, rather than
  rejecting a different canonical size. The new
  `canonical_size_is_parsed_but_reopen_rejects_mismatched_bytes` test (`:143`)
  confirms canonical size `30` parses successfully, then rejects three on-disk
  bytes with `Integrity` and preserves them. Both tests pass without weakening
  the production parser or integrity check.
- The hostile fixture (`recovery_tests.rs:105`) uses owner-marker bytes for the
  owner and three-byte payloads for scratch/final names. All six symlink/hardlink
  substitutions now reach their intended rejection checks and preserve outside
  sentinel bytes. The formerly unreachable root-replacement assertions also
  execute and pass; no weakening to a generic error assertion was necessary.
- `sha1_compatibility_corruption_is_rejected_before_scratch_cleanup` (`:156`)
  publishes a SHA-1 compatibility entry, corrupts it to same-size `bad`, and
  verifies reopen returns `Integrity` while preserving both corrupt final and
  unlinked scratch bytes. This closes the specific missing SHA-1-corrupt-reopen
  fixture noted in the initial table, not SHA-1 cryptographic-security limits.
- `content_cache.rs:337` gates the existing cache tests with
  `cfg(all(test, unix))`; `:340` adds a separate non-Unix unsupported-platform
  test. Source inspection confirms valid-limit `create`/`reopen` return
  `UnsupportedPlatform` before creating/adopting the root. The test asserts no
  root exists after `create`, and checks the `reopen` error. This conditional
  test is **not compiled or executed by the Darwin run**, and no cross-target
  or native Windows build/run is claimed.
- Repeated reopen, reconstructed capacity, SHA-256 corruption preservation and
  the six-boundary killed-child scratch/linked-publication fixture still pass.
  The previously recorded exclusive-writer, advisory-lock, path-race,
  sync-error-after-cleanup and nontransactional error-receipt limits remain.

### One requested rerun

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher-core --lib content_cache::
```

Run exactly once for this follow-up using the existing target. Exit **0**;
**29 passed, 0 failed, 1 ignored, 0 measured, 98 filtered out**; 30 tests selected.
Incremental profile finish: **0.45 s**; test execution: **0.76 s**. The benchmark
remains ignored. The 29 passing registrations comprise 19 original tests plus
10 recovery tests, including the standalone `process_worker` helper registration;
the helper's environment-free pass is not an additional fault scenario. Actual
child-kill evidence comes from the dedicated killed-process test. No large
fixtures, broad build, sync checks or native target checks were run.

### Follow-up SHA-256 snapshot

These four cache source/test hashes were captured before the rerun and rechecked
unchanged after it. Supporting helper, core export, package manifest and lockfile
hashes were rechecked against the initial table and are unchanged. The two
original report hashes were also unchanged; later reconciliation is outside this
snapshot and must not be represented as reviewed here.

| Path | SHA-256 |
| --- | --- |
| `crates/core/src/content_cache.rs` | `664558b8ad997b8d4e893ab01abb5f66d53a479356c83e5f3f08c34cfb2b4b6c` |
| `crates/core/src/content_cache/recovery.rs` | `9157f57c5b9cb831df2784e8d4d66f97f81b839e8abaf13783472b34be559e97` |
| `crates/core/src/content_cache/tests.rs` | `0850b876a823ffc65a9e4c26c79b4ecb14f193ed2eec3a842dac1fee4426037d` |
| `crates/core/src/content_cache/tests/recovery_tests.rs` | `a042ad8a848517ab80b869746555d3a376258c85e9a66d80f2bd3fa683f7b1b4` |
| `reports/content-cache-contract.md` | `0442c28060d4c6745dfb8c5b8bb00948e39574a28b49ea5c2f627070b748552b` |
| `reports/content-cache-implementation.md` | `061b9dcf0924477677ce8c1fc6f2c04febea59a0fe37ddbbbc7ba8e18692b3e9` |

No native Windows/Linux, power-loss durability, HTTP resume/downloader integration,
broad security, full application or master-plan checklist credit is awarded by
this follow-up. Production acceptance remains outside this bounded verdict.

## Documentation/handoff follow-up — October 13, 2026, 09:08 EDT

Reviewed the updated `content-cache-contract.md`,
`content-cache-implementation.md` and new `content-cache-recovery.md` against the
source/test snapshot independently rerun in the preceding follow-up. All four
cache source/test SHA-256s still match that snapshot exactly. This is a
documentation-only review; no third test run was performed or implied. The
independent current result remains **29 passed, 0 failed, 1 ignored**, while the
first **25 passed, 2 failed, 1 ignored** run remains historical in this report.

**Documentation/handoff verdict: initial blocker 3 is resolved for this bounded
handoff.** Current contract and implementation summaries explicitly supersede the
October 12 disposable-only material, which remains labeled historical. The
takeover report records the incomplete worker delivery, both fixture errors,
lead corrections, added regressions and exact narrow command/result without
treating the expired-auth worker's missing final report as reliable evidence.
This is a reviewed lead takeover record, not a recovered worker final report.

The current descriptions agree with the reviewed persistent Unix-only cache,
cooperating-process advisory exclusion, canonical reconstructed capacity,
SHA-256/SHA-1 compatibility verification and synthetic process-kill recovery.
They preserve explicit path-race, payload-versus-physical-quota,
nontransactional error receipt, power-loss/fault-matrix and downloader/integration
limits. No new code or documentation blocker was identified within this scope.

Read their evidence-retention language specifically as inventory/integrity
rejection **before scratch cleanup**, not a promise that every failed reopen
preserves every file: a sync error after removal can still follow changed
scratch inventory, as documented in this independent review. The non-Unix
unsupported-platform fixture is source-reviewed only, not native evidence.
Historical timing numbers and broad worker/lead ownership claims are not new
independent measurements; billing and master-plan completion are not reviewed.

The takeover report's present-tense reference to reviewer revalidation is closed
by this report's preceding green rerun and this documentation check; it does not
constitute a separate run. Previously pending documentation/handoff gates are
now closed only for the reviewed report hashes below. No native Windows/Linux,
power-loss, HTTP resume/downloader, broad-security, production or checklist
acceptance is awarded. Only this independent report was edited by this reviewer.

| Reviewed report | SHA-256 |
| --- | --- |
| `reports/content-cache-contract.md` | `b32adf16edd6b966dce240bdc3246dfa4bfc2a57ddd9e498876762d35cf8f188` |
| `reports/content-cache-implementation.md` | `e89594536cd2cbc4bcc3fe1812a35f3b305134caa8310d94f9605f54ca28c08a` |
| `reports/content-cache-recovery.md` | `18c3d4e2ef58ad7f6c4b6f90e021b88ed03f929ac7bbb84362f11ae2aa31815f` |
