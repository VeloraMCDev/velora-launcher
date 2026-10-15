# velora-launcher-core

Core owns loader installation and progress composition; neutral HTTP/digest
operations remain delegated to platform utilities. Unused direct sha1/sha2/hex
dependencies were removed without changing package versions or segment lockfile
ownership. Synthetic loader profile/merge fixtures cover contract parsing, library
precedence and inherited metadata, not installed loader/native acceptance.

Maintained Rust launcher engine segment in the root Cargo workspace.

```sh
cargo test --locked -j 1 -p velora-launcher-core
```

Run from the repository root. Reusable libraries must not depend on private gameplay or the complete Panel host. See ../../docs/BOUNDARIES.md and docs/components for component contracts and source attribution.

## Staged instance sync

Sync preflights manifest digests, paths, ownership conflicts and existing JSON
state, then stages verified replacement files before cleaning or replacing the
current tree. Existing state fields and `.scopenet/removed/<epoch>/mods` quarantine
paths remain compatible. Unknown user-file collisions are rejected, not overwritten.
Publication moves record inverse operations; a publication error rolls back when
the recorded files remain unchanged. State is serialized and file-synced before
the final rename, after successful file publication.

Sync now retains a cooperative process lock in `.scopenet/sync.lock` and records
versioned, bounded move/parent intents in `.scopenet/staged-sync/journal.json`.
Restart under ownership rolls back an uncommitted transaction or completes
authorized committed cleanup after checking state and artifact fingerprints.
Changed/ambiguous evidence and nonempty legacy staging without a journal remain
blocked for manual review; do not delete staging blindly because it may hold
previous files. Synthetic process-exit recovery passes on Darwin arm64, not the
complete native OS or power-loss matrix. Unix directory fsync is implemented;
Windows directory durability, native reparse/alias/locked-file behavior,
background I/O draining and cross-store transaction recovery remain gates.
Details: `../../reports/sync-recovery-implementation.md` and its independent review.
Independent replacement intents are persisted as one write-ahead batch while
retaining each rename's parent-directory flushes. Synthetic cold-sync measurements
improve relative to the preceding journaled source, but synchronization overhead
and native performance gates remain. See `../../reports/sync-batch-optimization.md`.

## Isolated content cache

The optional cache prototype is not wired into downloads or the installed layout.
It supports explicit Unix-only reopening with a stable cooperative owner lock,
canonical bounded inventory and verified final bytes. Process interruption drops
unlinked scratch and retains verified linked finals; it does not resume a partial
HTTP download. Non-Unix usage fails explicitly without creating a cache. See
`../../reports/content-cache-contract.md` and the recovery implementation/review
for trust assumptions, synchronization limits and remaining production gates.
