# Launcher operations regressions — initial Agent B

Work session: October 11–12, 2026. Local checkout only. No production behavior change.

Historical initial-audit evidence: the copied command fixture described here was
removed during the preparation-ownership implementation and replaced by 13 direct
production-manager/lease tests. The game payload fixtures remain. See
`preparation-implementation.md` for current behavior and test counts; paths and
line numbers below describe the initial snapshot, not the current implementation.

## Scope

Read root `AGENTS.md` and the four mandated repository docs. Inspected host
preparation admission/cancellation, run registration/kill/exit and event payloads.
No core tests or frontend audit files were changed. No application startup,
credential-store opening, player data, network service, Java process, branch,
commit, PR, deployment or release is part of these tests.

Changed paths:

- `launcher/src-tauri/src/commands.rs`: inline `#[cfg(test)]` operation fixture,
  eight current-behavior tests and two ignored desired-invariant reproductions.
- `launcher/src-tauri/src/game.rs`: inline `#[cfg(test)]` payload/run-removal
  checks, three current-behavior tests and one ignored desired-invariant reproduction.
- `reports/operation-manager-design.md`: typed manager proposal and integration gates.
- `reports/launcher-operations-regressions.md`: this evidence and validation report.

## Coverage and limitations

The command tests use real Tokio task/abort handles, current-thread scheduling,
oneshot kill channels and the production `RunningGame`/`StatePayload` types, with
synthetic instance/run strings. They mirror the current host registry decisions
in a test-only fixture. They do **not** invoke the actual Tauri commands or execute
install/sync. Changing production predicates will not automatically change this
fixture; keep these reproductions paired with source review until a separately
approved extraction allows direct manager tests. No timers or sleeps are needed:
the startup oneshot establishes that preparation has begun, and the current-thread
runtime makes abort-before-poll admission deterministic.

The payload tests serialize the actual production payload types, including the
unchanged core progress event forwarded by the host (no core file edits). The run-removal
test mirrors the watcher's retain expression; it does not run a process watcher
or assert frontend behavior. The tests characterize weaknesses, not endorse them.

| Test suffix (`operation_…`) | Expected default-suite behavior |
| --- | --- |
| busy_slot_blocks_launch_and_repair_across_instances | A live preparation blocks both operation kinds globally. |
| finished_slot_allows_relaunch_without_explicit_clear | A completed handle can remain stored without preventing admission. |
| wrong_instance_cancel_aborts_the_global_slot | Cancelling beta removes/aborts the existing slot and labels idle as beta. |
| cancel_reopens_admission_before_abort_finishes | The slot is available before the old task is polled to cancellation completion; replacement remains owned after old join. |
| delayed_cancel_aborts_same_instance_replacement | After the first task is cancelled and joined, an indistinguishable late cancel removes its replacement. |
| cancel_without_active_task_still_emits_idle | Even an empty slot produces a caller-labelled Cancelled/idle payload. |
| repair_conflict_is_instance_specific_but_launch_is_allowed | Same-instance repair is blocked by a run; another-instance repair and same-instance launch are allowed when preparation is free. |
| kill_targets_run_identity_and_is_idempotent | Killing one run leaves another same-instance run's sender untouched; unknown/repeated kills do nothing; watcher owns removal. |
| exit_payload_preserves_run_identity | Serialized exit retains instance/run/code/crash fields. |
| progress_payload_has_no_owner_identity | Actual forwarded progress schema lacks instance/run/operation/sequence fields. |
| old_exit_removes_only_its_run_but_idle_has_no_identity | Removing old run preserves new run; generic idle lacks run/operation identity. |

Three tests are ignored with explicit reasons, and intentionally assert desired
invariants that the current fixture/payload violates:

- `operation_wrong_instance_cancel_should_preserve_active_preparation`:
  cancellation must match the operation owner.
- `operation_cancel_should_block_relaunch_until_old_task_finishes`:
  cancellation must retain the lease through cleanup.
- `operation_terminal_state_should_identify_the_finished_run`:
  a delayed terminal state must identify its owner.

## Findings with source anchors

1. **Cancel identity mismatch:** `state.rs:54` lacks owner metadata;
   `commands.rs:497` ignores instance ownership when taking the abort handle, then
   emits idle for the supplied instance. This also allows a delayed cancel request
   from an earlier preparation to abort a replacement using the same global slot.
   The fixture demonstrates the ownership gap, not actual IPC invocation.
2. **Rapid relaunch:** `commands.rs:498` releases the slot before abort completion;
   `commands.rs:481` permits admission when no slot exists. The deterministic
   fixture proves admission-before-task-completion. It does not prove actual
   filesystem corruption or that any detached subtask is running.
3. **Repair conflict:** `commands.rs:477` rejects repair for tracked same-instance
   runs, but allows another instance when preparation is free. Global preparation
   exclusion at `commands.rs:480` is separate. `game.rs:138` installs before checking
   already-running at `game.rs:141`; sync/options are guarded at `game.rs:153`, but
   server-list injection at `game.rs:165` is not. This is a write-safety review gate,
   not a demonstrated repair race or a new production fix.
4. **Stale events:** `game.rs:53` makes idle/preparing/error uncorrelated;
   `game.rs:88` forwards progress without host identity. `game.rs:355` correctly
   removes only the exiting run, but `game.rs:370` unconditionally emits generic
   idle, even if another same-instance run remains. `game.rs:314` and `game.rs:333`
   send uncorrelated log batches. Backend ambiguity is confirmed; frontend
   susceptibility is deliberately left to its separate audit owner.
5. **Positive run identity behavior:** `state.rs:39`, `commands.rs:505`, and
   `game.rs:365` support exact run-targeted kill and correlated exit. Preserve these
   semantics in a typed manager; preparation cancellation is a separate lifecycle.
6. **Destructive/shared-write gate (independent review E6):**
   `commands.rs:577` checks tracked games before deletion but not preparation;
   `commands.rs:586` clears shared cache without ownership checks and suppresses
   errors. Admission/leases must include delete and cache clear, instance-path
   aliases, install/config writes and shared resources. No destructive command
   was executed or added to the fixture suite. Direct synthetic-store tests for
   delete/prepare and clear/download exclusion and partial-removal failures remain.
7. **Spawn-ownership gate (independent review E6):** `game.rs:241` spawns before
   the close-after-launch sleep at `game.rs:244`, before run registration;
   `crates/core/src/launch.rs:266` has no configured kill-on-drop. Abort completion
   does not certify child cleanup. Ordinary registration at `game.rs:251` has no
   intervening await after spawn; log-reader/watch tasks at `game.rs:280` and
   `game.rs:294` are separate owners. Require synthetic subprocess handoff/exit/kill
   tests and native close/hide/minimize acceptance, preserving intended game
   survival after launcher close. No subprocess or Java launch was executed here.

Short source paths in this section are relative to `launcher/src-tauri/src/`;
the explicitly named core path is inspection only. `reports/independent-review.md`
was read on October 12; it does not approve a production implementation.

## Validation

Executed from the repository root using Rust/Cargo 1.98.1. The native macOS host
test build completed; it did not start a Tauri application or game. Other agents'
concurrent core/frontend/report changes were left untouched. Main owns the broader
launcher and source checks; no duplicate full-suite build was launched by Agent B.

| Exact command | Actual result |
| --- | --- |
| `rustc --version` | `rustc 1.98.1 (48a229cea 2026-09-01)` |
| `cargo --version` | `cargo 1.98.1 (797e8a9bc 2026-08-05)` |
| `cargo test --locked -j 1 -p velora-launcher --lib operation_regression_tests` | Initially queued on the shared build lock; stopped with exit 143 to align with the other agent's debug-zero profile. No test result from this attempt. |
| `CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher --lib operation_regression_tests` | Exit 0. Build finished in 8m 16s. **11 passed, 0 failed, 3 ignored, 11 filtered out**; test execution 0.00s. Shared build lock released. |
| `CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher --lib operation_regression_tests -- --ignored` | Queued but stopped with exit 143 before acquiring the build lock, following main's no-duplicate-build coordination. No test result from this attempt; used the freshly built binary below instead. |
| `target/debug/deps/velora_launcher_lib-8a1597d2e7c1f775 operation_regression_tests --ignored` | Exit 101, intentionally. **0 passed, 3 failed, 22 filtered out**. All three desired-invariant reproductions fail for the documented reason; these remain ignored in the normal suite. No Cargo lock/build required. |
| `rustfmt --check --edition 2021 --config skip_children=true launcher/src-tauri/src/commands.rs launcher/src-tauri/src/game.rs` | Exit 1 for existing full-file formatting differences. Separate HEAD baseline checks also exit 1; production formatting was not changed. |
| `rustfmt --check --edition 2021 --config-path rustfmt.toml /tmp/velora-agent-b-operation-checks/commands-operations.rs /tmp/velora-agent-b-operation-checks/game-operations.rs` | Exit 0 for the final extracted inline test modules. An intermediate game-module check found a new wrapping difference; fixed only that test expression before this final check. |
| `node scripts/check-boundaries.mjs` | Exit 0: compatibility identities unchanged; reusable Rust boundaries and independent Infra workspace verified. |
| `git diff --check` | Exit 0. |
| `git diff --no-index --check /dev/null reports/operation-manager-design.md` | Exit 1 for new-file differences; no whitespace diagnostics. |
| `git diff --no-index --check /dev/null reports/launcher-operations-regressions.md` | Exit 1 for new-file differences; no whitespace diagnostics. |

The explicit ignored run reported these exact assertion sites:

- `launcher/src-tauri/src/commands.rs:1852`: beta cancellation removed alpha's slot.
- `launcher/src-tauri/src/commands.rs:1863`: relaunch admission was not blocked.
- `launcher/src-tauri/src/game.rs:458`: terminal `run_id` serialized as null.

The built test binary path above is the exact artifact from this run, not a stable
future command path. Rebuild with the targeted Cargo command to obtain its current
path, then pass `operation_regression_tests --ignored` to explicitly reproduce.

The final test-only scope audit/extraction command was:

```sh
node <<'NODE'
const fs = require('fs');
const cp = require('child_process');
for (const name of ['commands', 'game']) {
  const source = `launcher/src-tauri/src/${name}.rs`;
  const current = fs.readFileSync(source, 'utf8').replace(/\r\n/g, '\n');
  const baseline = cp.execFileSync('git', ['show', `HEAD:${source}`], { encoding: 'utf8' }).replace(/\r\n/g, '\n');
  const start = current.indexOf('#[cfg(test)]\nmod operation_regression_tests {');
  const end = name === 'commands' ? current.indexOf('#[cfg(test)]\nmod casino_path_tests {', start) : current.length;
  if ((current.slice(0, start).trimEnd() + '\n\n' + current.slice(end)).trimEnd() !== baseline.trimEnd()) throw new Error(`Non-test change: ${source}`);
  fs.writeFileSync(`/tmp/velora-agent-b-operation-checks/${name}-operations.rs`, current.slice(start, end).trimEnd() + '\n');
  console.log(`Test-only scope verified: ${source}`);
}
NODE
```

Result: exit 0 for both paths. Removing only the added modules leaves HEAD content
unchanged after CRLF normalization; a few patch-context line endings changed, not
production statements or behavior. Baseline formatting checks used HEAD copies
in `/tmp/velora-agent-b-operation-checks/`, not edits to production source.

Native IPC, install/sync quiescence, destructive-command exclusion/recovery,
subprocess spawn ownership and frontend stale-event rejection remain unexecuted
integration gates. Fixture passes and deliberate ignored failures do not establish
these production guarantees. See the typed design note and independent review E1,
E2 and E6 before authorizing any implementation or release.
