# Preparation ownership implementation

Date: October 12, 2026. User authorized the next bounded Phase 1 source task. This
is not completion of the full job-manager design, Gate 1, or a release.

## Implemented behavior

- The new production `PreparationManager` stores an ephemeral UUID, exact owning
  instance, abort handle and cancellation flag. It admits one preparation, and
  does not invoke a spawn closure while occupied.
- Cancellation is a no-op for another instance, a repeated request, an empty
  registry or a finished worker. Recognized cancellation aborts without removing
  the slot. The supervisor awaits the actual worker before cancellation state
  publication and exact-ID finalization. A completed handle alone is not enough
  for admission: the registry remains occupied until finalization.
- Premature completion cannot clear an unfinished owner. A stale completion ID
  cannot clear a newer owner even when that newer worker has completed.
- Terminal publication occurs with no registry mutex held; the slot remains busy
  until publication returns and the supervisor re-locks for exact-ID finish.
  This avoids reentrant Rust-listener deadlocks without opening an admission gap.
- One in-process resource lease gates preparation, deletion, cache clearing and
  Panel username changes. It is acquired before registry inspection, retained
  through scoped asynchronous work and released on rejection/completion/abort.
  A busy registry still rejects admission after the resource guard has dropped.
- Existing running-game restrictions and multiple-window launch behavior remain.
  Cache clearing now reports non-NotFound filesystem errors, rather than silent
  success. Command names, caller arguments and payload shapes remain unchanged.

## Files and compatibility

`launcher/src-tauri/src/preparation.rs` and its direct tests implement the new
registry/lease API; `state.rs`, `commands.rs`, `accounts.rs`, and `lib.rs` compose
it. `launcher/README.md` describes the contract and its limitations. No frontend
or `game::run` production change, new dependency, lockfile change, storage schema,
installed path, account/protocol ID, credential/updater identity or signing change.

The copied Phase 0 commands fixture module was removed: it continued to execute
old logic independently of the real implementation. Its two ignored owner/join
contracts are replaced by passing tests of the production manager. Remaining
game-payload fixtures are retained and are not native event acceptance.

## Validation and review

- Direct manager/resource tests: 13 passed. Real Tokio tasks, startup oneshots,
  controlled Notify completion and joins; no AppState construction/live stores.
- Main complete local native-host suite: 27 passed, one stale-event contract
  ignored. Core suite: 43 passed, four known-gap contracts ignored; seven existing
  online tests remain ignored. The final run after strengthening stale-finish
  coverage and scoped formatting exited 0; log:
  `/tmp/velora-revitalization-preparation/tests-final.log`.
- New production module and direct tests pass scoped rustfmt checks. Tracked
  whitespace, boundary/identity, workflow/publication, tracked secrets, separate
  untracked-source/report scans and local Markdown link checks are run by main.
- Independent exact-diff review: `preparation-review.md`; actual test execution
  and limitations: `preparation-regressions.md`. Reviewer does not self-approve
  implementation and did not duplicate main's full builds.

## Remaining gates

Instance ownership is **not** full request correlation. A delayed cancel for an
earlier preparation of the same instance can still cancel its replacement;
public operation IDs and consumer migration require a separate contract. Old
game idle/progress/log events remain uncorrelated. Native IPC listener ordering,
supervisor event acceptance, actual delete/cache/rename failure scenarios and
cross-platform behavior are not certified by manager unit tests.

Joining an aborted task does not prove that independent Tokio filesystem work,
loader helpers, texture threads or subprocesses have stopped. The existing
close-after-spawn ownership window remains unresolved. No transactional rollback,
interprocess lock, symlink/alias containment, resource recovery or process-kill
guarantee is claimed. These remain Gate 1 requirements before release.

## Rollback

Revert the manager composition, lease integration and its README/tests together
as one source patch; preserve earlier downloader and audit work. No persistent
state conversion or restore is needed. Rollback reintroduces the old ownership
and admission gaps and the prior silent cache-clear failure behavior. No commit,
PR, merge, package, release or deployment was performed.
