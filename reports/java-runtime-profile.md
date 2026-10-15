# Synthetic Java-runtime contract and profile

**Historical baseline:** The implementation, nine-contract results and timings
below describe the initial October 13, 2026 fixture-only slice, before runtime
integrity changes. They are not current behavior or fresh validation. The
bounded integrity follow-up is documented in `reports/java-runtime-integrity.md`;
current tests verify the manifest and artifact content on successful online warm
ensures. Its matched five-sample rerun is recorded separately at the end here.

October 13, 2026. This bounded VEL-014 slice adds runtime-ensure contracts and
an explicit multi-sample profile, not a completed master-plan checkpoint.
The bounds follow `reports/preparation-path-profile.md`; its sync timings are
not comparable Java baselines. No checklist or other report is changed here.

## Implementation and reproduction

Only `crates/core/src/java.rs`, `crates/core/src/java/profile_tests.rs` and this
report are owned by this slice. The public `ensure_runtime` signature and
`RUNTIME_INDEX` remain unchanged. A private index-origin argument lets tests
execute the production ensure body; the public wrapper always passes the existing
constant. Platform keys, executable paths and `.scopenet-runtime` identity remain
unchanged. There is no production caching, optimization or security-semantics
change, dependency/lockfile addition, commit, packaging or deployment.

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher-core --lib java::profile_tests -- --nocapture
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher-core --lib java::profile_tests::controlled_java_runtime_profile -- --ignored --exact --nocapture
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher-core --lib java::tests::parses_versions -- --exact
rustfmt --edition 2021 --check crates/core/src/java/profile_tests.rs
```

The fixture uses only a disposable `Layout` root, an ephemeral IPv4 loopback
listener, a no-proxy/no-redirect client and synthetic metadata. Client requests
have a five-second timeout, ensure calls a fifteen-second bound, and server
headers an 8 KiB/five-second bound. The server serially serves closed connections;
configured download concurrency is 4, with only one artifact. Dropping each
fixture aborts its listener task and removes the temporary root. No live runtime
stores, player data, Java version probe, JVM, launcher, native webview or game
process are used. The synthetic executable contains 4,096 repeated bytes, not
a runnable Java binary. Unix executable permissions are checked, never executed.

Nine ordinary contracts validate:

- Cold install and matching-marker warm ensure return the same runtime directory,
  preserving marker, verified payload and unmanaged synthetic sentinel.
- Index HTTP failure falls back only with both marker and executable present;
  the other three prerequisite combinations error without changing seeded files.
- Malformed JSON also falls back when installed; a valid index without the
  component errors instead of falling back.
- A matching marker without an executable reinstalls; a changed marker fetches
  the manifest but reuses an already digest-verified artifact.
- Manifest failure preserves previous marker/artifact/sentinel; checksum failure
  makes exactly four artifact attempts and preserves previous bytes and marker.
- Matching-marker warm ensure accepts a present same-size corrupted artifact
  without re-verifying it. This characterizes existing behavior, not endorsement.

Every contract checks exact index/manifest/artifact request counts and zero
unexpected routes. Ordinary cold/warm coverage shares the same sample helper as
the profile, without timing thresholds. The sole ignored test emits one
`JAVA_RUNTIME_PROFILE` JSON record, schema version 1, with five independent
samples and numeric request counters. No URLs, filesystem paths or error strings
are printed in its telemetry.

## Measurement bounds

Each sample creates a fresh logical runtime root/client/server, then times actual
runtime ensure cold and warm in that order. Timers exclude fixture creation and
post-operation assertions but include the bounded await, loopback fetches,
deserialization, file checks, download verification, permissions and marker
publication where applicable. Cold makes exactly one index, one manifest and one
artifact request; warm still makes one index request, zero manifest/artifact
requests, and retains all installed bytes. OS page/disk caches are uncontrolled;
warm also follows cold on the same client/server. These are different paths in
one implementation, not matched before/after optimization results.

## Observed results and validation

Measured October 13, 2026 at approximately 12:03 UTC on Darwin arm64 with
Rust 1.98.1, unoptimized debug test profile and test debug info disabled.

| Flow | Median ms | Min–max ms | Index / manifest / artifact requests per sample |
| --- | ---: | ---: | --- |
| Cold logical runtime ensure | 1.244 | 1.209–1.689 | 1 / 1 / 1 |
| Matching-marker warm ensure | 0.271 | 0.267–0.298 | 1 / 0 / 0 |

Ordered cold microsecond samples: `1689, 1325, 1244, 1209, 1211`.
Ordered warm microsecond samples: `298, 293, 267, 270, 271`.
All five samples passed marker/artifact/sentinel and route-count invariants;
every sample reported zero unexpected requests. The emitted JSON contains
the raw samples and counters, without fixture origins or root paths.

- Focused ordinary contracts: **9 passed, 0 failed, 1 explicitly ignored
  profile**, 87 filtered out, test execution 5.64 seconds. Initial Cargo invocation
  waited for the concurrent Panel build's build-directory lock; no other process
  was stopped and no alternate target/dependency set was created.
- Explicit exact ignored profile: **1 passed, 0 failed**, 96 filtered out,
  test execution 0.01 seconds. No wall-clock thresholds are asserted.
- Existing exact Java-version parser regression: **1 passed, 0 failed**,
  96 filtered out, test execution 0.00 seconds; this parses strings, not a JVM probe.
- Scoped new-file rustfmt and whitespace checks pass. No broad workspace,
  online, native, packaging or release validation is claimed here.

## Existing weaknesses and remaining gates

Inspection and contracts confirm that index HTTP/JSON failure fallback checks
marker readability and executable existence, not marker/digest validity. The
matching-marker warm return likewise does not verify installed artifact content.
Manifest reference SHA-1 is used as marker identity, not checked against fetched
manifest bytes. Installation is not a whole-runtime transaction: individually
verified files can be published before a later failure; the preservation fixture
only covers its single failed artifact. These preexisting weaknesses are not
fixed or given stronger safety guarantees in this slice.

Representative real-runtime metadata and download latency, larger manifests,
multiple files, symlink/platform matrix behavior, failure/repair across a complete
installation, Java version detection/spawn, native preparation and actual
game/loader compatibility remain separate gates. No Java-spawn latency, physical
cold-cache timing, native performance, throughput, significance, speedup or
performance-gain claim is established. Full VEL-014 evidence-led optimization,
matched final measurements and native acceptance remain open.

## Integrity follow-up profile

The follow-up retains the same five-sample, one-artifact, 4,096-byte synthetic
fixture, configured concurrency 4, client, loopback transport and timer scope.
Warm now makes **1 index / 1 verified manifest / 0 artifact** requests and
deep-checks declared artifact bytes. Every warm sample additionally checks that
the matching marker's modification timestamp is unchanged. A synthetic old
timestamp is set before the warm timer, solely to detect unnecessary writes.
There is no JVM/native work, controlled physical cold cache or performance-gain
claim. The baseline warm numbers above are historical, not the current path.

Measured October 13, 2026 at approximately 12:12 UTC, Darwin arm64, Rust 1.98.1,
unoptimized debug test profile with test debug info disabled:

| Flow | Historical median ms | Current median ms | Current min–max ms | Current index / manifest / artifact requests |
| --- | ---: | ---: | ---: | --- |
| Cold logical runtime ensure | 1.244 | 1.319 | 1.178–4.633 | 1 / 1 / 1 |
| Matching-marker warm ensure | 0.271 | 0.568 | 0.503–0.758 | 1 / 1 / 0 |

Current ordered cold microsecond samples: `4633, 1334, 1178, 1319, 1211`.
Current ordered warm microsecond samples: `758, 580, 568, 503, 546`.
All five samples meet marker timestamp, artifact/sentinel and request-count
invariants. The first cold sample is retained, not discarded as an outlier.
The warm median is **0.297 ms higher** than the historical marker-only path in
these two runs. This honestly records the additional metadata/integrity work;
it does not isolate network, digest, decode, filesystem or permission costs.
The existing install loop still applies executable permissions on warm ensures.
Runs are not simultaneous, OS caches/system load are uncontrolled and five tiny
samples do not establish significance, native latency or a performance gain.

Focused current contracts: **16 passed, 0 failed, 1 explicitly ignored profile**,
87 filtered out, 5.62 seconds. Explicit exact profile: **1 passed, 0 failed**,
103 filtered out, 0.03 seconds. Exact existing parser regression: **1 passed,
0 failed**, 103 filtered out, 0.00 seconds. Scoped new-file rustfmt and whitespace
checks pass. At this implementation-time snapshot, independent review and main's changed-core validation remained gates;
the cancelled combined five-package sweep is not a passing result.

Final lead reconciliation: independent review now passes 16 contracts, exact
source hashes match, and final changed-core tests pass 101 ordinary cases with
three explicit-only timing profiles. See `java-runtime-integrity-review.md` and
`final-local-sweep.md`. This closes the bounded source-review/sweep gates only,
not native preparation, performance optimization or VEL-014 acceptance.
