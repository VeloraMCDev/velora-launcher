# Bounded Java-runtime integrity follow-up

October 13, 2026. This changes the online runtime-ensure integrity behavior found
by the bounded VEL-014 fixture; it is not an optimization or a completed VEL-014
checkpoint. Only `crates/core/src/java.rs`,
`crates/core/src/java/profile_tests.rs`, `reports/java-runtime-profile.md` and
this report are changed by this follow-up. Public `ensure_runtime`, the default
`RUNTIME_INDEX`, platform keys, executable paths and `.scopenet-runtime` identity
remain unchanged. No dependencies, lockfiles, metadata caches, new persisted
paths, commits, deployments or checklist changes are introduced.

## Implemented contract and exact limits

- The selected manifest reference must contain exactly **40 ASCII hexadecimal
  digits** (upper or lowercase). Empty, short, long, nonhex, whitespace and
  non-ASCII references fail before requesting the manifest.
- Fetching requires successful HTTP status. A known Content-Length greater than
  **16 MiB = 16,777,216 bytes** fails before body collection. Streamed response
  chunks are checked before appending; appending even one byte beyond that limit
  fails. Exactly 16,777,216 bytes are permitted. The cap covers the response bytes
  exposed by reqwest, including decoded bytes when its client decodes compression;
  it is not a process-wide memory or network-byte limit.
- SHA-1 is calculated over the **actual collected manifest bytes**, never a
  reserialized JSON object, and compared case-insensitively to the strict
  reference. JSON decoding occurs only after a matching digest. All of these
  checks precede directory creation, downloads, permissions, links and marker
  writes. SHA-1 compatibility is retained; this is not a new authenticity scheme.
- A matching installed marker no longer bypasses the manifest. A successful
  online ensure obtains the verified manifest and uses the existing deep
  size/SHA-1 artifact checks. A corrupted declared artifact is repaired through
  the existing verified-download path before returning success.
- The marker is written only if its current value differs from the selected
  reference, and only after all downloads and the existing installation loop
  succeed. Matching-marker completion/repair does not unnecessarily rewrite it.
  Reference letter case is preserved in marker bytes, retaining marker identity.
- Unmanaged files remain untouched. Existing symlink, executable-permission,
  durability and partial-installation behavior is not redesigned here.

## Fixture coverage and reproduction

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher-core --lib java::profile_tests -- --nocapture
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher-core --lib java::profile_tests::controlled_java_runtime_profile -- --ignored --exact --nocapture
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher-core --lib java::tests::parses_versions -- --exact
rustfmt --edition 2021 --check crates/core/src/java/profile_tests.rs
```

The original nine contracts now require warm manifest requests and replace the
corruption-acceptance characterization with a corruption-repair regression.
Cold/warm samples check retained marker modification time, installed artifact
bytes and the unmanaged sentinel. Seven additional contracts cover:

1. Valid-reference digest mismatch on both syntactically valid, mutating JSON
   and invalid JSON: digest verification wins before installation or decoding.
2. Six malformed-reference variants: zero manifest/artifact requests.
3. Uppercase valid digest verification and exact reference marker bytes.
4. Digest-matching invalid JSON rejected before installation.
5. Exact 16 MiB manifest body accepted, verified and decoded with padding.
6. Oversized advertised Content-Length rejected with only a small actual body.
7. Oversized chunked **and** close-delimited bodies rejected by streamed checks.

Rejection cases retain old marker, artifact and unmanaged sentinel, create no
declared new directory and issue no artifact request. Body-limit stream fixtures
generate repeated 16 KiB chunks, not large logged bodies; only bounded error
categories are asserted. All routes have exact numeric request counts with zero
unexpected routes. Test roots are disposable and clients are no-proxy, no-redirect
loopback-only fixtures with five-second request/fifteen-second ensure bounds.
No live store, JVM/game, native webview or launcher process is exercised.

## Focused results

October 13, 2026 at approximately 12:12 UTC, Darwin arm64, Rust 1.98.1:

- Ordinary contract filter: **16 passed, 0 failed, 1 explicitly ignored profile**,
  87 filtered out, test execution 5.62 seconds. Cargo initially waited for the
  shared build-directory lock; no other worker's process was stopped by this slice.
- Explicit five-sample exact profile: **1 passed, 0 failed**, 103 filtered out,
  test execution 0.03 seconds. All route, bytes and marker-time invariants passed.
- Exact existing parser test: **1 passed, 0 failed**, 103 filtered out,
  test execution 0.00 seconds. This is string parsing, not a Java version probe.
- Scoped new-file rustfmt and owned-file whitespace checks pass. No new target
  directory or package/dependency combination was built. Large cap-test bodies
  exist only in memory/network buffers, not large runtime files or raw logs;
  the oversized Content-Length case rejects a small body before collection.

The matched synthetic profile is fully recorded in
`reports/java-runtime-profile.md`, preserving the historical baseline. Current
cold median **1.319 ms** (1.178–4.633), warm median **0.568 ms** (0.503–0.758).
Historical warm median was **0.271 ms**: the current median is **0.297 ms higher**,
with one additional manifest request, SHA-1/decode, artifact deep verification and
the existing install loop. This is not a stage-cost decomposition or evidence of
a native performance gain.

## Remaining gates and deliberately unchanged behavior

Index HTTP/JSON failure fallback still accepts an installed readable marker and
existing executable without re-verifying them. The fallback contract is preserved,
not a fresh offline integrity guarantee. Index metadata remains on its existing
fetch path and is not covered by the new manifest-body cap. Artifact-reference
validation, path/symlink trust, permission failures, concurrent modification and
whole-runtime transactional publication/durability are not strengthened here.
Multiple successful file publications before a later failure remain possible;
the single failed-artifact contract does not prove whole-install rollback.

The profile uses a small synthetic executable and uncontrolled OS caches. Warm
now incurs an additional manifest request, digest/decode work and artifact deep
verification. Full real-runtime, platform/architecture matrix, native preparation,
Java version/spawn and game/loader acceptance, representative larger manifests,
matched final VEL-014 measurements and evidence-led optimization remain open.
There is no Java-spawn/native performance, significance, throughput or gain claim.

**Historical implementation-time status: separate-agent re-review was pending.**
These focused checks are implementation evidence, not that independent review. Main's native sweep
compiled the earlier version and must rerun the changed core afterward; its prior
result does not validate this follow-up. Main reports Panel native build and
synthetic backup/restore passed independently; this worker does not claim to have
rerun them. The cancelled combined five-package sweep is **not** reported as
passing. Main plans the original four-package sweep and separate Panel tests
after source stability, reusing existing artifacts.

## Final lead reconciliation

The separate review is now complete in `java-runtime-integrity-review.md`:
16 contracts pass independently with bounded acceptance, not blanket security.
Lead confirmed both source hashes match that review and the final changed-source
four-package suite passes (core 101; host/utility/shared 99). Separate Panel
library tests pass 72; final native build and synthetic backup/restart/restore
pass. See `final-local-sweep.md` for exact commands and scope. The cancelled
combined command remains excluded. This follow-up adds no VEL-014 checkpoint.
