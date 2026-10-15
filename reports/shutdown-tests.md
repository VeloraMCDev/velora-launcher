# Bounded shutdown regressions

Implemented on October 13, 2026, directly in the shared checkout. Changes are
limited to the new `launcher/src-tauri/src/shutdown/tests.rs`, appended tests in
`launcher/src-tauri/src/preparation/tests.rs`, and this report. No Cargo inputs,
production code, branches, commits, credentials or installed/player stores changed
by this test task.

## Implemented coverage

Eight shutdown tests cover admission closure, readiness before/after all permits
drain, idempotent begin, ordinary accepted task completion without automatic abort,
started and unpolled cancellable task ownership until cancellation is processed, registration
after begin, multiple independently registered waiters across nonfinal/final drops,
completion before first waiter poll, and blocking writer ownership after its async
awaiting wrapper is cancelled.

The blocking fixture uses a std thread, a channel release gate and a UUID-named
temporary synthetic root with RAII cleanup. The write must complete before its
permit drops and shutdown becomes ready. It never constructs `AppState` or opens
the keychain, game, network, launcher settings or real player files.

Four appended preparation tests cover shutdown with no active work, repeated
shutdown during active work, cancellation/finish ownership before and after join,
shutdown after normal completion, refusal to invoke new spawn closures before and
after finalization, and mutation lease rejection without leaking the resource lock.
An already-issued resource lease remains owned until explicitly dropped.

New async tests use a current-thread runtime, explicit channel/notification gates,
and five-second timeout bounds on awaited completion. Unpolled abort assertions
run without yielding between spawn and cancellation. Waiter wakeups are inspected
with manually polled futures and independent wake counters, rather than sleeps.
The std thread has a bounded receive deadline and is joined after completion.
Timeouts are failure guards, not timing-performance assertions.

## Validation and remaining gates

Both Rust test files were parsed with `rustfmt --edition 2021 --emit stdout`,
discarding the formatted output without modifying existing test formatting.
This checks parsing only, not compilation, types, runtime behavior or format compliance.

No Cargo command or test suite was run: the main agent is concurrently integrating
production files and explicitly owns validation after its stable message. These
are implemented tests, not reported passing tests. Main should run focused
`shutdown::tests` and `preparation::tests` launcher library tests against the stable
integration, then its relevant broader checks. Native close-window orchestration,
real credential storage, release packaging and supported-OS shutdown acceptance
remain outside these isolated synthetic unit tests.

## S1 utility cache cancellation follow-up

Added three tests in the disjoint new file
`crates/platform-utils/src/http/cache_cancellation_tests.rs`. Main owns production
`http.rs` changes and its test-module registration; this worker did not edit it.

- A two-worker Tokio test calls actual `get_json_cached_with` against a loopback
  HTTP 200 fixture. An injected synchronous writer announces entry and waits on a
  five-second std-channel gate before calling actual `write_cached_json`. Aborting
  the awaiting task cannot finish it or drop a test-local RAII owner before release;
  the cache must remain absent until release and contain the exact response bytes
  afterward. The joined task may finish successfully or report cancellation, but
  must not panic, and its owner must drop only after the inline write completes.
- Public `get_json_cached` creates a nested cache online, then serves that exact
  cached JSON after the one-request loopback server has shut down.
- Public `get_json_cached` still returns successful online JSON when a regular file
  blocks cache-parent creation, leaving that synthetic sentinel unchanged.

All files use existing `tempfile` cleanup. Clients disable proxies locally and have
two-second request deadlines; server/request/completion awaits have five-second
guards. No global hooks, environment changes, dependencies, host types, live
stores or deployment changes are involved. The cancellation test models ownership
locally; it does not certify the host's complete preparation/shutdown wiring.

Validation: the new Rust file is parsed and formatting-checked with rustfmt only.
No Cargo command or tests were run pending main's stable integration message.
Main must register this file and execute the utility cache tests before treating
the S1 regression as passing.

## Lead execution after integration — October 13, 2026

The lead subsequently executed all worker fixtures: full utility suite **13
passed**, including the three cache tests; host suite **92 passed**, including
eight tracker and four preparation tests plus lead-authored two account and three
installer regressions. The post-format host rerun also passes 92. Full stable
host/core/utility/shared sweep passes 256, with three timing profiles and seven
existing online tests ignored. This supersedes only the author-time execution
status above; it does not attribute Cargo runs to the worker or add native credit.
Logs: `/tmp/velora-revitalization-final/shutdown-final-rust.log` and
`/tmp/velora-revitalization-final/shutdown-final-host-format-tests.log`.
