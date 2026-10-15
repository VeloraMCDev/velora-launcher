# Native host command extraction implementation

## Scope and provenance

- Date: October 12, 2026. Bounded VEL-006 source extraction; independent review and native acceptance remain separate gates.
- Baseline HEAD: `4f639266a515c3cb05769b055a4772c4dcb89cb8`.
- Initial working-tree `launcher/src-tauri/src/commands.rs`: 1,732 lines, SHA-256 `0371ee381c2600f7884c24d58f4020c1ebfd2deb82c5277af41c8360534f5e09`.
- This was a dirty-tree snapshot containing earlier operation/settings fixes, not a claim of equivalence to clean HEAD.
- No commits, branches, releases, deployments, live stores, dependencies, lockfiles or registration edits by this worker.

## Implemented structure

The existing `commands.rs` remains the compatibility facade. Domain-specific imports replace the monolithic import set. Explicit exports retain every command function and its Tauri-generated `__cmd__` and `__tauri_command_name_` macro paths. Existing public DTO names remain reexported. The facade is now 361 lines.

Shared `Res`, `err` and `aerr` retain their definitions in the facade. Existing authenticated request construction and Panel error parsing move unchanged to `commands/panel_client.rs`; only these two helper declarations gain `pub(super)` visibility for sibling use. No new generic service abstraction or API/error policy is introduced.

### Paths changed by this worker

- `launcher/src-tauri/src/commands.rs`
- `launcher/src-tauri/src/commands/accounts.rs`
- `launcher/src-tauri/src/commands/bootstrap.rs`
- `launcher/src-tauri/src/commands/casino.rs`
- `launcher/src-tauri/src/commands/economy.rs`
- `launcher/src-tauri/src/commands/guilds.rs`
- `launcher/src-tauri/src/commands/instances.rs`
- `launcher/src-tauri/src/commands/launch.rs`
- `launcher/src-tauri/src/commands/maps.rs`
- `launcher/src-tauri/src/commands/market.rs`
- `launcher/src-tauri/src/commands/panel_client.rs`
- `launcher/src-tauri/src/commands/profiles.rs`
- `launcher/src-tauri/src/commands/progression.rs`
- `launcher/src-tauri/src/commands/social.rs`
- `launcher/src-tauri/src/commands/telemetry.rs`
- `launcher/src-tauri/src/commands/textures.rs`
- `launcher/src-tauri/src/commands/updates.rs`
- `launcher/tests/command-surface.test.mjs`
- `reports/host-extraction-implementation.md`

### Persistence integration handoff

- Account login, Discord, registration, offline addition, selection, removal and username handlers: `launcher/src-tauri/src/commands/accounts.rs`.
- Skin/cape, skin-profile persistence and community profile/post handlers: `launcher/src-tauri/src/commands/profiles.rs`.
- These paths are distinct from the lead-owned `launcher/src-tauri/src/accounts.rs` and `launcher/src-tauri/src/profiles.rs`.
- Account/profile handler calls are unchanged by extraction. Lead may implement separately reviewed save-before-memory and locking changes after this snapshot.

## Exact compatibility evidence

1. All **124 command names and signatures** match the captured pre-extraction snapshot, including sync/async mode, argument order/names/types, optionality and result types. The snapshot is embedded in the permanent npm test with source hash, baseline HEAD and working-tree provenance.
2. All **124 registration paths and their order** match the initial `lib.rs` snapshot. The permanent test also checks explicit function/macro exports through each domain and the original `commands::` facade. This worker did not edit `lib.rs`.
3. A one-time extraction gate compared all **144 top-level definitions**, including command bodies, DTOs, command/serde attributes, internal helpers and the existing casino path test module. Every definition hash matched, normalizing CRLF/LF and only the two deliberate helper visibility changes. Aggregate SHA-256 of the sorted initial name/hash records: `bada67f28617d1f337d0d66003c58c1736ab5e7ddbbfeb0f8889440a94538db5`.
4. That body-checksum gate passed before handoff, then was removed from the permanent suite as requested. Legitimate future body changes do not need to match extraction-era checksums. IPC signature and registration gates remain permanent.
5. Negative detector tests prove renamed commands, changed argument types, changed return types and changed async mode are detected; this is not merely a command-count check.

## Local validation

- Initial extraction suite: **5 passed**, including the one-time all-definition comparison.
- Final permanent extraction suite: **4 passed**.
- `cd launcher && npm test` using Node 24: **40 passed, zero failed/ignored** at the handoff checkpoint. This includes concurrent lead frontend work and does not attribute those other tests to this extraction.
- Scoped `git diff --check`: passes.
- No Cargo check/test was started: heavy native validation is explicitly coordinated with and owned by the lead. Files are ready for that integrated build; compiler/native acceptance is not claimed here.

## Limitations and remaining gates

- The source contract detector is intentionally tailored to the existing top-level Rust declarations and single-line Tauri command attributes. It is not a general Rust parser or a replacement for compiling Tauri-generated handlers. Future structural syntax changes require deliberate detector maintenance.
- `apply_patch` normalized the extracted files to LF despite CRLF payloads. Existing bodies, comments and layout were preserved otherwise; the one-time equivalence gate explicitly normalizes line endings. No wholesale rustfmt pass was applied.
- Native macro resolution, compiler diagnostics, OS-specific behavior, credential integration and existing account persistence changes need lead validation and independent review. No new performance, crash durability or release-readiness claim.
- Rollback is source-only: restore the pre-extraction facade and remove the new domain files/test in a reviewed change. No persisted paths, IDs, payload shapes, stores or schemas change.
