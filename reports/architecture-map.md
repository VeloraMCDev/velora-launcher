# Architecture map

## Recovery continuation boundaries — October 13, 2026

Core sync owns a stable cooperative lock and bounded relative-path intent journal
inside existing `.scopenet` metadata; state/quarantine identities stay unchanged.
Restart verifies/replays precommit rollback or authorized committed cleanup. A
verified unchanged request is read-only, not a bypass for actual mutations or
deep corruption. The isolated Unix cache reconstructs verified bounded inventory
under stable ownership; it is not connected to downloads or installed layouts.
Host instance-manifest fallback accepts only typed transport failures, keeping
existing cache format/path but not claiming offline authority provenance. New
frontend browser facade exists solely in a nonproduction fixture entry. Existing
application/platform/gameplay, updater and credential-store boundaries remain.
Scope and unresolved native/cross-store gates: `recovery-progress.md`.

## Final Java integrity boundary — October 13, 2026

Public runtime ensuring delegates through a private injectable index seam for
loopback tests; the default Mojang URL, runtime paths and marker identity stay
unchanged. Manifest fetch validates reference/digest and bounds response bytes
before decode/install mutation. Online warm uses that verified manifest to
deep-check declared files, repairing corruption without rewriting unchanged
markers. No new persistent cache/path/dependency is introduced. Historical index
failure fallback is unchanged; whole-runtime transactions, concurrent ownership
and offline integrity remain separate work. See `java-runtime-integrity.md` and
its independent review. Final scoped validation is in `final-local-sweep.md`.

## Historical 75% architecture — October 13, 2026

Fixed secondary-page registry and per-key promise coalescing defer modules, not
account/instance/page state. Home remains eager and owns typed operation intent.
Shared dialog/switch/status primitives consume semantic tokens and bounded focus
ownership. Diagnostics maps approved signatures to local safe summaries; no upload.
Core exports an isolated exclusive-root content cache without changing downloader
or persistent layouts. Details and non-goals: `progress-validation.md` and its
implementation reports. Existing host/client/contract boundaries remain intact.

## Historical 50% architecture — October 12, 2026

The native commands facade now reexports 124 preserved IPC commands across 16
domain modules plus an internal request helper. Shared `panel_client.rs` owns
bounded JSON transport and redacted network/auth/status/decode/config failures;
vendor auth, skin binary fetch and signed updater remain separate policies.
`velora-shared` owns migrated Discord/config wire contracts; Panel producers,
host typed decoders and TypeScript consumers share synthetic fixtures. Other
legacy Value endpoint families remain inventoried, not universally typed.

Frontend account/context/settings helpers own the new mutation/reconciliation
boundaries; preparation events retain correlated ownership. Profile/social state
remains component/page owned, explicitly inventoried rather than rewritten.
Native accounts and profiles use clone-persist-commit atomic JSON; credential
rollback restores exact original encrypted bytes or absence. No durable multi-file
journal is claimed. Root downloader owns capped attempts/production redirects and
cancellation cleanup; core loader tests cover real parsing/merge boundaries.

Platform/gameplay dependencies and persisted/signing/protocol identities remain
stable. See `fifty-percent-validation.md` for final evidence and remaining gates.
Earlier architecture descriptions below are historical.

## Historical 25% source additions — October 12, 2026

The historical flow inventory below describes the baseline/early batches, not
remaining current source behavior. Native preparation now carries admitted UUIDs
through cancellation and progress/state/run/log payloads. The production frontend
`preparation-events.ts` domain owns admission/event filtering; the store maintains
run ownership and exit tombstones. Process/background ownership still needs work.

Root utility HTTP uses exclusive owned temporaries and full-response/declared-size
verification. Root core sync separates safety preflight and reversible publication
into `sync/safety.rs` and `sync/transaction.rs`; state publishes last after staged
verification. There is no durable journal/restart recovery. Native `persistence.rs`
provides atomic typed JSON replacement; settings update clones, saves then commits
memory under AppState's write lock. Other persistence paths are not migrated.
Platform/gameplay boundaries, operational identities and independent segment
lifecycles are preserved. See the corresponding implementation reports for limits.

## Boundaries and ownership

| Segment | Entrypoints / relationships | Initial owner |
| --- | --- | --- |
| Launcher UI | `launcher/src/App.svelte`, `launcher/src/lib/store.svelte.ts`, Tauri bridge/types, pages and components | Frontend audit worker |
| Native host | `launcher/src-tauri/src/commands.rs`, `state.rs`, `game.rs`, settings/accounts/secrets | Operation regression worker |
| Install engine | `crates/core/src/install.rs`, `http.rs`, `sync.rs`, Java/loaders/launch/layout | Download/sync regression worker |
| Reusable HTTP/FS | `crates/platform-utils/src/http.rs` and neutral shared contracts | Lead; implementation deferred |
| Panel host | `panel/server` composes reusable Panel/auth libraries plus gameplay | Lead inventory; unchanged |
| Independent services | `infra`, `panel/operations`, Java integrations, Docs | Separate checks/releases; unchanged |

## Critical flows

1. At baseline, UI launch/repair/cancel used one global `AppState.task` abort handle. The latest scoped fix replaces it with `AppState.preparations`: owner instance/internal UUID plus joined supervisor finalization, with a shared in-process mutation lease. Game processes still use the separate `RunningGame` vector keyed by run ID; public preparation IDs and event correlation remain future work. See preparation implementation and operation design.
2. Install resolves Minecraft metadata, runtime, loader/libraries/assets and launches the process through core modules. Core aggregate progress wraps reusable `velora-platform-utils` HTTP download primitives. Shared HTTP primitives already exist: avoid a redundant downloader abstraction.
3. Panel manifest/revision drives `sync.rs`: inspect state, identify managed/unknown files, clean/prune as requested, download, then persist `.scopenet/state.json`. This ordering lacks a transactional rollback boundary; see download/sync design.
4. Native settings/accounts/secrets reside under the existing application data directory. Settings and encrypted secrets use temporary-file patterns in places; cached manifest and sync state also have direct writes. Review each path independently before claiming atomic persistence.
5. Platform libraries must not import gameplay or the complete Panel host. The complete Panel host still composes gameplay directly; no formal extension interface has been implemented.

## Dependency sequencing

Operation test fixtures → approved identity/event contract → real command-boundary regressions → job manager implementation → UI consumer migration.

Controlled HTTP/FS regressions → approved manifest/path/install transaction contract → staged file verification → reversible sync commit/recovery → interruption/native upgrade acceptance.

Frontend audit + production bundle measurements → scoped stale-response/navigation tests → domain extraction → measured lazy loading. Do not couple these refactors to an installer state migration.
