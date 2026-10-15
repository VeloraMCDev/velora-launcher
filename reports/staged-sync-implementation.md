# Staged sync and publication rollback

October 12, 2026. Root core `sync` retains its public signature, persisted IDs,
`.scopenet/state.json` and clean-mod quarantine location. No live stores were used.

## Implemented ordering

Preflight rejects unsafe/reserved/overlapping/aliased paths, ordinary symlinks,
nonregular targets, invalid manifest SHA-1 and unknown user-file collisions.
Corrupt or unreadable persisted state fails closed rather than being silently
replaced. Current files/state are fingerprinted before staging.
`.scopenet/staged-sync` is exclusively claimed; an existing workspace blocks a
new sync and requires manual inspection/recovery. All replacement files are
downloaded, hash/size verified and file-synced in staging before live moves.
Publication records reversible moves and original content fingerprints. New state
is serialized and file-synced, then renamed last. Successful rollback removes its
workspace. If reverse targets are occupied or moved content is externally changed,
rollback refuses destructive replacement and preserves backups/workspace.

## Evidence

Existing failed-batch, failed-clean and fast-path wrong-size desired invariants
are now enabled passing regressions. Six additional actual sync safety tests cover
preflight path/digest/alias rejection, corrupt-state and user collision preservation,
Unix symlink escape, final-state rename failure after live moves restoring the full
synthetic tree, occupied rollback target and externally modified live destination.
Final locked core suite passes; exact command/counts are in `milestone-validation.md`.
Independent bounded review: `staged-sync-review.md`. Injected final publication
failure proves in-process rollback behavior, not power-loss/restart recovery.

## Remaining gates

There is no durable transaction journal, automated interrupted-commit recovery,
parent-directory fsync or interprocess lock. Cancellation during staging retains
the workspace and blocks retries pending manual recovery. Path operations retain
TOCTOU windows and do not establish native handle-bound containment. Supported OS,
disk-full/fsync failure, power-loss, restart and native upgrade tests remain open.
Snapshot hashing adds warm-path work; no performance/no-regression claim is made.
Do not delete an interrupted workspace blindly: it may contain original backups.
