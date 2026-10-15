# Independent bounded download-policy review

October 12, 2026. Reviewed root utility downloader changes, actual core regression
integration and lead-produced logs. No source/test edits, duplicate builds,
services, stores, commits or deployments. Independent packs utility snapshot and
broader process supervision are outside this review.

## Verdict

No blocking defect found within the explicitly bounded source/local-fault scope.
The prior cancellation cleanup gap is closed by owned synchronous file lifetime
and enabled regression evidence. VEL-003 checkpoint 4 may receive source/local
fault acceptance only; this is not native Windows/OS, crash, hostile-path or
whole-process quiescence credit. The deliberate compatibility and performance
changes below must remain documented rather than described as behavior-neutral.

## Ownership, cancellation and failures

- `PartialDownload` exclusively creates a unique sibling via `create_new`, owns
  its `std::fs::File` and path, and avoids the destination itself. Chunk writes,
  flush, close, rename and deletion execute synchronously rather than through
  detached filesystem workers. At network suspension points, dropping the
  aborted future drops the guard; `cleanup` takes/drops the file before removing
  its path. Task join after abort therefore observes completed destructor cleanup,
  or a logged deletion failure, not an outstanding async chunk writer.
- Normal stream/write/size/digest errors close before cleanup. Publication closes
  before synchronous rename and has no cancellation await between verification
  and publication. A cancellation request arriving during synchronous work may
  be delayed and publication may complete; it is not a guarantee that an abort
  request prevents any already-started publication.
- Explicit cleanup failures add owned-path deletion context to the original
  error chain. The actual core fault replaces the owned path with a directory and
  asserts both cleanup and checksum failure contexts while preserving the old
  destination. Cancellation Drop cannot return an error, so it logs cleanup
  failure. `cleanup` clears its ownership flag before removal; a failed deletion
  is not retried by Drop. This is honest residual/manual-cleanup behavior, not
  an assertion of successful deletion on every filesystem.
- Ownership is path-based, not a handle-bound hostile-directory proof. Local path
  replacement, shared-destination writers, crashes and power loss remain gates.
  File flush is not file/directory fsync crash durability.

## Response and size limits

- Fresh responses still require exactly 200. Initial artifact URLs must have an
  HTTP(S) scheme, host and no embedded user/password; invalid URLs fail before a
  request/partial creation. Final artifact URLs receive the same validity check,
  with initial-HTTPS/final-HTTP downgrade rejected before writing.
- Positive declared size remains an exact ceiling and final equality contract.
  `download_one` now gives omitted/zero size an 8 GiB per-attempt ceiling, even
  without a digest. `download_one_with_limit` rejects declared size above its
  caller ceiling before networking; callers can select another explicit ceiling
  without changing `Download` fields. Advertised unknown-size length above the
  limit fails before partial creation.
- Every actual streamed chunk checks checked-add overflow, positive declared
  ceiling and explicit limit before synchronous write, hashing or byte callback.
  Chunked/absent/misleading lengths therefore cannot bypass the stream guard.
  No-digest acceptance still has no authenticity guarantee. This limit is per
  attempt/file, not an aggregate batch/resource or decoded-JSON memory bound.
  Retries can consume additional network bytes; retry progress is not redefined.

## Redirect policy and intentional compatibility

The production shared client installs the custom policy combined with reqwest's
10-hop limit. It permits HTTP(S) CDN changes, rejects embedded credentials and
non-HTTP targets and rejects downgrade after any HTTPS hop. Thus HTTP to HTTPS
to HTTP is rejected, not just initial HTTPS to HTTP. Initial private/local HTTP
remains permitted for configured/self-hosted artifacts: no SSRF/private-network
protection is claimed. This shared-client policy also affects JSON/authlib callers
using the same client, not only `download_one`; credential-bearing redirect URLs
and insecure chains formerly accepted may now fail intentionally.

Caller-supplied clients retain their own redirect policy. Initial/final artifact
checks cannot certify hidden intermediate hops (especially HTTP to HTTPS to HTTP
with initial HTTP). Pure policy decisions exercise downgrade/credentials/schemes;
loopback integration exercises following an allowed redirect and rejecting a
loop. There is no native TLS/network matrix claim. Protocol IDs, persisted paths,
credential-store/updater/signing identities and package lockfiles are unchanged
by this bounded utility patch.

Synchronous writes/flush/rename/remove can block a runtime worker under slow disks
or storage faults. This is a correctness tradeoff to remove detached-write races,
not a throughput/latency improvement. Download bodies of unspecified size above
8 GiB now fail unless a caller explicitly chooses another ceiling. Broad process
shutdown, native Windows sharing modes, storage exhaustion, performance profiling
and restart recovery remain open.

## Evidence and reviewed hashes

Inspected `/tmp/velora-revitalization-50/download-final.log`: core **65 passed,
zero failed/ignored**, utility **10 passed, zero failed/ignored**; seven online
tests remain ignored and are not acceptance. Actual core fixtures cover aborted
partial cleanup, preserved destination/retry, owned sibling isolation, publication
and cleanup faults, advertised/chunked no-digest limits and pre-network rejection.
Utility tests cover guard Drop and redirect decisions/loop behavior. Tests use
synthetic local data and transport. No duplicate Cargo execution by this reviewer.

| Path | SHA-256 |
| --- | --- |
| `crates/platform-utils/src/http.rs` | `48d140a0a05b26957893b3b95e0400b6f0ca793324598449cbb0f4b777c17358` |
| `crates/core/src/http.rs` | `d417c108db354ebf6970037daf48b2ea794ded07bb14f582430913c69693440f` |

Source hashes identify this review snapshot, not release/native acceptance.
