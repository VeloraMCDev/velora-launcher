# Launcher typed operation manager — initial Agent B design

Work session: October 11–12, 2026. Status: proposal only; no production implementation.

## Scope and constraints

The launcher host should own preparation lifecycle, IPC identity and game-process
tracking. Reusable install/sync libraries must remain independent of Tauri, gameplay
and the complete application host. This note does not propose changing core code,
frontend code, persisted instance IDs, filesystem layout, credentials, signing,
updater identities or deployment. Packaging/release/native acceptance remain manual.

Read: root `AGENTS.md`, `MONOREPO.md`, `docs/BOUNDARIES.md`,
`docs/VALIDATION.md`, and `docs/DEPLOYMENT_TARGETS.md`.

## Current evidence

| Concern | Evidence | Consequence |
| --- | --- | --- |
| Preparation identity | `launcher/src-tauri/src/state.rs:54` stores only an optional abort handle | No instance, operation kind, generation or join ownership is available to cancellation. |
| Admission | `launcher/src-tauri/src/commands.rs:476` checks a running-game list, then a single preparation slot | A busy preparation blocks all instances; same-instance repair is rejected while a tracked game exists. |
| Cancellation | `launcher/src-tauri/src/commands.rs:497` takes the slot, requests abort and emits idle using the caller's instance | A request for another instance cancels the active preparation; no completion barrier precedes new admission. |
| Preparation errors | `launcher/src-tauri/src/commands.rs:486` emits an unversioned error | A queued old event cannot be attributed to a preparation generation. |
| Progress | `launcher/src-tauri/src/game.rs:88` forwards the core event directly | The host adds neither operation nor instance identity. |
| State events | `launcher/src-tauri/src/game.rs:53` emits `run_id: None` | Preparation, repair completion, cancellation and generic idle/error lack generation identity. |
| Running games | `launcher/src-tauri/src/game.rs:250` creates a UUID and registers a kill sender | Running state has a run identity; killing targets that identity, not an instance. |
| Exit cleanup | `launcher/src-tauri/src/game.rs:355` removes one run, then `game.rs:370` emits generic idle | Another run for the same instance may remain; the idle event is not an authoritative instance summary. |

Line anchors describe the production source inspected in this checkout. Test-only
additions are after these production sections.

## Proposed typed host model

Use distinct noninterchangeable runtime identifiers. Operation IDs and event
sequence numbers are ephemeral; they do not replace any persisted instance ID or
existing Minecraft/credential/updater identity.

```rust
struct OperationId(uuid::Uuid);
struct RunId(String);
struct InstanceId(String);

enum OperationKind {
    Launch,
    Repair,
}

enum OperationPhase {
    Queued,
    Preparing,
    Cancelling,
    Succeeded,
    Cancelled,
    Failed,
}
```

These are illustrative design types, not compiled changes. Keep IPC serialization
explicit; never substitute bare boolean pairs for the public operation kind.

An operation record owns its ID, instance, kind, phase, event sequence, cancellation
signal and task completion observation. Internal task handles are not serializable.
A run record owns a separate run ID, parent launch-operation ID, instance ID,
kill sender and exit observation. Launch success means a process was registered
and handed to its watcher, not that its eventual gameplay session succeeded.
Repair success means preparation completed without starting a process.

## Admission and ownership

Initially preserve the current single global preparation limit. Do not introduce
parallel install/sync work or per-instance preparation concurrency without core
coordination and shared-cache write-safety validation.

- Serialize admission and reservation atomically in the manager. A task's actual
  startup must follow reservation; no separate check-then-insert race.
- Reject repair while any tracked run for that instance is alive. Reserve the
  instance write lease before starting repair; run registration and write-lease
  ownership must be coordinated through the same manager.
- Preserve multiple game runs per instance unless product requirements explicitly
  change. Validate their filesystem behavior separately: the current
  `game.rs:138` install call precedes the `game.rs:141` already-running check, and
  server-list writes at `game.rs:165` are outside the sync/options guard.
- Keep preparation ownership until completion/cleanup is observed, including
  cancellation and failure. A finished handle can be reaped lazily, but only the
  matching operation ID may clear its reservation.
- Avoid holding a synchronous mutex across await, event emission, process kill,
  installer work or file IO. Either use one owner task with request/reply messages
  or short atomic registry sections and an explicit completion channel.

### Destructive commands and shared-resource leases (reviewer gate E6)

`commands.rs:577` only checks tracked runs before deleting an instance directory;
it does not reserve against preparation. `commands.rs:586` clears the shared cache
without checking preparation and suppresses removal errors. A manager guarding
only launch/repair would leave these paths outside its safety boundary.

Require atomic admission and completion ownership for delete-instance and
clear-cache as well as preparation. Delete must exclude preparations, running
games and writers of that physical instance path; cache clearing must exclude all
workers using the shared cache. Inventory shared libraries, assets, versions,
Java runtimes and natives separately: a logical instance lock alone is not a
shared-resource lock. Existing path sanitization can alias logical instance IDs;
resource keys must account for actual resolved paths and detect collisions without
renaming installed directories. Establish a lock order or single reservation
owner to prevent deadlocks. Cancellation must not release these leases while
owned filesystem work remains active.

Return explicit destructive-operation success/failure, including partial removal
and cleanup/recovery failure, rather than unconditionally reporting success.
Preserve existing user confirmation semantics; define cancellation and recovery
for destructive operations separately from installation rollback. Required tests
include delete-versus-prepare, cache-clear-versus-download, aliased instance IDs,
locked files, permission failures and cleanup failure using synthetic stores only.
These are future direct-production tests, not covered by the current fixtures.

## Cancellation contract

`cancel(operation_id, expected_instance_id)` returns a typed outcome such as
Requested, AlreadyTerminal, NotFound or IdentityMismatch. A stale ID must never
cancel a replacement operation. If a transitional IPC endpoint accepts only an
instance ID, it must resolve the current owner and reject mismatches, not take the
global slot blindly. Treat repeated cancellation as idempotent.

Transition Preparing to Cancelling; keep the preparation/write lease occupied.
Emit Cancelled only after the worker and any owned subwork have stopped and
cleanup has completed. Prefer cooperative cancellation at safe boundaries;
an abort request alone is not a guarantee that detached tasks, blocking work or
partial filesystem writes have quiesced. Coordinate these guarantees with the
core owner before enabling rapid relaunch after cancellation. Retain a bounded
escalation policy, but do not silently unlock resources after a timeout.

Do not conflate cancel-preparation with kill-running-game. Define the process
spawn handoff explicitly: if cancellation wins before registration, dispose of
the spawned child before releasing the lease; if registration wins, return the
registered run identity and use the run kill command. The existing after-launch
close path (`game.rs:243`) needs native acceptance: it exits before ordinary run
registration and watcher setup.

### Spawn ownership (reviewer gate E6)

Independent review E6 was read and its spawn source checked without core edits.
`game.rs:241` obtains a child before the close branch awaits at `game.rs:244`;
that branch exits without registering a normal run. `crates/core/src/launch.rs:266`
configures capture/detachment but no `kill_on_drop` setting. Dropping a cancelled
preparation must not be represented as proof that its child was terminated.
In the ordinary branch, registration at `game.rs:251` has no intervening await
after spawn; do not incorrectly describe it as the same await race. Log readers
at `game.rs:280` and the watcher at `game.rs:294` are separately spawned tasks.

Use an explicit child-owner/handoff guard, tracked watcher completion and owned
reader shutdown. Decide cancellation before and after registration, child exit
during handoff, spawn failure and launcher shutdown separately. Preserve the
intentional close-after-launch behavior where Minecraft survives launcher exit;
blanket kill-on-drop would change that behavior and is not authorized here.
Synthetic subprocess tests and Windows/macOS/Linux native hide/minimize/close
acceptance must verify this contract before a production manager is accepted.

## Events and stale-state handling

Wrap existing core progress at the host boundary, leaving its contract unchanged:

```text
OperationEvent { operation_id, instance_id, sequence, kind, phase, payload }
RunEvent       { run_id, instance_id, parent_operation_id, sequence, payload }
```

Use typed phase/outcome values, a monotonic per-owner sequence and exactly one
terminal transition per owner. Reject obsolete completion mutations inside the
manager; event consumers additionally reject unknown IDs and older sequences.
Identity is needed even when callback/event delivery is delayed after cancellation.
Instance equality alone is insufficient for two launches of the same instance.

Emit a correlated run exit plus an authoritative instance snapshot of active run
IDs and active preparation, rather than unconditional idle. An old run exit must
not overwrite a newer running state. Log batches currently emitted at
`game.rs:314` and `game.rs:333` also need run attribution. A snapshot query after
listener attachment/reconnection repairs missed-event state.

Prefer versioned new IPC events/endpoints or a coordinated compatibility period.
Do not claim the frontend accepts added fields or typed phases until its owner
verifies the contract. Keep tokens, authenticated URLs, command arguments and
operator paths out of operation metadata. Logs/error text require separate
redaction review; a typed envelope alone does not sanitize payload contents.

## Rollout and acceptance gates

1. Agree on lifecycle and IPC contracts with core/frontend owners. Keep current
   installed identities and all independent build/lockfile boundaries intact.
2. Extract the host manager in a separately approved implementation task; inject
   workers/event sinks so actual manager code can be tested without Tauri or stores.
3. Convert the scoped reproductions to direct manager tests: wrong-ID/stale-ID
   cancellation, cleanup barrier, repeated cancellation, repair/run exclusion,
   failed-task admission, old completion versus replacement, sequenced progress,
   and old-run exit while a second run lives.
4. Add worker tests for cancel during install/sync, partial writes, child spawn
   handoff, and subprocess exit/kill ordering. Use isolated synthetic directories
   and local fixtures, never live player stores or production endpoints.
5. Verify actual IPC consumption and native desktop/game behavior, including
   multiple game windows and hide/minimize/close preferences. Current fixture
   tests do not satisfy these integration gates.

No releases, deployment, provisioning or data migration are authorized by this
note. The ignored desired-invariant tests remain explicitly nonacceptance evidence
until their production behavior is implemented and directly exercised.
