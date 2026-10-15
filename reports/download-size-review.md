# Independent declared-download-size review

Date: October 12, 2026. Read-only source audit; this report is the only file written by this reviewer. Root instructions and MONOREPO.md, docs/BOUNDARIES.md, docs/VALIDATION.md, and docs/DEPLOYMENT_TARGETS.md were read. No build/test duplication, live stores, deployments, branches, PRs, or commits.

## Verdict and review status

**Final reviewed verdict: APPROVE the current four-line production size guard, README contract, and current worker tests. No source-review blocker; this is not release acceptance.** Re-inspection after main's final-results request confirms the production diff is present, not empty or pending. Nonzero sizes consistently originate as artifact/file byte lengths, not advisory download estimates. No intentional inaccurate nonzero production hint was identified in tracked Rust callers. External/cached manifests were not fetched or exhaustively verified; stale or incorrect upstream metadata remains a compatibility risk.

The initial audit preceded the production patch. Following main's fresh-diff request, this reviewer inspected the current diffs for `crates/platform-utils/src/http.rs`, its README, and `crates/core/src/http.rs`. The production change adds the four-line size guard after flush/drop and before checksum/rename, with only adjacent line-ending changes. Cleanup uses contextual error propagation: a failed removal still returns before publication, though that error takes precedence over the detailed expected/actual mismatch message. This is safe and matches the README; fault-injection evidence remains absent. Existing unrelated working-tree edits were not touched. Gate0 progression is user-approved; native and transaction gates remain open. Actual budget telemetry is unknown; this was a bounded audit, not release acceptance.

## Actual caller inventory and meaning

Repository-wide Rust searches found constructor calls below and only the two `Download` type definitions, **no direct production `Download { ... }` initializers**. `Download::new` converts `None` to zero; `Some(0)` also remains zero. `file_ok` already ignores zero/absent sizes, supporting the same download contract.

| Caller | Size source and interpretation |
| --- | --- |
| `crates/core/src/install.rs:70` | Version JSON download deliberately supplies `None`: unknown, not empty. |
| `crates/core/src/install.rs:114` | Client JAR artifact's optional `size`; file bytes. |
| `crates/core/src/install.rs:127` | Logging file's optional `size`; file bytes. |
| `crates/core/src/assets.rs:36` | Asset-index JSON's optional `size`; **not** `totalSize`, which describes the assets. |
| `crates/core/src/assets.rs:53` | Individual asset object's required `size`; file bytes. |
| `crates/core/src/libraries.rs:35` | Resolved library/native artifact size; modern artifact/classifier and legacy top-level library sizes are passed through. Missing legacy metadata stays `None`; empty artifact URLs are filtered. |
| `crates/core/src/java.rs:142` | Runtime manifest `downloads.raw.size`; raw installed bytes, not an alternative compressed runtime representation. |
| `crates/core/src/sync.rs:130` | Panel `FileEntry.size`; distributed file bytes. Empty SHA-1 is normalized away independently of size. |
| `crates/core/src/authlib.rs:41` | `None`: unknown. |
| `crates/core/src/loaders/forge.rs:100` | Forge/NeoForge installer `None`: unknown. |
| `crates/platform-utils/src/mc_textures.rs:217` | Mojang client artifact required size; JAR bytes, not extracted texture bytes. |
| `panel/server/src/routes/account.rs:188` | Client artifact download supplies `None`: unknown. |

`crates/core/src/version.rs` models artifact, library, index, and logging sizes as optional, with serde defaults. Asset objects and Java raw-download sizes are required integers. Panel `FileEntry.size` is a required `u64`; zero represents either unknown metadata or an actual zero-byte file, and this fix deliberately cannot distinguish them.

Panel producer tracing: `crates/panel-packs/src/lib.rs:125` uses extracted override `buf.len()`, line 185 passes mrpack `fileSize` (serde-defaulted to zero when missing), line 436 passes CurseForge `fileLength`, and line 573 forwards size to instance distribution mutations. `panel/server/src/routes/admin.rs:488` records uploaded `data.len()`. `panel/server/src/store.rs:228` forwards stored size into `FileEntry`. These paths do not intentionally synthesize approximate nonzero sizes. The existing signed database-size to unsigned manifest cast warrants caution for corrupt legacy records; no live database was inspected and no schema change is proposed.

## Safety and compatibility findings

- **Decoded bytes, not HTTP Content-Length:** root Cargo enables reqwest gzip; locked reqwest is 0.12.28. Its locally installed `src/async_impl/client.rs:1218` documentation confirms default gzip decoding and removal of encoding/length response headers. The existing stream counter and SHA-1 both cover bytes written to disk. Compare that counter to manifest size; compressed wire length is not interchangeable. A gzip-encoded JSON/artifact response must validate against decoded file bytes. Local primary dependency source was inspected directly; no network response was needed for this conclusion.
- **Fail closed despite matching SHA-1:** stale/wrong nonzero metadata will now fail rather than publish a hash-valid body and potentially redownload it on later size checks. This is consistent with `file_ok`, but changes successful-download behavior; do not quietly reinterpret all sizes as hints. No intentional production exception was found.
- **Preserve old destination:** the fresh mismatch guard returns before rename, after the file handle is closed, and removes the computed `.part` path, not the existing destination. Cleanup failure propagates with context rather than being best-effort. This does not establish concurrent ownership of that path; existing same-destination races are outside this fix.
- **Zero is unspecified, not strict empty:** zero/None must accept nonempty content when other checks permit it. This does not enforce true empty-file length. Size alone is not cryptographic integrity; unknown-size/no-hash installer downloads retain their existing limitations.
- **Existing limits remain:** four-attempt retry also applies to deterministic size mismatch; progress counts retry bytes and can exceed initial totals. This check is not a transfer cap, cancellation-cleanup fix, concurrent-destination isolation, fsync durability, or multi-file transaction. Stream/rename failure and cancellation cleanup behavior are not newly solved by it. Native overwrite semantics and transaction acceptance remain separate gates.

## Shared-library and provenance boundaries

`crates/core/src/http.rs` re-exports `velora_platform_utils::http::*`; root workspace dependencies select `crates/platform-utils`. Launcher and Panel root consumers therefore share the reviewed implementation, including the texture/account paths, without introducing application/gameplay dependencies into the reusable crate.

`packages/packs-platform-utils` is a distinct excluded Cargo workspace with its own Cargo.lock and LICENSE, despite also naming its package `velora-platform-utils`. `crates/panel-packs/Cargo.toml:11` explicitly selects that independent package. Its duplicate HTTP implementation will **not** acquire this fix. No constructor caller into that duplicate downloader was found in the searched checkout; pack imports currently forward metadata through their own pipeline. Do not claim every downloader copy is fixed or merge the independent lifecycle during this task.

Fresh boundary verification also read `packages/packs-platform-utils/SNAPSHOT.json`: it records the SDK revision and source-unchanged adaptation. SHA-256 of its current `src/http.rs` is `cf28dd21fe74c81caf8f33e6becaaa53232644640632b9d9ed3cbfe6e831883d`, exactly matching the snapshot record. `git diff -- packages/packs-platform-utils` is empty. Snapshot source, package metadata, and recorded hashes are untouched; this is explicitly **not a universal Panel-pack downloader fix**.

`docs/components/sdk/PROVENANCE.md` and PROVENANCE.json attribute HTTP/path/metadata extraction to original SCOPENET-MC sources; launcher and pack provenance retain historical extraction records. These are historical attribution, not proof of current byte identity. Both utility packages retain LICENSE files. Keep attribution and independent locks intact; MIT does not grant Velora artwork/trademark rights. The inspected manifests introduce no gameplay/complete-host dependency into platform-utils.

## Test review and remaining evidence

The fresh test-worker patch enables the hash-valid size-gap test and adds shorter/longer declared-size failures both with matching SHA-1 and without SHA-1, explicit `None`/`Some(0)` compatibility, and exact-size success. Crucially, its complete-body fixtures advertise the actual HTTP body length, isolating manifest-size mismatch from transport truncation. Mismatch cases preserve an existing destination and assert the directory contains only that destination, covering partial cleanup. Successful cases assert publication and absence of `.part`. Core tests exercise the root shared implementation through its re-export; production orchestration is unchanged apart from adjacent line endings.

The worker's updated `reports/download-size-fix.md` records the expected pre-fix red run and post-fix targeted result: **10 passed, 0 failed, 1 ignored**, 36 filtered out. This is attributed worker evidence, not an independent test rerun; this reviewer deliberately did not duplicate builds/tests.

Main additionally reports passing full suites: **platform-utils 6 passed; core 43 passed / 4 ignored; launcher 22 passed / 3 ignored; online 7 ignored**. These are main-reported results, not independently executed or log-certified by this reviewer. Ignored online tests are not online acceptance. Current source/test inspection and the recorded targeted results support final source approval; no additional production source is pending in this review. Any subsequent test formatting must preserve behavior.

Explicit nonblocking follow-ups after final source review:

1. Gzip response with wire length different from decoded length, plus chunked/no-Content-Length response using decoded size. Existing fixtures are plain Content-Length responses only. Dependency-source inspection supports the convention, but this is not executable gzip coverage. **Nonblocking for this written-counter check:** it uses the same decoded stream already written and hashed, introduces no header dependency, and does not change transfer decoding or framing.
2. Absent-destination size-mismatch nonpublication and cleanup-failure fault injection. Current mismatch tests all begin with a previous destination. The guard itself is reviewed as safe for both absence and presence; native filesystem behavior remains a separate gate.
3. Unknown-size success without a checksum and true empty-body compatibility are not explicitly covered by the fresh additions, although constructor normalization and unchanged control flow support them.
4. Existing cancellation cleanup remains explicitly ignored and must not be reported fixed. Broader main-owned platform/core/launcher checks and native/transaction acceptance are not independently certified here.

These are nonblocking evidence gaps/risk notes for this small source fix, not claims that additional downloader behavior has been implemented or authorization to expand scope. The inspected guard precedes rename, unknown semantics and checksum validation remain intact, and README limitations are honest. This review applies to the inspected fresh diffs; later changes require re-review. Native and transaction gates remain open.
