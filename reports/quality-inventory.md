# Quality inventory

Inventory at baseline HEAD; search matches are triage signals, not a list of defects.

## Largest source candidates

| File | Lines | Interpretation |
| --- | ---: | --- |
| `panel/operations/web/src/lib/icons.svelte.ts` | 2,440 | Asset catalogue; do not prioritize solely by size |
| `panel/server/src/routes/guilds.rs` | 2,175 | Domain route hotspot; outside first implementation batch |
| `launcher/src-tauri/src/commands.rs` | 1,859 | Large IPC/API/lifecycle surface; contracts before extraction |
| `panel/server/src/routes/servers.rs` | 1,479 | Server-domain hotspot |
| `panel/server/src/db.rs` | 1,433 | Persisted schema risk; operational migrations retained |
| `launcher/src/pages/Guilds.svelte` | 1,243 | Frontend coupling hotspot |
| `panel/server/src/seed.rs` | 1,194 | Seed policy requires separate review |
| `launcher/src/pages/Social.svelte` | 1,191 | Frontend coupling hotspot |
| `launcher/src/pages/Settings.svelte` | 874 | Settings/domain extraction candidate |

Method: `git ls-files`, count newline-split entries in Rust/Svelte/TS/MJS/Java, exclude paths containing test/scripts/fixtures; inspect generated/catalogue/archive entries separately. 1,535 tracked files; 845 heuristic production candidates. Archived integration code is not a maintained refactor target.

## Search triage and concrete evidence

- No literal TODO/FIXME matches in scanned source extensions. This does not imply no missing behavior.
- 2,810 Rust `unwrap`/`expect` matches **including tests**; many highest counts are fixtures. Do not blanket-rewrite valid invariants. Inspect poisoned-lock handling and untrusted I/O individually.
- 1,180 Rust `Value`/`serde_json::Value` matches including tests/types; launcher commands has 68 lexical matches. This does not prove each boundary is unsafe.
- 83 Rust filesystem-write lexical matches including tests. `crates/core/src/sync.rs:36` writes state directly; `launcher/src-tauri/src/state.rs` writes cached manifest directly. Blocking filesystem/hash work exists inside async core preparation; profile before moving everything to worker pools.
- HTTP implementation is in `crates/platform-utils/src/http.rs`, not a `download.rs` file. Core reexports it and adds aggregate progress. Confirmed already-shared logic invalidates a blanket deduplication task.
- Launcher production build has a large main chunk; browser mocks are separately imported chunks, not evidence they execute in native production.
- Existing native credential tests protect unreadable/missing-key cases. Preserve `net.scopenet.launcher` service and existing data paths.

## Test gaps / claims requiring evidence

See worker regression reports and independent review. Default ignored online tests do not certify loader support, Java downloads, server joins or upgrade safety. Synthetic browser fixtures certify UI observations only. Native process overlap, stale events, sync failure recovery and interrupted file promotion require further real-boundary tests before Gate 1. No broad visual rewrite or source restructuring is authorized by this inventory.
