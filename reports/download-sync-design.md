# Agent C: staged download/sync rollback design

Started 2026-10-11; validation continued 2026-10-12 (session-local dates).
Initial design only; **not implemented**.
Scope: core test additions and these two Agent C reports. Launcher/Tauri changes
belong to the other agent; baseline reports belong to the coordinating agent.
Read root AGENTS.md, MONOREPO.md, docs/BOUNDARIES.md, docs/VALIDATION.md and
docs/DEPLOYMENT_TARGETS.md. No new infrastructure, deployment, release,
dependency, lockfile, persisted identity or production behavior changes.

## Evidence and current behavior

- `crates/core/src/http.rs:11`: `download_all` collects all results before
  returning the first error. Completed siblings are not rolled back. Its
  `files_done` counter counts finished attempts, including errors, rather than
  verified/published files; progress is not a transaction receipt.
- `crates/platform-utils/src/http.rs:107`: `download_one` retries four times.
  `crates/platform-utils/src/http.rs:124`: each attempt streams into a sibling
  `.part`, checks SHA-1 when provided, then renames into the destination.
  Manifest size is not checked before rename. Stream errors/cancellation can
  leave `.part`; checksum mismatch removes it. Individual-file replacement
  does not provide batch atomicity or power-loss durability.
- `crates/core/src/sync.rs:128`: changed revisions/deep verification check size
  and SHA-1; unchanged revisions only check existence and accept truncation.
- `crates/core/src/sync.rs:109`: clean updates remove managed files, move player
  mods and delete the resource-pack cache **before** any replacement downloads.
- `crates/core/src/sync.rs:134`: ordinary dropped-file removal happens only
  after downloads succeed. `crates/core/src/sync.rs:149`: revision/ownership/
  clean epoch save happens last, but `crates/core/src/sync.rs:33` writes the
  state file directly, without a temporary-file transaction.
- Reviewer E4/E5 in `reports/independent-review.md` adds source-confirmed gates:
  `crates/platform-utils/src/paths.rs:16` is lexical only; reserved state paths,
  destination aliases and symlink parents remain possible. `clean` suppresses
  filesystem errors at `crates/core/src/sync.rs:63` and
  `crates/core/src/sync.rs:76`; ordinary removal does so at
  `crates/core/src/sync.rs:140`. Failure can therefore still lose ownership or
  advance clean epoch. A newly managed destination can overwrite an unknown file
  through `crates/platform-utils/src/http.rs:151`. These are review findings,
  not containment/error/overwrite reproductions in this test-only patch.
- Passing fixtures establish preservation of prior state on a failed single
  download/cancellation, not safety of a mixed batch or failed clean update.
  The executable known-gap contracts are listed in download-sync-regressions.md.

## Proposed invariants

1. No live mutation until every required replacement is fully downloaded and
   verified against the manifest's size and existing SHA-1 contract. Define
   whether size zero is exact empty or unspecified before implementation;
   `Download` currently conflates `None` and zero. Preserve public compatibility.
2. A failed/cancelled operation leaves the previous revision, clean epoch,
   managed bytes, dropped files and relevant clean-update content recoverable.
   Success publishes all files and state together from the launcher's viewpoint.
3. Ordinary sync never deletes unknown paths. Explicit clean update retains the
   existing semantics: quarantine player mods at `.scopenet/removed/<epoch>/mods`,
   remove managed files and clear the server resource-pack cache, preserving
   worlds, screenshots, options and unrelated configuration. Existing `clean`
   actually moves all regular top-level mod files, not only `.jar` suffixes.
4. Never launch, concurrently sync or report success from an intermediate tree.
   Multi-file filesystem publication is not a single atomic rename: isolation
   and recovery are required; external programs cannot be promised an atomic view.
5. Keep `.scopenet/state.json`, its revision/managed/clean_epoch fields, installed
   identities and protocol SHA-1 semantics. No database migration or live-store
   conversion is implied. Transaction metadata is additive and private.

## Proposed state machine

### Plan and stage (no live changes)

- Acquire an instance-wide interprocess lock before recovery/planning and share
  the exclusion contract with launch. Launcher integration is a separate gate,
  not part of Agent C's test-only patch.
- Load and retain the exact old state bytes. Missing state is a first install;
  malformed state must be distinguished from missing state for safe recovery,
  rather than silently authorizing destructive cleanup.
- Validate normalized manifest paths, duplicates, case-fold collisions on
  supported filesystems, reserved `.scopenet` paths, symlinks/reparse points and
  parent containment. Lexical `safe_join` alone is not a full containment guard.
  Normalize ownership keys and destinations identically: `mods/a.jar`,
  `mods/./a.jar`, repeated separators and backslash variants must not become
  independent ownership entries for one file. Reject file/directory collisions
  and destination/temporary aliases. Apply containment validation to persisted
  managed paths as well as the incoming manifest. Preflight the complete set
  before cleaning; rejected paths must fail, not silently skip then save success.
- Identify replacements, removals and clean-epoch moves. If a desired managed
  path collides with an unknown user file, stop for an explicit conflict policy
  or retain a recoverable copy; never silently overwrite. Recheck file identities
  immediately before apply so user changes during staging are not lost.
- Stage beneath the same instance filesystem in a unique transaction directory
  (proposed `.scopenet/transactions/<id>`). Do not reuse destination `.part`
  names across operations. Download every required file there, validate status,
  complete stream, exact declared size under the chosen compatibility policy,
  and SHA-1. Use existing bounded concurrency/retry limits initially.
- Build the sorted target managed set and next state in staging. Verify even
  same-revision existing files at least by size; retain explicit deep SHA-1
  verification. Do not trust existence as proof of a complete file.

### Prepare and apply (recoverable changes)

- Persist a versioned write-ahead journal containing transaction ID, old state
  bytes, planned next state, relative source/destination/backup paths, expected
  file identities/hashes, clean epoch, and operation intents. No credentials or
  operator addresses. Flush journal and verified staged files before live changes.
- For each operation, durably record intent before moving anything, back up the
  old managed file on the same filesystem, then publish the staged replacement.
  Record completion after the rename. Recovery must tolerate crashes between
  intent, rename and completion without trusting a completion flag alone.
- Move dropped managed files and resource-pack cache into rollback storage,
  instead of deleting immediately. Move unknown top-level mod files to a
  transaction quarantine until commit; do not overwrite preexisting epoch
  quarantine entries. Backup/quarantine collisions are explicit errors.
- On any failure, reverse applied operations and restore old state bytes. Do not
  discard the journal or backups until rollback is verified. A failed rollback
  blocks launch and subsequent sync with a clear recoverable status.
  Propagate directory enumeration, quarantine creation/move, removal and backup
  errors explicitly. Suppressed errors cannot count as completed removals or
  permit target revision/epoch/ownership publication.

### Commit and finish

- Write next state to a temporary sibling of `.scopenet/state.json`, flush it,
  then replace the state atomically using platform-appropriate primitives.
  Parent-directory durability and Windows replacement/locked-file behavior need
  explicit testing. Commit state only after the new tree is verified.
- Make the exact persisted target state plus verified tree/journal agreement the
  commit decision. Recovery can recognize this if a crash precedes the journal's
  final committed marker. Matching only revision is insufficient (deep repairs
  may retain the same revision).
- After the commit point, finalize player-mod quarantine into the existing
  epoch location with collision-safe, idempotent operations; clean up obsolete
  backups/staging. A finalization error is a committed-but-cleanup-pending
  transaction, not permission to roll back after deleting backups.
- Preserve unknown files created after staging or a crash. Recovery must compare
  identities before deleting/replacing anything; conflicts block recovery for
  operator review rather than guessing. Never recursively delete a directory
  merely because its name resembles a transaction directory.

## Cancellation, interruption and restart

Staging cancellation can discard its owned temporary directory without touching
the instance. Apply cancellation requires journal-driven rollback/recovery;
async Drop alone cannot reliably complete it. Abrupt process termination relies
on the persisted journal. Recover incomplete work on the next launch/sync while
holding the same lock, before exposing the tree. Distinguish cancellation, failed
verification, successful commit, cleanup pending and rollback failure in future
results/events without presenting retry byte totals as verified progress.

## Implementation and release gates

- First fix verification/temporary lifecycle in the reusable platform HTTP layer,
  independently of gameplay/application hosts. That production change is outside
  this assignment; Agent C exercises its re-exports only.
- Then implement journaled staging, atomic state persistence and rollback in
  core. Coordinate launch/cancel/lock behavior with the launcher agent. Keep
  ordinary tests green and unignore each known-gap contract only when fixed.
- Add failure injection at each rename/journal/state-write boundary, disk full,
  permissions/locked files, state corruption, quarantine collisions, symlinks,
  concurrent mutation and interrupted rollback. Include cancellation after a
  successful sibling publishes and crashes just before/after state commit.
- Reviewer containment acceptance: use two synthetic temp roots for symlink
  escape attempts; incoming and persisted managed paths must leave the outside
  sentinel untouched. Reject aliases, case-fold variants, reserved
  `.scopenet/state.json`/transaction/quarantine destinations, file/directory and
  destination/temporary collisions before any live change. Include Windows
  reparse points/reserved names and mutations of parent paths during apply.
- Reviewer filesystem-error acceptance: inject failed read_dir, entry reads,
  create_dir, rename, unlink and cache removal as well as state persistence.
  Assert exact old state bytes and recoverable old tree, retained ownership and
  unchanged clean epoch; failed rollback must retain its journal and block launch.
- Reviewer unknown-file acceptance: manifest adoption of a previously unmanaged
  destination must not silently overwrite user bytes. Test same-path user mods/
  config, existing quarantine targets and user changes during staging/recovery;
  assert explicit conflict or a verified recoverable backup, not only file counts.
- Perform process-kill/restart and real filesystem durability tests separately
  from these in-process task-abort fixtures. Validate Linux/macOS/Windows rename
  semantics, case-insensitivity, cross-device rejection and launch exclusion.
- Run relevant local/source checks, preserve segment build/license boundaries,
  then native desktop/game upgrade acceptance with synthetic stores. Packaging,
  signing, admin approval and release remain manual. No deployment occurs here.
