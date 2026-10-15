# Preparation lifecycle regressions

Date: October 12, 2026.

## Scope

Added `launcher/src-tauri/src/preparation/tests.rs`, containing thirteen unit tests
against the production `PreparationManager` and `mutation_lease` API. No mirrored
manager, application-state fixture, new dependency, credential access, player
data, live store, native IPC, game process, or deployment is used.

Tasks are real Tokio tasks. A oneshot confirms startup before lifecycle assertions;
`Notify` controls successful completion, and joins establish actual completion or
abort delivery. Tests do not use timing sleeps to decide correctness.

## Coverage

- Wrong-instance cancellation preserves the active owner and unfinished task.
- Matching-owner cancellation aborts an unfinished task, marks cancellation, and
  retains ownership and busy admission before joining and until explicit finish.
- Repeated cancellation is a no-op, including after the aborted task joins.
- Normal completion and explicit finish clear ownership; repeated finish fails.
- Premature finish rejects an unfinished matching owner without clearing its slot;
  finish succeeds after the worker joins.
- A stale operation ID cannot finalize a new owner for the same instance.
- Busy admission does not invoke the supplied spawn closure.
- A completed task handle remains busy until explicit finalization, after which
  another instance can be admitted.
- Empty and completed-task cancellation are no-ops without changing ownership.
- Mutation leases exclude other leases and direct resource-lock users.
- A busy registry rejects mutation after the resource guard is dropped, including
  after cancellation and joining, until explicit finalization.
- A completed but unfinalized registry rejects mutation without leaking the
  resource lock; mutation becomes available after finalization.

## Validation

The standalone test file was formatted successfully with:

```sh
rustfmt --edition 2021 launcher/src-tauri/src/preparation/tests.rs
```

Targeted test command:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher --lib preparation::tests
```

At initial inspection, the production module and its library registration were
absent. They became available during a bounded availability wait, including
`#[cfg(test)] mod tests;`. The first targeted run passed all twelve original tests
(25 filtered out). After the requested premature-finish regression was added, the
targeted command was rerun: **13 passed, 0 failed, 0 ignored, 25 filtered out**.
The final run compiled successfully in 6.37 seconds and reported test execution
completed in 0.00 seconds. No full-suite run was duplicated.

The standalone `rustfmt --check` and `git diff --check` also passed. Only the test
file and this report were edited for this task; production changes were provided
separately. No commits, branches, pull requests, or deployments were created.

## Remaining gates

These tests do not certify native IPC command wiring, account/state integration,
game-process launch or termination, or draining background I/O after an abort.
An aborted Tokio task is not proof that spawned subprocesses or independent
blocking/background work have stopped. Native acceptance and process/background
I/O checks remain separate gates. Packaging and releases remain manual.
