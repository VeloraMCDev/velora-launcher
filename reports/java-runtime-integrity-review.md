# Independent bounded Java-runtime integrity review

October 13, 2026. Separately authorized reviewer; sole write ownership is this
report. Reviewed the working-tree change to `crates/core/src/java.rs`, new
`crates/core/src/java/profile_tests.rs`, and the implementation/profile evidence
in `reports/java-runtime-integrity.md` and `reports/java-runtime-profile.md`.
Read the root instructions, `MONOREPO.md`, `docs/BOUNDARIES.md`,
`docs/VALIDATION.md` and `docs/DEPLOYMENT_TARGETS.md`. No nested `AGENTS.md` was
found under `crates` or `reports`. Read the path and HTTP helpers only to trace
the actual verification/publication behavior.

## Verdict and exact scope

**Bounded acceptance: no blocker found for the manifest-verification and online
warm-artifact-repair change reviewed here.** The specified compatibility
identities remain intact, and the focused 16 contracts pass independently.
Acceptance includes the deliberate availability/latency tradeoff described
below; it does not mean online warm behavior is unchanged.

This closes only this independent review of the identified source snapshot.
It grants no blanket security, release, native compatibility, performance-gain,
VEL-014 or master-plan checkpoint credit. Main's broader changed-source sweep,
representative Mojang-runtime/platform testing, packaging/signing and native
Java/game/loader acceptance remain separate gates. No other worker's test or
deployment result is adopted as reviewer evidence.

Base HEAD was `4f639266a515c3cb05769b055a4772c4dcb89cb8`; the review is of uncommitted
working-tree contents, not just that commit. SHA-256 snapshots, unchanged across
the focused run:

| Reviewed file | SHA-256 |
| --- | --- |
| `crates/core/src/java.rs` | `893bccb2c28ba4c32fa654b0d310dfd68c27b27a8c8c0af2cdbbe941f3465787` |
| `crates/core/src/java/profile_tests.rs` | `0c029ce897dc9dea98fe735586b21849272c6f5304e96e5282b409bd7ed24674` |
| `reports/java-runtime-integrity.md` | `05105e3843b3dba4ec89bf0fe6c4bbf8db25953ab19ec70e93835b2649bad958` |
| `reports/java-runtime-profile.md` | `e15271a5477d2a24ed464f2d9a636a1dc9efb7134b6c6b20e5d015839748599c` |

## Verified contracts

- **API and persisted identities:** `java.rs:123` retains public
  `ensure_runtime(client, layout, component, concurrency, reporter)` and its
  return type. The private index-origin seam is not a public override; the
  wrapper always passes the original Mojang `RUNTIME_INDEX` at `java.rs:13`.
  Platform keys, executable paths, `Layout::runtimes()` and the existing
  `runtimes/<component>/.scopenet-runtime` marker identity are unchanged. The
  patch does not introduce a cache, alternate root, dependency or signing/store
  identity. Broader unrelated working-tree changes are outside this verdict.
- **Strict manifest reference:** `java.rs:60` rejects anything other than 40
  ASCII hexadecimal bytes before its network request. The six malformed cases
  cover empty, short, long, nonhex, whitespace and non-ASCII references.
  Uppercase references verify case-insensitively while retaining their exact
  reference bytes as marker identity; marker equality itself remains exact.
- **Bounded response collection:** known Content-Length above 16,777,216 bytes
  is rejected before body collection. Each returned chunk is checked against
  the remaining allowance before append, including absent-length responses.
  The maintained `bytes.len() <= limit` invariant makes the subtraction safe.
  Exactly 16 MiB is accepted; tests independently exercise oversized advertised
  length, chunked framing and close-delimited framing. This is a collected-body
  length bound, not a cap on allocator capacity, reqwest buffers, decoded JSON
  allocations, total RSS or wire traffic. It bounds bytes exposed by reqwest;
  compression behavior follows the supplied client.
- **Digest before decode and installation:** SHA-1 is over collected response
  bytes, not reserialized JSON. Only a matching digest reaches JSON decoding;
  only successful decoding reaches the installation loop at `java.rs:160`.
  Neither malformed references, request/body failures, cap rejection, digest
  mismatch nor JSON decode failure reaches directory creation, artifact
  downloads, permission/link updates or marker writes. Tests use both mutating
  valid JSON and invalid JSON with mismatched digests to establish ordering.
- **No marker-only online bypass:** after a successfully parsed index and
  selected entry, every ensure fetches/verifies the manifest. Declared files
  pass through `http::file_ok(..., true)` at `java.rs:174`, checking declared
  nonzero size and nonempty digest. Missing or same-size corrupted artifacts
  are downloaded through the existing verified publication path. Warm repair
  and changed-marker reuse have explicit request/byte contracts. The helper
  publishes a downloaded file only after its size/digest checks; this is not a
  new whole-install transaction.
- **Matching marker not rewritten:** `java.rs:205` skips the write when the
  marker value read at entry exactly equals the selected reference. The
  cold/warm fixture resets the marker to an old synthetic modification time
  and checks that time and bytes after warm verification. A changed reference
  is written only after downloads and the existing install loop. This is a
  snapshot comparison, not protection against concurrent marker modification.
- **Failure preservation evidence:** shared rejection assertions retain old
  marker, installed artifact and unmanaged sentinel, create no declared new
  directory, and require zero artifact requests. Manifest HTTP failure retains
  seeded bytes. The bad-artifact fixture seeds genuinely different prior bytes,
  receives wrong same-size content on all four attempts, and retains those prior
  bytes and marker. This proves the single failed-file scenario, not rollback
  of directories or other files already published in a multi-file installation.

## Online-warm availability and latency tradeoff

Previously a matching marker plus present executable returned after the index
request. Now that same installation must also fetch and verify the selected
manifest and deep-check declared files. If the index succeeds but the manifest
request times out, fails HTTP, exceeds the cap, mismatches the digest or cannot
decode, even a matching-marker installation errors instead of launching via
that old bypass. There is no verified cached-manifest fallback. This deliberate
fail-closed behavior enables repair but adds a metadata-host availability
dependency; it is not merely a transparent performance change.

The manifest-failure fixture uses an old, nonmatching marker. The exact
matching-marker manifest-outage case has no dedicated fixture; nevertheless,
inspection establishes it unambiguously because no marker comparison branches
around `verified_runtime_manifest`. That additional regression fixture would
strengthen availability documentation, not block this bounded acceptance.

Latency includes one extra manifest request, collection/hash/decode, synchronous
artifact reads/hashes and the existing permission/link loop. The normal shared
client has 15-second connect and 60-second read timeouts, not the fixture's
five-second total request/fifteen-second ensure bounds. The manifest helper adds
no independent total deadline or retry policy; a caller-provided client may
behave differently, and continuing reads can extend total latency. The byte cap
is not a time bound.

The supplied profile honestly distinguishes its historical marker-only baseline
from the current follow-up: warm median 0.271 ms versus 0.568 ms, an additional
0.297 ms in two tiny loopback runs. I did not rerun the benchmark or independently
reproduce those timings. They do not predict remote-network or real-runtime
latency, isolate stage costs, or establish significance or performance gain.

## Unchanged fallback and residual gaps

- **Index-error fallback is not offline integrity:** `java.rs:145` still returns
  an installed directory on index fetch/parse error when any readable marker
  string and an existing console executable are present. It does not validate
  marker syntax/reference, file type, artifact bytes or executable permissions.
  The tests preserve HTTP/JSON failure fallback and its prerequisite matrix;
  they do not prove corruption rejection in this branch. A parsed index lacking
  the requested component still errors. Index collection remains uncapped by
  this new manifest limit. A compromised index is not authenticated by a digest
  that the same index supplies; retained SHA-1 is protocol compatibility, not a
  new signature/authenticity scheme.
- **Paths, symlinks and races:** `safe_join` rejects lexical traversal but does
  not prevent preexisting symlink-parent escape. The component itself is still
  joined directly. Link targets are not containment-validated; Unix warm
  ensures remove/recreate declared links and ignore failures, while non-Unix
  link handling is unchanged. File verification follows links and has a
  verify/use race. No runtime-wide lock or concurrent-modification defense is
  established. These existing properties are not hardened by this change.
- **Permissions and complete usability:** Unix executable permission failures
  are discarded; even an intact warm installation executes that permission
  loop again. Link failures are also discarded. There is no final guarantee
  that the expected Java executable exists and is executable. Tests check the
  happy-path Unix mode, not failure behavior or real JVM operation.
- **Durability/publication:** verified downloads use per-file temporary
  publication, not whole-runtime staging/rollback. Directory creation and other
  successful artifact replacements can precede a later failure. Flush/rename
  does not establish file/directory fsync durability; marker writes are direct,
  not crash-safe atomic commits. A retained matching marker is not proof that
  every other operation succeeded or remains intact under races/crashes.
- **Manifest shape and artifact references:** serde decoding is not semantic
  validation of a usable runtime. Unknown kinds, unsafe relative entries,
  missing downloads/targets are silently skipped; empty/incomplete manifests,
  normalized-path collisions and missing Java declarations are not rejected
  as such. Artifact digests do not receive this new strict reference validation;
  empty digest and zero size retain helper semantics that weaken verification.
  The 16 MiB body cap is not an entry-count/decoded-object bound. Representative
  valid Mojang manifests, multi-file failures, malformed-shape fixtures and
  Windows/macOS/Linux architecture/link cases remain separate compatibility
  and hardening work, not guarantees of these 16 tests.

These limitations prevent broader security/offline-integrity/usability claims.
They are not newly introduced blockers for the narrowly specified improvement.

## Independent validation and ownership

Ran on Darwin arm64 with Rust 1.98.1:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher-core --lib java::profile_tests
```

Result: **16 passed, 0 failed, 1 explicitly ignored profile, 87 filtered out**;
test execution 5.64 seconds. Cargo initially waited for the shared build-directory
lock and reported the test profile finished in 47.84 seconds. No concurrent
process was interrupted and no alternate target directory was used. The ignored
multi-sample benchmark was not run. Tests use disposable roots and synthetic
loopback fixtures, never a native app, JVM, game or live runtime store.

Only this report is written by this reviewer using `apply_patch`. No production
code edits, branch, commit, native launch, live-store access or deployment is
performed. The two implementation reports' pending-review statements describe
their earlier evidence state; this separate report records the completed bounded
review without altering those other workers' owned files.
