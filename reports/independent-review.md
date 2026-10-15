# Agent E — independent initial review

Date: October 12, 2026 (updated session-local date). Baseline HEAD: `4f63926`.
**Status: findings and design review only. No production fixes yet; no implementation approval.**

## Scope and evidence discipline

Read root `AGENTS.md`, `MONOREPO.md`, `docs/BOUNDARIES.md`,
`docs/VALIDATION.md`, and `docs/DEPLOYMENT_TARGETS.md`. Started independent
source inspection before the other reports existed. Inspected launcher host
state/commands/game events, core sync/download/install/launch, the actual reusable
platform HTTP/path implementation, frontend store/effects/mock adapter, and CI.
Compared HEAD production code with the concurrent test-only diffs to avoid
mistaking another worker's additions for a production fix.

Only this report is written by Agent E. Other agents already had source/test
changes in this shared checkout. No source edit, credential-store access,
application startup, player-store access, commit, PR, release, or deployment by E.
Main owns fresh baseline npm/core/source checks; their outcome is not inferred here.
Line anchors below refer to production sections inspected; appended tests can
change their later test-section line numbers.

## Findings requiring implementation gates

### E1 — High: cancellation drops ownership before worker quiescence

Evidence: `launcher/src-tauri/src/state.rs:54`,
`launcher/src-tauri/src/commands.rs:476`, `launcher/src-tauri/src/commands.rs:497`.
The slot contains an abort handle, not an instance/operation owner. Cancel takes
the global slot regardless of the supplied instance, aborts without joining,
and emits idle labeled with the request's instance. A wrong-instance or stale
cancel can affect another preparation. Removing the slot admits another task
before cancellation completion is observed. Synchronous sections and filesystem
work need not have quiesced merely because abort was requested.

Gate: typed operation ownership; atomic admission/reservation; cancellation
acknowledgement distinct from completion; retain write ownership through cleanup;
direct tests of the production manager for wrong/stale IDs, repeated cancel,
cancel/relaunch, late completion, failure and cleanup failure. A copied fixture
proves the current decision pattern, not command integration or filesystem damage.

### E2 — High: stale events can overwrite an unrelated preparation

Evidence: `launcher/src-tauri/src/game.rs:53`, `launcher/src-tauri/src/game.rs:88`,
`launcher/src-tauri/src/game.rs:365`, `launcher/src-tauri/src/game.rs:370`,
`launcher/src/lib/store.svelte.ts:234`, `launcher/src/lib/store.svelte.ts:259`.
Progress has no operation/instance identity. Generic preparing/idle/error has no
operation identity; exit has a run ID, but the watcher then emits generic idle.
The frontend accepts idle/error/running globally without matching `launchingId`
or a preparation generation. An older game's exit can reset a newer preparation;
progress and logs from independent owners are mixed. Run removal and kill already
target exact run IDs correctly; preserve that positive behavior.

Gate: host-correlated operation and run envelopes, monotonic owner sequences,
frontend identity filtering, and authoritative active-run/preparation snapshots.
Test delayed old progress/error/exit during a newer same-instance and different-
instance operation. Instance equality alone does not solve same-instance relaunch.

### E3 — High: failed clean and mixed-batch updates leave changed live bytes

Evidence: `crates/core/src/sync.rs:57`, `crates/core/src/sync.rs:109`,
`crates/core/src/sync.rs:134`, `crates/core/src/http.rs:48`,
`crates/platform-utils/src/http.rs:151`.
Clean removes prior managed files and pack cache and moves player mods before
replacement verification. Ordinary batches publish each successful sibling
before another sibling can fail. Saving the old revision until the end does not
restore the old tree. This is actual data-safety exposure, not merely misleading UI.

Gate: verified staging before live mutations; rollback/recovery journal for apply;
same-filesystem backups; launch exclusion; atomic state replacement; synthetic
failure injection at every rename/state/journal boundary and process-kill restart.
Preserve old state bytes and unknown user files. Failed rollback blocks launch.
Multi-file publication must not be described as a single atomic filesystem action.

### E4 — High: lexical path safety does not provide filesystem containment

Evidence: `crates/platform-utils/src/paths.rs:16`, `crates/core/src/sync.rs:123`,
`crates/platform-utils/src/http.rs:129`, `crates/core/src/sync.rs:33`.
`safe_join` rejects traversal lexically but follows existing parent symlinks during
later IO. It accepts reserved `.scopenet` paths. Normalization for `wanted` only
replaces backslashes while `safe_join` also collapses dot/empty components: aliases
such as `mods/a.jar` and `mods/./a.jar` can share a destination but different
ownership keys. Deterministic destination `.part` names can collide across aliases
or with another manifest destination. Case-insensitive aliases add native risk.
These are source-confirmed missing validations, not an executed exploit claim.

Gate: preflight complete manifest before cleanup/download; reject reserved metadata,
duplicate normalized destinations, file/directory and destination/temp collisions;
symlink/reparse-aware containment including persisted managed paths; unique owned
temporary files; test case-insensitive and Windows reserved-name behavior. Do not
silently skip unsafe files and persist a successful target revision.

### E5 — Medium/high: verification and ownership persistence have gaps

Evidence: `crates/platform-utils/src/http.rs:124`, `crates/core/src/sync.rs:128`,
`crates/core/src/sync.rs:136`, `crates/core/src/sync.rs:33`.
Manifest size is not checked before downloader rename. An unchanged sync revision
only checks existence, not even size/type. Direct state writes can be torn;
malformed state silently becomes default. Removal/clean errors are suppressed,
yet success can advance epoch or drop ownership of files that were not removed.
A new managed path may overwrite an existing unknown user file without backup.
Retry bytes and finished-file counters are not verified-publication metrics.

Gate: distinguish absent/corrupt state, size-zero/unspecified compatibility,
regular-file validation, explicit failed-removal/conflict policy, atomic/durable
state writes, unknown-file preservation, permission/locked-file/disk-full tests.
Fix the downloader in `crates/platform-utils`, not just its core re-export;
preserve reusable platform independence and review all existing callers.

### E6 — High gate: manager ownership must cover destructive commands and spawn

Evidence: `launcher/src-tauri/src/commands.rs:575`,
`launcher/src-tauri/src/commands.rs:585`, `launcher/src-tauri/src/game.rs:138`,
`launcher/src-tauri/src/game.rs:165`, `launcher/src-tauri/src/game.rs:241`,
`crates/core/src/launch.rs:266`.
Delete checks tracked runs, not preparation; cache clearing ignores preparation
and suppresses errors. Install precedes the already-running guard and server-list
injection is outside it. The close-after-launch branch sleeps after spawn before
registering any run; cancellation can drop the task there while the child is not
tracked, with no `kill_on_drop` configured. Ordinary capture registers without an
intervening await, but process ownership still needs an explicit handoff contract.
Shared cache/natives/runtime writes require broader leases than instance sync.

Gate: include delete/cache/install/configuration/process handoff in the exclusion
policy; preserve intentional multiple-run and close-after-launch semantics.
Exercise a synthetic subprocess across cancellation/registration/exit/kill and
native hide/minimize/close, including a game surviving launcher shutdown. Do not
infer child cleanup from an AbortHandle test or abruptly change detach behavior.

### E7 — Medium: frontend asynchronous effects and subscriptions lack lifecycle gates

Evidence: `launcher/src/lib/store.svelte.ts:95`,
`launcher/src/lib/store.svelte.ts:227`, `launcher/src/pages/Settings.svelte:24`,
`launcher/src/pages/Settings.svelte:118`, `launcher/src/lib/textures.svelte.ts:23`.
Event subscriptions are initiated without awaiting registration, retaining cleanup
handles, or handling listener rejection. Bootstrap precedes listener attachment;
there is no reconciliation snapshot for missed events. Settings profile requests
depend on active account but have no stale-response guard; Discord depends on the
panel-account boolean/tab, not the particular account, so same-kind switching need
not refetch. Late results can display another account's stale data. Home's public-
server effect uses an `alive` cleanup guard, a positive local pattern.

Gate: account/panel/instance generation keys, delayed-response and unmount tests,
awaited listener attachment with failure handling and reconciliation/cleanup.
Use actual store/effect reducers with injected IPC, not only the browser demo.
`svelte-check`, rune-location scanning and Vite build do not prove effect ordering.

## Security and identity compatibility review

- Preserve `net.scopenet.launcher` application/keychain identity, existing account
  IDs/UUIDs, `.scopenet` metadata/quarantine paths, layout, settings/account schema,
  legacy environment compatibility, updater signing identity and release approval.
  New operation IDs are ephemeral; they must not replace persisted instance IDs.
- `sanitize_id` is lossy: distinct panel IDs can map to the same directory. New
  locks must use the actual resource path as well as logical identity. Detect
  collisions; do not rename installed directories without compatibility testing.
- `game.rs:59` falls back to cached instance manifests on all fetch failures,
  including explicit 404/access denial, not just outages. Cache is instance-path
  scoped, not panel/account scoped. Define offline/authorization policy and test
  server switching/revocation; do not claim offline cache guarantees authorization.
- Existing `accounts.rs:258` limits panel bearer tokens to matching panel accounts;
  retain this guard. Updater checks signed approved releases and approved paths;
  operation cleanup must not bypass or re-key these checks.
- `crates/core/src/launch.rs:257` writes Java argument files, including arguments
  that may carry access tokens, without explicit private permissions/cleanup.
  Treat journal/error/log/command serialization as a separate redaction gate;
  typed event identity does not make payloads safe. Never collect live tokens here.
- SHA-1 artifact verification preserves Minecraft contracts; it is not an origin
  authorization mechanism. Retain independent strong updater signature checks.
  Review URL/redirect/log handling and trusted manifest boundaries separately.

## Test assessment and independent execution

The B operation fixture duplicates admission/cancel/kill predicates, although it
uses real Tokio abort/oneshot primitives and production payload structs. Its
passing tests can stay green if production commands regress. Payload serialization
checks are genuine but do not call `emit_state`/the watcher or frontend. Its three
ignored desired invariants are reproduction documentation, not acceptance tests.

The C tests invoke production `sync`, `download_all` and the re-exported real
downloader with loopback HTTP and temporary synthetic directories. These are real
filesystem/network behavior tests, not mocked sync logic. They cover old-byte
preservation for a single failed download and state preservation on task abort,
but do not imply batch rollback, crash recovery, interprocess locks or native
rename semantics. No core online install tests were run by E.

E executed the existing local test binary
`target/debug/deps/velora_launcher_core-8f3569f1cdebf1a3` directly to avoid competing
with Main's baseline Cargo build. Binary timestamp: October 11, 2026 23:58:44 local.
This was not a fresh source build/provenance attestation; anchors in panic output
already differ from concurrent edited test lines. Main must confirm the final
checkout build. Each command used the indicated filter:

| Filter | Independently observed result |
| --- | --- |
| `sync::tests::` | 9 passed, 3 ignored |
| `http::regression_tests::` | 3 passed, 2 ignored |
| `--ignored sync::tests:: --nocapture` | 3 intentional failures: unchanged-revision truncation retained; successful sibling stayed updated after batch failure; old managed file absent after failed clean |
| `--ignored http::regression_tests:: --nocapture` | 2 intentional failures: size mismatch published bytes; aborted download retained `.part` |

These five failures substantiate current gaps; they are not default-suite failures
and must not be counted as implemented fixes. Default green plus ignored red is
an honest characterization baseline, not permission to release. The inspected
root CI workflow does not execute launcher npm checks or core Rust tests; docs
listing a command and the online test comment claiming CI execution are not proof
that this workflow runs it. Main's relevant local checks remain required.

## Review of other workers and safe sequencing

Reviewed B's `operation-manager-design.md` and
`launcher-operations-regressions.md`, C's `download-sync-design.md`, and their
currently available test-only diffs. Their principal ownership/transaction findings
agree with this independent inspection. B explicitly acknowledges copied fixture
limitations; C distinguishes file rename from batch transaction/durability and
preserves existing IDs/quarantine paths. These are useful proposals, not approved
implementations. At this initial write, C's regression report and D's frontend
report were not yet available; later review must be recorded explicitly.

### Follow-up in this same initial review: all five reports now available

Reviewed C's newly arrived `download-sync-regressions.md` and D's
`frontend-audit.md` before handoff. C's recorded 36 passing/5 ignored core tests
are worker-reported fresh-build evidence; E's direct-binary runs independently
confirm the narrower results above. C correctly states that the failed-clean
reproduction stops at missing managed bytes, before its later cache/mod assertions,
and explicitly excludes process crashes/native concurrency. Its warning to use
`--lib` and module filters avoids accidentally running remote online tests.
The diff check of the four B/C edited source files reports no whitespace errors;
their observed changes remain test additions, not production fixes.

D's request-ownership recommendations agree with E7. Independently inspected
`launcher/src/pages/Guilds.svelte:64` and
`launcher/src/pages/Social.svelte:143`: delayed guild/chat requests can write
without a request-generation check; send completion appends to the current
message array after awaiting, even if its conversation changed. Add those direct
controller tests alongside account/profile effects. D's 12 existing capability/
presentation tests are not event, request-ordering, router or accessibility tests.
D records their Node 25 execution separately from the required Node 24 baseline,
and correctly does not equate a synthetic screenshot, build duration or bundle
bytes with native behavior/performance acceptance. E did not independently view
that screenshot or certify D's broader Panel accessibility claims. For this
launcher safety effort, E1–E6 remain gates before release even if the broader UX
backlog is pursued independently. Browser mocks must not be treated as a universal
network sandbox or as evidence of native cancellation/filesystem behavior.

B's regression report still has a validation placeholder at this observation;
do not mark its native-host test run complete until actual command/outcome is
recorded. No report or test-only patch reviewed here warrants implementation
approval. Fresh-build, direct-production lifecycle/IPC and native acceptance gates
remain explicit.

Recommended sequencing for a separately authorized implementation:

1. Agree host/core/frontend ownership, IPC versioning, cancellation, unknown-file,
   clean-update and recovery contracts. Inventory every writer/deleter and shared
   resource. Keep the single preparation limit; do not introduce parallel sync.
2. Add direct tests of extracted production lifecycle/reducers and harden reusable
   downloader validation/owned temporaries. Establish cancellation quiescence;
   neither a host manager alone nor a checksum fix alone closes transaction safety.
3. Implement core preflight, journaled staging/apply/rollback/restart recovery and
   atomic state persistence under resource leases. Make launch/delete/cache observe
   those leases. Include rollback failure, corrupted metadata, aliases/symlinks,
   quarantine collisions and unknown user changes during staging/recovery.
4. Wire correlated events, registration/reconciliation and frontend generation
   guards together; convert copied fixtures to tests of production paths. Unignore
   contracts only when implementations pass; retain explicit unsupported gates.
5. Main performs fresh relevant segment/source checks. Independent patch review
   then evaluates exact diff and test evidence. Separately require Windows/macOS/
   Linux filesystem/native IPC/subprocess tests and synthetic old-store upgrades.
   Signing, packaging, release approval and deployment remain manual and out of
   this assignment. E does not self-approve any implementation.

**Coordination message:** prioritize cancellation quiescence plus transaction
recovery, not cosmetic idle fixes. B must include destructive-command/shared-write
and spawn ownership; C must prove recoverable state/tree agreement, not revision
alone; D must reject stale owner events and async account responses. No production
fix is established by the currently passing characterization tests.
