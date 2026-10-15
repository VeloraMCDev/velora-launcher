# Independent downloader response final source review

Date: October 12, 2026. Baseline HEAD: `4f639266a515c3cb05769b055a4772c4dcb89cb8`.

**Verdict: strict fresh-response, declared-size-before-write, exclusive-temp and ordinary failure-cleanup behavior are supported by the current source and regression assertions. Final execution acceptance remains pending main's Rust run.** No duplicate build/test run was performed; historical green results are not presented as results for this revision. This does not approve cancellation cleanup, native Windows behavior or release readiness.

## Scope and source identity

Reviewed from the repository root under AGENTS.md, MONOREPO.md, docs/BOUNDARIES.md, docs/VALIDATION.md and docs/DEPLOYMENT_TARGETS.md. Inspected the reusable production HTTP downloader and its actual core re-export/orchestrator regressions, including the corrected oversized-chunk assertions. Only this report and `reports/staged-sync-review.md` are written; no production/test/snapshot/package edits, external network use, native/live-store work, commit, PR or deployment.

| File | SHA-256 |
| --- | --- |
| crates/platform-utils/src/http.rs | `17eee397578c75c6c063ba0f5041ea5318b89cd121e8b72c5a84ccf261deeef7` |
| crates/core/src/http.rs | `ca9beecfab508c3acf058457dcdc444648bdd2afc8919cf1f11d8319ddebbed7` |

## Reviewed behavior and regressions

### Full fresh response

`crates/platform-utils/src/http.rs:119` accepts only final `200 OK` before creating a temp or consuming body bytes. A generic successful-status check no longer allows `206 Partial Content` or `204 No Content` to publish a fresh artifact. `crates/core/src/http.rs:291` rejects 206, 204, 205, 201 and 404, asserting zero byte progress, unchanged existing destination and no leftover temp. Client redirect behavior is unchanged; this check is not redirect-origin policy, resumable/range acceptance or validation of every response header.

### Positive declared size is a write ceiling and final equality check

The stream uses checked cumulative byte arithmetic and rejects a chunk exceeding positive `Download.size` before `write_all`, hash update or progress notification. End-of-stream positive size equality is checked independently of digest equality. Thus a matching checksum cannot authorize an oversized or undersized declared artifact. The ceiling comes from the download declaration, not merely HTTP Content-Length.

`crates/core/src/http.rs:203` and `crates/core/src/http.rs:236` cover size mismatch with matching hashes, both mismatch directions and absent checksum; truncated HTTP body and hash-error tests preserve the old destination and assert owned-temp cleanup. Successful exact size, zero and omitted size fixtures retain compatibility. `Download::new` still filters empty optional digests, SHA-1 equality remains case-insensitive and zero/omitted size still means unspecified, not empty. Staged sync separately requires a valid manifest SHA-1; the general downloader API remains optional-digest.

### Exclusive sibling attempt files and publication

`crates/platform-utils/src/http.rs:173` constructs sibling `.velora-download-<PID>-<atomic counter>.part` names, opens with `create_new`, retries existing candidates without truncation and bounds allocation attempts to 128. A process-wide counter separates concurrent attempts; exclusive creation protects occupied candidates even across processes/PID reuse. The candidate equal to the requested destination is skipped. This is exclusive temp allocation, not exclusive final-destination ownership.

After streaming and flush/verification, the owned file is dropped before rename. Normal success publishes only the verified attempt via rename; normal body/hash/size/publication errors remove that attempt's temp. Cleanup failure adds context while retaining the underlying error rather than reporting success (`crates/platform-utils/src/http.rs:156`). There is no directory/file durability protocol guaranteeing power-loss persistence.

`crates/core/src/http.rs:345` overlaps an active stalled attempt with a successful attempt, proving the first temp and a legacy foreign `.jar.part` are not shared, truncated or removed. `crates/core/src/http.rs:370` forces rename failure using an occupied directory destination, asserting preservation of its sentinel and removal of the attempt temp. `crates/core/src/http.rs:384` forces cleanup failure and asserts both cleanup and checksum errors survive; all four failed attempts' retained artifacts are intentional evidence. Direct write/flush failure and every create-new collision branch are not separately fault-injected.

### Corrected oversized-chunk test

`crates/core/src/http.rs:320` gates the response after a four-byte server prefix, then exposes an eight-byte body against a seven-byte declaration. The corrected callback checks positive progress increments no larger than seven and actual owned-temp metadata no larger than seven. The retry-wide accumulated progress assertion is `1..=28`: accepted progress across four attempts is not a successful-file length and is not necessarily exactly sixteen bytes.

The test no longer assumes that reqwest/TCP presents every server write as an exactly four-byte response chunk. Its gated prefix gives an initial accepted callback; later chunk boundaries may split or coalesce. The source establishes the before-write guard, while the fixture checks bounded writes and destination preservation. Callback-only metadata observation is not a general independent high-water monitor of all filesystem activity. Aggregate downloader progress still includes bytes accepted on failed retries; that existing behavior is not changed here.

## Completion and remaining gates

- VEL-003 checkpoint 1's declared-size/digest publication contract remains intact. Checkpoint 2's full-response/exclusive-temp/positive-size handling has source and fixture support. Checkpoint 3 has response/body/hash/size/publication/cleanup fault regressions, subject to main's final execution results; it is not exhaustive OS I/O fault acceptance.
- VEL-003 checkpoint 4 stays open. `crates/core/src/http.rs:279` still explicitly ignores cancellation-temp cleanup. Abort drops the future before ordinary cleanup; owned leftovers are not reclaimed automatically. The interruption/retry and overlapping-attempt fixtures prove destination preservation and isolation, not cancellation cleanup.
- Zero/omitted size still has no independent byte ceiling; without digest or positive size a 200 body can publish without an integrity/length contract. With a digest but no size, unbounded transfer remains possible before final verification. Do not describe all downloads as bounded or verified.
- Redirect policy, external networking, malicious-path/TOCTOU and cross-process destination arbitration are not accepted. Concurrent successful writers can race the final rename. PID/counter temps prevent sharing an attempt file, not races at publication.
- Native Windows replace/open-handle behavior, crash/power-loss durability and cancellation/quiescence acceptance remain manual gates. Source tests use synthetic temporary directories and local HTTP fixtures; this review ran neither those fixtures nor native acceptance.

## Handoff

No new blocking source finding was identified for the requested narrow fresh-response/downloader changes. Main should attach final Rust results for these source identities before marking new checkpoints execution-verified. Keep the ignored cancellation regression and no-size/redirect/native gates visible; no checklist, packaging or release status is changed by this report.
