# VEL-012 isolated prototype implementation and local evidence

## Current recovery follow-up — October 13, 2026

The source has advanced beyond the disposable prototype below. Explicit Unix
reopen, stable exclusive owner locking, inode/canonical inventory checks,
reconstructed bounds and synchronized process-interruption recovery are now
implemented. Lead corrected two erroneous worker fixture assertions, added
canonical-size/content mismatch and SHA-1 corruption regressions, and gated the
Unix fixture module separately from non-Unix unsupported-platform behavior.
Focused current suite: **29 passed, zero failed, one explicit benchmark ignored**.
Current contract is the first section of `content-cache-contract.md`; implementation
handoff and remaining gates are in `content-cache-recovery.md`. Historical counts,
timings and no-reopen/no-fsync statements below are not current source guarantees.
No downloader integration, HTTP resume, installed-store migration or deployment.

## Historical disposable prototype — October 12, 2026

Date: October 12, 2026, America/New_York. Baseline HEAD:
`4f639266a515c3cb05769b055a4772c4dcb89cb8`. The checkout already contains other
uncommitted agents' work; results below are working-tree results, not pristine-HEAD
certification. This is **isolated prototype evidence, not production caching**.

## Exact owned files

| File | Change |
| --- | --- |
| `crates/core/src/content_cache.rs` | Explicit-root bounded synchronous cache, typed key/namespace/limits/errors, verified staging/retrieval, no-clobber publication and request cleanup |
| `crates/core/src/lib.rs` | Only `pub mod content_cache;` export |
| `crates/core/src/content_cache/tests.rs` | Synthetic integrity/isolation/cancellation/filesystem fixtures and opt-in controlled local benchmark |
| `reports/content-cache-contract.md` | Contract, trust assumptions, limits and unaccepted gates |
| `reports/content-cache-implementation.md` | This implementation/evidence record |

Existing `velora-platform-utils` SHA-256/SHA-1 helpers and the existing `tempfile`
dev dependency are reused. No dependency, lockfile, deployment, updater/signing,
credential-store, protocol or installed-data identity changes are made by this task.
No live downloader call site uses this module. No commits, branches, worktrees,
deployments or external infrastructure are created.

## Focused validation

Commands run sequentially; no broad concurrent build:

```sh
rustfmt --edition 2021 crates/core/src/content_cache.rs crates/core/src/content_cache/tests.rs
CARGO_PROFILE_TEST_DEBUG=0 cargo test content_cache -p velora-launcher-core -j1 --locked
CARGO_PROFILE_TEST_DEBUG=0 cargo test content_cache::tests::controlled_local_benchmark -p velora-launcher-core -j1 --locked -- --ignored --exact --nocapture --test-threads=1
```

- Final focused integrity run: **19 passed, 0 failed, 1 ignored** (opt-in benchmark),
  65 unrelated library tests filtered out; seven online tests filtered out. Test
  execution 0.02 s; final incremental build 4.71 s. Earlier runs with 14 and 17
  passing fixtures are superseded, not additional counts.
- Explicit benchmark run: **1 passed, 0 failed**, five measured iterations. The
  default ignore prevents synthetic sleeps/timing from becoming ordinary CI timing
  assertions. There is no test assertion that warm must be faster than cold.
- Coverage: independent known SHA-256 `abc` vector and upper-case normalization;
  SHA-1 compatibility; exact-empty content; unknown/invalid keys; same-size hash
  corruption and size corruption; truncated/excess source bytes; source-error URL
  redaction; object/aggregate payload/entry bounds; retrieval staging headroom;
  cancellation during population/retrieval and after verification; reader failure;
  unwind cleanup/poisoning; full retry after interruption; namespace/instance
  separation; concurrent publish-once behavior; final-name collision preservation;
  unknown/missing/directory/socket entries; symlink parents/entries/replaced root;
  Unix hard-link aliases and unchanged outside sentinels; closed-handle cleanup,
  idempotent cleanup preserving a replacement sentinel, synthetic unlink failure,
  and exact committed-plus-scratch accounting with two committed objects.
- `rustfmt --check`, owned-file whitespace/export-scope checks and
  `node scripts/check-boundaries.mjs` pass, including compatibility identities.
- These are local synthetic filesystem and synchronous-reader tests, not native
  application, real HTTP validator/resume, crash restart or durable recovery tests.

## Controlled local cold/warm/interruption benchmark

Environment: Darwin arm64, Rust 1.98.1, local debug/unoptimized test profile with
`CARGO_PROFILE_TEST_DEBUG=0`. Final run completed October 12, 2026 around 21:59 EDT
after the closed-handle cleanup fix; these values supersede the earlier run.
Each iteration uses fresh canonicalized temporary roots and a deterministic
1,048,576-byte fixture (`index % 251`), SHA-256 identity and namespace `(1,2,3)`.
Limits: 1 MiB per object, 4 MiB payload including scratch, eight committed entries.

The controlled origin is an in-memory reader capped at 64 KiB per read with a
requested 250 microsecond sleep on each read, including the EOF probe. This
deliberate, reproducible delay is not a network-speed model. Cold means an empty
**application cache**, not a cold operating-system disk/page cache. A cold attempt
performs 17 origin reads including EOF and verifies/publishes the whole object.
Warm retrieves and fully reverifies the entry, with no origin reader or origin bytes.

Interruption is a synthetic connection-reset reader error after 262,144 bytes in
a separate fresh root. The measured interruption interval includes error cleanup;
the test asserts that root contains zero entries afterward. Full retry uses a new
reader from byte zero and transfers all 1,048,576 bytes, then verifies and publishes.
The subsequent recovered read is checked for exact bytes but is outside the retry
timing. Fixture generation, key generation, directory creation and final tempdir
teardown are outside the measured intervals.

Actual observations (microseconds, `Instant`, no estimated timings):

| Iteration | Cold populate | Warm verified get | Interrupted attempt + cleanup | Full retry |
| --- | ---: | ---: | ---: | ---: |
| 1 | 11,144 | 4,432 | 1,853 | 9,923 |
| 2 | 9,082 | 3,615 | 1,511 | 9,055 |
| 3 | 9,053 | 3,628 | 1,516 | 9,009 |
| 4 | 8,982 | 3,559 | 1,524 | 9,154 |
| 5 | 9,005 | 3,595 | 1,520 | 9,102 |
| Median | 9,053 | 3,615 | 1,520 | 9,102 |

Every iteration retained exactly 1,048,576 payload bytes after successful cold
publication, zero scratch bytes after injected interruption, and read the full
1,048,576 bytes again on retry. Peak payload reservation is 2 MiB when retrieving
the 1 MiB committed object; physical block allocation/metadata and peak RSS were
not measured. Full retry is intentionally **not resume** and saves none of the
262,144 interrupted origin bytes.

These numbers are local observations only. Artificial origin delays, warm OS
caches, debug compilation, scheduler noise and a small five-run sample prevent
generalizing to player installs, launch time, Internet throughput, native desktop
performance or a production speedup percentage. There is no before/after live
downloader comparison and no install-speed claim.

## Remaining limits and gates

### Response to emerging main-review concerns

`Scratch` now owns an `Option<File>` and explicitly takes/drops that handle before
unlink in both normal cleanup and `Drop`. An idempotent successful cleanup also
prevents a second unlink at that name. Local regressions cover success, synthetic
unlink error and replacement-sentinel preservation; the existing cancellation,
reader-interruption and unwind tests continue passing. This fixes source ordering
without claiming a native Windows run.

Retrieval's additional `key.size` reservation is retained, not weakened: the
published source is already counted once and retrieval creates a **distinct**
scratch copy before verification. A new regression publishes two 3-byte entries
and proves the 6-byte committed accounting remains unchanged; an 8-byte budget
rejects retrieval and a 9-byte budget permits it and removes scratch. Returning
bytes from a full root without scratch headroom would violate this prototype's
payload bound. Native acceptance and independent review remain uncredited.

The contract report is the authoritative limitations list. In particular: no
shared persistent root, interprocess locking, eviction, reference tracking,
automatic corruption repair, resume, durable publish/recovery or hostile path-race
defense. SHA-1 remains compatibility-only. Cleanup failures can leave scratch and
fail-close later access; successful link followed by failed unlink can leave a
verified entry despite an `Io` result. No fault-injected disk-full/unlink-denial or
process-kill/power-loss evidence is supplied.

Native OS/filesystem, security/data-safety independent review and durable recovery
gates remain open. Integration into the downloader is deferred. This worker does
not award its own VEL-012 completion points or claim recovery/security acceptance.
Rollback consists of removing these isolated module/test/report additions and
the single export; no live-store migration or deployed artifact must be undone.
