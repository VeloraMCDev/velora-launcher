# Independent synthetic preparation-profile review

Date: October 13, 2026.

## Decision and scope

**Bounded acceptance:** the new fixture exercises real core sync and records
descriptive synthetic timings without timing-based pass/fail criteria. No blocking
defect was found for that purpose. It is not a comparable install/Java baseline,
proof of a performance improvement, or native preparation acceptance.

This is the separately requested reviewer pass, not implementation self-review.
Read the supplied root AGENTS instructions, MONOREPO.md, docs/BOUNDARIES.md,
docs/VALIDATION.md and docs/DEPLOYMENT_TARGETS.md. No deeper AGENTS.md was found
under crates/core or reports. Consulted the checked-in master-plan completion
checklist for preparation-performance scope; no separately attached master-plan
document was available in this review context. No checkpoint credit, percentage,
or ledger change is assigned by this report.

Primary scope: crates/core/src/sync/performance_tests.rs and its test-only module
wiring at crates/core/src/sync.rs:21. Supporting inspection covered the real sync
path, its snapshot/publication/cleanup helpers, the loopback HTTP fixture and the
shared file verifier. This is not a general review of the other concurrent changes
in the already-dirty checkout. Only this report was authored; no production edits,
browser/native application use, live stores, branches, commits or deployments.

Reviewed source SHA-256 identities:

| File | SHA-256 |
| --- | --- |
| crates/core/src/sync/performance_tests.rs | 2b5a6ab923511f83ba0021a93415bc401d2368ecf9860285c78e893ec81e9ea6 |
| crates/core/src/sync.rs | 941e79106aabc21af9ffa417ad3fc8866b5d4c36b388f42f0a0bb609df49303c |

## Verified behavior

- **Test-only wiring:** `mod performance_tests` is guarded by `#[cfg(test)]`.
  The reused sync helpers and HTTP regression module are likewise test-only.
  The three-file fixture runs normally; the five-sample, 64-file profile is
  explicitly ignored unless requested. No new production execution path results
  from this wiring.
- **Isolation:** each sample owns a fresh `tempfile::TempDir`; all instance files,
  state and staging paths are beneath it. Content and filenames are synthetic.
  The server binds `127.0.0.1:0`; its client disables proxies and has a two-second
  request timeout. The server task is aborted on drop and the temporary directory
  is RAII-owned. No launcher layout discovery, credential store, Java process,
  installed game directory or live state is used.
- **Real cold/warm/deep/repair work:** all four phases call the production `sync`
  function, not a copied/mock sync implementation. Cold asserts every manifest
  file downloaded and no removals/cleaning. Same-revision warm and explicit deep
  runs assert zero downloads/removals/cleaning. Repair overwrites one managed file
  with different bytes of the same length, then asserts exactly one deep repair,
  no removals/cleaning, and exact restored payload bytes. The final untimed loop
  verifies every managed file with declared size and SHA-1.
- **Preservation and cleanup:** after each successful phase, assertions check
  revision 9, managed-file count, exact bytes of the unmanaged synthetic save,
  and absence of `.scopenet/staged-sync`. The real successful sync path includes
  state publication and `transaction::finish` cleanup before returning.
- **Timing scope:** each phase timer starts immediately before `sync` and ends
  after its awaited successful return, including sync's own snapshots,
  verification, HTTP work where needed, publication and staging cleanup.
  Directory/server/client/manifest setup, external preservation assertions,
  corruption setup and final all-file digest assertions are outside those phase
  timings. Separate metadata and fingerprint loop timers include their loop
  assertions; they are standalone probes, not a decomposition of the phase totals.
- **No thresholds or unsupported claims:** elapsed values are recorded, never
  compared to speed limits or ratios. The HTTP timeout is fixture safety, not a
  performance acceptance threshold. Output labels explicitly disclose a fresh
  temporary instance, loopback HTTP, uncontrolled OS cache and debug test build;
  the ignored-test annotation rejects native-preparation acceptance. Reviewed
  code contains no native/Java success or performance-gain claim.

## Measurement limitations and non-blocking gaps

1. **Configured concurrency is not demonstrated parallel server throughput.**
   `concurrency: 4` describes the downloader argument. The reused `LocalHttp`
   accept loop handles each connection and body sequentially, with connection
   close after each response. This server behavior and single-runtime synthetic
   workload can shape cold/repair time. Do not describe these samples as a
   four-way production download benchmark or infer concurrency improvements.
2. **Cold means empty managed instance, not cold OS/cache/network.** Every sample
   uses identical 16-KiB bytes for all files. Phases run in fixed cold/warm/deep/
   repair order; cache state is uncontrolled, there is no pre-change baseline,
   and these unoptimized test-build samples are not representative modpacks,
   real remote service latency or a statistically controlled comparison.
3. **Warm sync is not merely metadata checks.** Real sync fingerprints the managed
   tree and state before staging and rechecks snapshots before publication, even
   on the same-revision path. `metadata_checks_us` is only the later standalone
   shallow `file_ok` loop; `snapshot_fingerprints_us` is only one standalone
   per-file fingerprint loop, not all snapshot overhead within sync. Their
   difference cannot establish a production bottleneck or projected speedup.
4. **Preservation/state coverage is deliberately narrow.** The sentinel is a save
   outside managed mods. The fixture does not exercise an unmanaged mod alongside
   managed files, revision replacement/removal, clean-epoch changes, cancellation,
   failures/rollback, interrupted recovery or stale staging. State assertions
   check revision and managed count, not exact managed membership or clean epoch;
   all-file digest verification occurs after repair rather than immediately after
   each earlier phase. These are coverage limits, not evidence of failures, and
   must not be promoted to comprehensive sync safety acceptance.

## Execution evidence and remaining gates

The supplied fixture log
`/tmp/velora-revitalization-75/preparation-profile-fixture.log` reports one pass,
zero failures and 86 filtered tests. Independently reran only the authorized small
fixture command:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher-core --lib sync::performance_tests::profile_fixture_checks_real_sync_and_preserves_unmanaged_content
```

Reviewer result: exit 0; one passed, zero failed, 86 filtered; test execution
finished in 0.07 seconds. That harness duration is not a preparation baseline.

Although described as running at handoff, the supplied
`/tmp/velora-revitalization-75/preparation-profile-benchmark.log` was complete when
read: it contains one `SYNC_PROFILE` JSON record, five samples with 64 cold,
zero warm/deep and one repair download each, and a successful final test summary.
This is observed lead-run evidence, not an independently rerun benchmark; the
reviewer did not execute the ignored profile or any broader suite.

Acceptance is limited to the synthetic core-sync fixture and honest descriptive
measurement scope. Comparable install/Java profiles, evidence-led optimization,
before/after measurements, native launch/loader no-regression acceptance and the
release-OS/runtime matrix remain separate gates. Nothing here authorizes a
release, deployment, identity/storage change or additional completion credit.
