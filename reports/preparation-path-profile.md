# Synthetic preparation-path profile

## Current recovery follow-up

The original measurements below are historical pre-journal evidence. Staged-sync
now uses cooperative ownership and a write-ahead recovery journal for mutations,
with a verified read-only guard for unchanged requests. Current comparable
five-sample measurements and significant synchronized cold/repair costs are in
`sync-recovery-implementation.md`. This remains a synthetic fixture, not VEL-014
native install/Java/loader acceptance or a general performance gain.

October 13, 2026, measured at approximately 11:50 UTC. This advances VEL-014
discovery without awarding its complete install/Java-profile checkpoint. Only
tests and reporting changed; production sync, download, Java, persisted paths,
identities and transaction semantics are unchanged.

## Reproduction and scope

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher-core --lib sync::performance_tests::profile_fixture_checks_real_sync_and_preserves_unmanaged_content
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher-core --lib sync::performance_tests::controlled_preparation_profile -- --ignored --exact --nocapture
```

Runtime: Rust 1.98.1, Darwin arm64, debug test profile with test debug info disabled.
The second command prints one `SYNC_PROFILE` JSON record, schema version 1. Logs
are local at `/tmp/velora-revitalization-75/preparation-profile-fixture.log` and
`preparation-profile-benchmark.log`, not committed artifacts.

Five samples each create a fresh temporary instance with 64 synthetic managed
files, 16 KiB each (1 MiB payload), plus an unmanaged synthetic save. Files have
real declared size/SHA-1 checks but are not valid Minecraft mod jars. The existing
test HTTP server binds only loopback, serves the same known bytes and closes each
connection; it processes responses serially. Sync's configured concurrency is 4,
not a claim that the fixture server transfers four bodies simultaneously. The
client disables proxies. No external origins, real player data, launcher app or
Java/game process is used. OS page/disk caches are uncontrolled.

Within each sample, the **actual production `sync`** runs in order:

1. Cold logical instance: no managed files, downloads all 64 and publishes state.
2. Same-revision warm: no downloads/removals/cleaning.
3. Deep verification: no downloads for intact files.
4. Same-size corruption of one synthetic file followed by deep repair: exactly
   one download and restored payload.
5. Separate metadata-only `file_ok` loop and snapshot-fingerprint loop over the
   repaired files. These are isolated helper measurements, not additive stage
   timers or a substitute for the full sync measurements.

Timers exclude fixture construction and post-operation assertions. Full sync
timers include network requests when applicable, validation, staging/state writes,
fingerprints, publication and cleanup. Every flow checks revision, managed count,
unmanaged save contents and absent staging workspace; final declared hashes match.
No timing threshold, speedup requirement or real-world throughput claim is imposed.
The ordinary three-file fixture exercises the same helper and runs with normal
tests. Only the five-sample timing run is explicitly ignored by default.

## Observed results

| Flow | Median ms | Min–max ms | Downloaded artifacts per sample |
| --- | ---: | ---: | ---: |
| Cold logical instance | 229.703 | 225.590–230.757 | 64 |
| Same-revision warm | 14.409 | 14.016–16.005 | 0 |
| Intact deep verification | 16.365 | 15.181–17.608 | 0 |
| One-file deep repair | 20.579 | 19.262–24.335 | 1 |
| Metadata-only helper loop | 0.118 | 0.112–0.130 | Not a sync flow |
| Snapshot-fingerprint helper loop | 3.690 | 3.546–3.893 | Not a sync flow |

These are different paths on the same current implementation, **not before/after
optimization results**. No statistical significance, physical cold-cache behavior,
native preparation latency, network throughput or install speedup is established.

## Evidence-led follow-up

The cheap per-file warm size check is not the complete warm sync. Inspection of
`sync.rs` shows content fingerprints for snapshots and subsequent mutation checks,
plus staged state writing/publication even with zero downloads. `safety.rs` hashes
regular-file contents; publication and rollback also use fingerprints. The measured
metadata-only loop therefore cannot be presented as warm-preparation latency.
The helper measurements do not attribute the remaining full-sync time to one cause.

Do **not** remove snapshot hashes or skip transactional checks merely to improve
this fixture. They protect concurrent mutation/rollback safety. Profile representative
larger packs, slower disks and state durability separately before proposing a
reviewed optimization. Digest/metadata caches need explicit invalidation and trust
contracts, not size/mtime-only correctness assumptions.

Java remains an unmeasured gate. Current `ensure_runtime` reads the installed marker
and fetches Mojang's index before its matching-marker fast return; index failure
can fall back only when an installed marker and Java executable exist. Version
detection, metadata latency, library resolution, extraction and process spawn are
not exercised by this sync fixture. No Java caching change or offline guarantee
is introduced. A bounded injectable-origin runtime fixture and controlled process
probe would be separate work requiring compatibility review.

## Validation

- Ordinary profile fixture: one pass, no failures.
- Explicit controlled profile: one pass, five samples with all invariants met.
- Updated core library suite: **85 pass, 0 fail, 2 explicitly ignored benchmarks**;
  no desired behavior is ignored. Other package/online results are the historical
  75% milestone evidence, not newly rerun in this slice.
- Scoped new-file rustfmt check passes. No dependency/lockfile changes or runtime
  data migration is part of this profile.
- `preparation-profile-review.md` independently accepts the bounded fixture and
  reruns its ordinary test: one pass, zero failures. It does not rerun the timing
  benchmark or grant native/Java/performance credit. Its limitations include
  serial server behavior, probe/cache ordering and narrow save/state coverage.
- Boundary/identity checks, tracked secret scan (six reviewed / zero unreviewed),
  separate nonignored-addition scan (126 files / zero findings), local Markdown
  checks (168 files / 101 links) and `git diff --check` pass. A fresh origin fetch
  still has zero divergent commits. No changes are committed.

The fixed ledger stays **57/76 (75%)**. Full VEL-014 profiling, evidence-led runtime
optimization, matched final measurements and native acceptance remain open.
