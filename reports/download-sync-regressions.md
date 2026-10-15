# Agent C: download/sync regression evidence

Started 2026-10-11; validation continued 2026-10-12, America/New_York.
All data are synthetic. No live instance directories, credentials, player stores,
remote download endpoints, branches, commits, PRs or deployments were used.
Only Agent C's changes to `crates/core/src/http.rs`, `crates/core/src/sync.rs`,
this report and download-sync-design.md belong to this assignment. Concurrent
launcher/Tauri edits and baseline reports are owned by the other agents.
Results describe the concurrent **working tree**, not pristine HEAD: other
agents were adding tests while these checks ran. Main separately reported its
core package run as 36 passed / 5 ignored plus 7 ignored online tests; that is
coordination evidence, not a second Agent C run or an online acceptance claim.

## Harness and scope

`crates/core/src/http.rs:75` adds a cfg(test)-only fixture/test module. An ephemeral
`127.0.0.1` TCP listener serves controlled HTTP/1.1 bodies, including incorrect
Content-Length and a stalled body. Clients disable proxies and have a two-second
timeout; servers abort on fixture drop. Stall fixtures delay the first body by
150 ms so aggregate progress crosses the existing 100 ms reporting throttle.
Cancellation synchronizes on received bytes/progress, not a guessed task sleep,
and waits for the aborted task to terminate. Temporary directories own all files.
Tests retain production retry timing (four attempts, about 5.6 seconds of backoff).

Core's HTTP re-exports exercise the actual platform download implementation;
no test-only replacement of production behavior, dependencies or public APIs.
`crates/core/src/sync.rs:158` adds synthetic state/file builders inside the
existing test module. No production behavior is changed. Existing tests remain.

## Passing additions

| Test / source evidence | Contract established |
|---|---|
| `http::regression_tests::checksum_mismatch_preserves_destination_and_removes_part`, `crates/core/src/http.rs:136` | Same-size wrong SHA-1 exhausts retries, returns a checksum error, preserves old destination bytes and removes its partial file. |
| `http::regression_tests::truncated_http_body_preserves_destination`, `crates/core/src/http.rs:149` | Server closes after 5 of 100 advertised bytes; even without a checksum, HTTP framing failure returns an error and preserves the old file. Does not promise removal of the partial artifact. |
| `http::regression_tests::interruption_preserves_destination_and_retry_replaces_partial`, `crates/core/src/http.rs:173` | Abort after receiving partial bytes leaves the old file; a fresh invocation publishes verified bytes and removes the sibling partial. |
| `sync::tests::failed_download_keeps_revision_state_dropped_files_and_unknown_files`, `crates/core/src/sync.rs:181` | Failed ordinary single-file update leaves exact persisted state bytes/revision, existing managed bytes, dropped config and unknown player mod unchanged. |
| `sync::tests::interrupted_sync_does_not_advance_persisted_revision`, `crates/core/src/sync.rs:196` | Abort after aggregate download progress leaves exact persisted state and old managed/dropped/unknown files unchanged. |
| `sync::tests::successful_update_persists_sorted_ownership_and_is_idempotent`, `crates/core/src/sync.rs:224` | Successful multi-file update publishes bytes, removes dropped ownership only, preserves unknown mod/config, records sorted ownership/revision/epoch and performs a no-network second check. |
| `sync::tests::deep_verification_repairs_truncation_at_same_revision`, `crates/core/src/sync.rs:251` | Explicit deep verification repairs a truncated file even at the same revision. |
| `sync::tests::successful_clean_update_preserves_non_mod_user_files_and_records_epoch`, `crates/core/src/sync.rs:265` | Successful clean plus download quarantines player mod bytes, removes old managed files/cache, preserves synthetic world/screenshot/options/config and does not clean again at the same epoch. |

## Ignored known-gap contracts (not ordinary failures)

Each test has an explicit `#[ignore = "known gap: ..."]` reason. These assert
desired future behavior, not current guarantees. Explicit opt-in execution
confirmed all five fail for the intended gap; ordinary runs skip them.

| Test / source evidence | Observed gap on explicit execution |
|---|---|
| `http::regression_tests::size_mismatch_must_not_publish_even_with_matching_checksum`, `crates/core/src/http.rs:188` | Correct SHA-1 for 5 bytes but manifest size 99 succeeds and replaces old bytes with `valid`. Size is not checked before publication. |
| `http::regression_tests::interruption_must_remove_partial_artifact`, `crates/core/src/http.rs:201` | Task cancellation leaves `managed.jar.part`. Destination remains unpublished; cleanup is not automatic. |
| `sync::tests::fast_path_must_repair_truncated_file`, `crates/core/src/sync.rs:292` | Unchanged-revision fast path retains `cut` rather than repairing `previous`. |
| `sync::tests::failed_batch_must_restore_already_published_files`, `crates/core/src/sync.rs:305` | With concurrency 1, first replacement publishes `updated`, second fails checksum; state bytes stay old but the first file is not restored. |
| `sync::tests::failed_clean_update_must_restore_previous_tree`, `crates/core/src/sync.rs:319` | Replacement checksum failure leaves old state bytes but the previous managed file is missing after pre-download clean. The test fails there before its later mod/cache restoration assertions. |

Use **`--lib` plus the specific module filter** when running these gap tests.
Do not use an unfiltered package-wide `-- --ignored`: existing online integration
tests intentionally contact remote services and are outside this assignment.

## Commands and observed results

All commands run from the repository root with the existing root lockfile.
`rustup show` reported active installed `1.98.1-aarch64-apple-darwin`;
`cargo --version` reported 1.98.1 and `rustc --version` reported 1.98.1.
Process inspection found no active rustup install. No installation was started.
Cargo waited for shared target-directory locks held by the coordinating/core and
launcher agents; no duplicate toolchain install, alternate target tree, or
termination of another agent's build was used.

| Command | Result |
|---|---|
| `CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher-core --lib` | Exit 0; 36 passed, 0 failed, 5 ignored; 41 total. Build/lock wait 1m25s, tests 5.62s. |
| `CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher-core --lib http::regression_tests` | Exit 0; 3 passed, 0 failed, 2 ignored, 36 filtered. Shared build lock/rebuild wait 5m46s; tests 5.62s. |
| `CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher-core --lib sync::tests` | Exit 0; 9 passed, 0 failed, 3 ignored, 29 filtered; tests 5.62s. Includes four existing sync tests. |
| `CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher-core --lib http::regression_tests -- --ignored` | Exit 101, expected; 0 passed, 2 known-gap failures, 39 filtered; 0.15s. Incorrect manifest size replaces old bytes; cancellation retains partial. |
| `CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher-core --lib sync::tests -- --ignored` | Exit 101, expected; 0 passed, 3 known-gap failures, 38 filtered; 5.62s. Fast-path truncation, published sibling and destructive pre-download clean reproduced. |
| `target/debug/deps/velora_launcher_core-8f3569f1cdebf1a3 http::regression_tests --ignored` | Exit 101, expected; 2 known-gap tests failed, 39 filtered. Ran the already-built binary while targeted Cargo checks waited on another agent's build. |
| `target/debug/deps/velora_launcher_core-8f3569f1cdebf1a3 sync::tests --ignored` | Exit 101, expected; 3 known-gap tests failed, 38 filtered; 5.62s. |

All four targeted Cargo commands completed after the shared lock was released;
ordinary exits were 0 and explicit known-gap exits were 101. Direct-binary hash
is local evidence only, not a stable build command. All five gaps remain ignored
with reasons; no ordinary tests were disabled to hide failures.

After explicit gap reproduction, reran
`CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher-core --lib`:
36 passed, 0 failed, 5 ignored, test duration 5.62s. Final scoped
`git diff --check -- crates/core/src/http.rs crates/core/src/sync.rs reports/download-sync-design.md reports/download-sync-regressions.md`
returned exit 0 with no whitespace diagnostics. Ignoring line-ending differences,
the core diff contains only additions (HTTP 135 lines, sync 176 lines); no
production statements were changed. Existing CRLF boundary lines touched by
the patch are line-ending-only differences. No formatter was run over production
code or other agents' files.

## Limits and next gates

- Ordinary green tests do not establish batch rollback, safe failed cleaning,
  manifest-size enforcement or crash-safe state writes. The five ignored tests
  intentionally prevent those claims.
- Interruption here means aborting an in-process Tokio task after controlled
  body receipt. It is not process kill, OS crash, power loss or launch/sync
  interprocess concurrency acceptance.
- Fixture update files use noncolliding managed destinations. Unknown-user path
  collisions, duplicate manifests, symlink/reparse escapes, disk-full errors,
  permission failures, state-write errors/corruption and quarantine collisions
  remain separate tests/design gates; no claim of coverage for them.
- Independent review E4/E5 (`reports/independent-review.md`) was read. Future
  acceptance explicitly includes normalized ownership/destination aliases,
  symlink/reparse containment for both manifest and persisted managed paths,
  reserved state/transaction paths, destination/temp and file/directory conflicts,
  suppressed enumeration/create/move/delete/cache errors, and newly managed
  destinations overwriting unknown user files. Design gates require preflight
  failure without mutations and recoverable old state/tree agreement; this patch
  adds no production fix or executed coverage for those reviewer findings.
- Tests ran on macOS/aarch64 only. Windows locked-file replacement and Linux/
  macOS durability behavior require separate validation with synthetic stores.
- Staging/journal/rollback implementation and remaining release gates are in
  download-sync-design.md. No such production implementation ships in this patch.
