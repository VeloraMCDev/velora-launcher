# Independent bounded manifest-denial review

October 14, 2026. **Verdict: accept the bounded process-local offline-cache denial fence.** No blocking defect found in the scoped implementation. This is not a complete authentication/cache security acceptance or a new checkpoint. The fixed ledger remains **62/76 (81.58%)**; existing native, supported-OS, upgrade and release gates remain open. Actual billed USD is unavailable.

## Scope and independence

Read root `AGENTS.md`, `MONOREPO.md`, `docs/BOUNDARIES.md`, `docs/VALIDATION.md` and `docs/DEPLOYMENT_TARGETS.md`, plus historical `reports/manifest-cache-authority.md` and `reports/authority-accessibility-review.md`. No nested `AGENTS.md` was found under launcher or reports.

Reviewed the new `launcher/src-tauri/src/game/manifest_denials.rs`, its production integration in `game::instance_manifest`, the `AppState` field/default, and the two new production-caller tests in `game/manifest_cache.rs`. Adjacent cache authority/persistence policy and synthetic secret-store setup were read only as necessary context. Whole-file hashes below identify bytes, not an independent review of every unrelated change in those files.

After the main agent explicitly confirmed Cargo idle and source frozen, this reviewer independently repeated the focused synthetic suite. Whole-host and non-test check evidence below remains the main agent's completed retained logs, inspected independently, not independent execution. No source or ledger edits, native keychain calls, real runtime-store access, external network requests, deployment or purchases were performed. Test activity was limited to disposable synthetic stores and loopback fixtures. The only authored repository file is this report.

## Findings

- **Mark-before-invalidate closes the historical same-state retry gap.** `deny` acquires the mutex, installs a fresh UUID for the instance cache path, and only then invokes persistent invalidation. An invalidation error does not remove the entry. The production caller uses this path for typed 401/403/404 statuses before recovery; transport fallback subsequently invokes the gated read. Thus restoring old matching provenance after a failed invalidation cannot reopen offline access through the same `AppState`.
- **Active denial generations fence stale publication.** The caller observes the generation before fetching. `refresh` compares that captured generation with the current entry under the same mutex, before invoking publication. Both a request captured with no denial and one captured with an older denial are rejected for cache publication while a newer denial remains recorded. Each denial replaces the UUID, rather than reusing a boolean tombstone.
- **Successful cache publication precedes clearing.** A matching-generation refresh executes `publish` under the gate lock and removes the entry only after publication returns success. Publication failure leaves the denial intact. The caller deliberately still returns a valid online response if publication is rejected or fails; this gate is an offline-cache fence, not cancellation of an already successful online fetch.
- **Read and mutation are serialized.** `read` keeps its mutex guard in scope while invoking the synchronous cache read. A denial cannot be installed between the in-memory check and that read. No gate guard spans the asynchronous HTTP fetch. All instance paths share one mutex, so slow synchronous cache I/O can serialize unrelated instances; this is a conservative correctness tradeoff, not a throughput claim.
- **Persistent compatibility remains unchanged in this slice.** The existing cache destination, authority fingerprint and version-1 flattened manifest format are retained; the new map stores only paths and random UUIDs in memory. `AppState::try_new` creates an empty map. No persistent denial journal, credential-store identity, signing identity, protocol identity, lockfile or database migration is introduced by the reviewed mechanism.

The historical report's same-process invalidation-failure limitation is addressed for subsequent gated operations sharing this state. Its restart/durability and broader authorization limitations are not superseded.

## Validation evidence

Independent repeat after explicit idle authorization:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher --lib game::manifest
```

**19 passed, 0 failed, 119 filtered out**, reusing existing compiled artifacts. Retained independent log: `/tmp/velora-manifest-denial-independent-20261014.log`, SHA-256 `0d491f65fe92dc32f55eed73b3fda358e9b994c088436a8ff90cf6b5b40a4962`. All four source hashes below were rechecked after this repeat and remained unchanged. No full-host or non-test check was independently repeated.

Inspected completed artifacts under `/tmp/velora-revitalization-20261014/`:

| Main-agent artifact | Observed result | SHA-256 |
| --- | --- | --- |
| `manifest-denial-focused.log` | 19 passed, 0 failed, 119 filtered out | `0068edc112ad36274a93ca0e241be57922ea9d4f098ad73a6f9086f89e8b7447` |
| `manifest-denial-host.log` | Whole launcher library: 138 passed, 0 failed | `b8151df9e63f5a7dfb645a119cabb87ac1f4b8d9814ba2c119b8e142c3d6c310` |
| `manifest-denial-check.log` | Launcher non-test dev check completed successfully | `7467246da64f770ab659018c98e635e76c6e2b9c43209aef21e4ee6244330ba8` |

The focused log includes both denial-module tests and both new caller regressions:

- `failed_invalidation_fences_only_its_instance_until_successful_refresh`: callback failure retains the fence; blocked reads do not invoke their callback; another path stays readable; failed publication retains denial; successful publication permits reads.
- `success_started_before_new_denial_cannot_publish_or_clear_it`: deterministic callback-order test exercises both initially absent and existing denial generations, confirms stale publication callbacks are not invoked, and permits refresh with the current generation.
- `actual_denial_fences_restored_cache_after_invalidation_write_failure`: synthetic 403 response with a deliberately rejected symlink cache destination, restoration of the old authority-bearing regular file, then refused-network retry through the production caller remains denied.
- `actual_successful_refresh_after_denial_restores_offline_access`: synthetic 200/401/200 responses through the production caller, then offline reuse, verifies the entry exists after denial and is cleared after successful refresh.

The failure fixture forces invalidation rejection through the persistence helper's destination validation; it does not simulate every write, rename or fsync failure. The helper callback test supplies a generic invalidation error. Existing caller tests retain 401/403/404 status coverage. Unit-test secrets use disposable generated fallback keys instead of the native keychain; loopback fixtures use synthetic account/token data. The independent focused repeat executed these fixtures successfully.

## Remaining limits and gates

1. **State-local, not restart durable.** A new process or independently constructed `AppState` starts with an empty gate. If persistent invalidation failed, old matching cache provenance can still be accepted after restart. There is no cross-process coordination, persistent tombstone, signed lease, TTL or offline revocation discovery.
2. **Not a total response-order protocol.** Once a denial has been successfully refreshed and removed, an older request that originally observed `None` can again match `None` and publish. The UUID check prevents overwriting/clearing an active newer denial, not all stale publication after that denial has been legitimately cleared. Concurrent account/settings changes and token replacement still lack credential epochs or cancellation. No broader stale-response freshness guarantee is accepted.
3. **Online use is not fenced.** A generation-mismatched successful response is returned to its caller even though it cannot publish or clear an active denial. This is consistent with the bounded offline-cache requirement; it does not establish cancellation of in-flight launch preparation after a later denial.
4. **Path-local and conservative.** Entries are keyed by the existing lexical instance cache path, not canonical inode identity or authority. Changing accounts/Panels does not by itself bypass an active entry for that path. Alias/cross-state/hostile ancestor-swap protection is not established. Existing authority matching still governs disk-cache reads.
5. **Concurrency evidence is bounded.** Generation tests model deterministic interleavings, not simultaneous production HTTP requests. No threaded read-lock contention regression, stress test, native filesystem/keychain acceptance, power-loss proof or supported-OS matrix was independently executed. Mutex poisoning still panics via the existing `unwrap` style rather than recovering to permissive access.

No automatic checkpoint credit, deployment authorization or expansion of the historical review's accepted scope follows from this verdict.

## Reviewed SHA-256 snapshot

| File | SHA-256 |
| --- | --- |
| `launcher/src-tauri/src/game/manifest_denials.rs` | `5d1dbfe1efb9b404a0cc522b283248036b5e26bb5440a16610105898adf3074e` |
| `launcher/src-tauri/src/game.rs` | `08fa90465fa43f14bf30d54bdfe7ab0d9668df8bb04ff4562a39b866842ef106` |
| `launcher/src-tauri/src/state.rs` | `d1bd284fee76b25ec80a62a6a4a55a0d9994db6ece593fbc2b1b853e84b6b61e` |
| `launcher/src-tauri/src/game/manifest_cache.rs` | `9aafadeaef1553d99d0bfdb5a0cfffc84f41ac16a89e16811121232eb826f322` |
