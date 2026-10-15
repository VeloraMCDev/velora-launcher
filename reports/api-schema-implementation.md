# VEL-009 bounded auth/Discord schema implementation

2026-10-12, parallel bounded worker. No dependency, identity, storage, endpoint,
release, deployment or automatic infrastructure changes.

## Implemented behavior

`velora_shared::launcher_contracts` adds typed connection configuration, Discord
start/connection/info/unlink, poll and authenticated response DTOs. Required data
is not defaulted. Unknown extension fields are retained, including auth user and
game-token extensions. Nullable connections stay `null`, not empty objects.
Auth optional omission versus explicit nullable game-token values is preserved;
legacy auth without optional fields still converts to existing `AuthResponse`
defaults (`pending=false`, no game tokens). Explicit null pending is rejected,
matching the existing auth contract. Discord info also retains omitted versus
explicit-null optional display names.

Poll decoding prioritizes presence of `token`, matching the existing launcher
branch, so a malformed auth payload cannot silently become a pending/link result.
Non-auth pending/link poll discriminants must be true. The producer only emits
these true values; malformed external false/non-object payloads now fail decoding.

`connections.rs` uses typed JSON responses for configuration, start, poll,
connection and unlink, wrapping the same existing producer functions. Existing
producer errors/statuses flow through unchanged. A producer contract violation
would now be an internal-server-error instead of emitting invalid successful JSON.
Ordinary success contents round-trip identically; object key order is not part of
the JSON contract. No OAuth data allocation, authorization, polling consumption,
expiry, credential or database logic changes. Community `discord::public_info`
is outside this worker's allowed server write scope and stays dynamic.

TS exports mirror Panel DTOs and define the separate token-free
`DiscordSignInPollResult` IPC union. These are declarations, not runtime validators.
The Rust host must validate before transforming/sending responses. This worker
does not modify or claim integration of the independently extracted commands.

## Producer → shared → Rust host → TS compatibility matrix

All evidence below is synthetic and located in
`crates/shared/src/launcher_contracts.rs::tests`. It is DTO fixture evidence copied
from inspected producer expressions, not an executed live Panel/router test.

| Producer / wire fixture | Shared contract and exact fixture evidence | Launcher Rust integration | TS consumer integration |
| --- | --- | --- | --- |
| communications `public_config`: `{discord_enabled:false,email_enabled:true}` | `ConnectionsConfig`; `configuration_and_start_preserve_producer_wire_and_extensions` exact round-trip | Public endpoint consumer integration pending lead verification | `ConnectionsConfig` exported; caller wiring pending |
| auth OAuth `start`: `{url,state}` | `DiscordStartResponse`; same test, plus future extension object round-trip | `discord_sign_in_start`, `discord_link_start` currently dynamic; lead should return DTO | `DiscordStartResponse` exported; SignIn/Settings inline annotation replacement pending |
| OAuth `linked`: null / `{id,name}` | `DiscordConnectionResponse`; `connection_preserves_null_and_linked_shapes`, including unknown field | `discord_connection` decoding pending lead | `DiscordConnectionResponse` exported; Settings wiring pending |
| OAuth unfinished/approval poll `{pending:true}`, linked poll `{linked:true}` | `DiscordPollResponse`; `pending_and_link_poll_preserve_discriminants_and_extensions` exact round-trip | `discord_sign_in_poll` typed match pending lead; same frontend IPC shape required | `DiscordSignInPollResult` exported; SignIn/Settings wiring pending |
| OAuth signed-in poll `AuthResponse` with `pending:false`, game tokens | `DiscordAuthResponse`; `auth_null_and_present_game_tokens_preserve_exact_wire` round-trip and conversion equality | `into_auth_response()` provided for existing `save_panel_account`; integration pending | Auth response is Panel-only; frontend must receive `{account}`, never tokens |
| Legacy auth lacks optional `pending`/`yggdrasil` | `legacy_auth_missing_optional_fields_preserves_omission_and_defaults`; existing `AuthResponse` decoding and new conversion both preserve defaults | Existing account auth consumers already typed; OAuth integration pending | Optional fields declared in Panel DTO only |
| Auth optional game tokens null; nested extension fields | `auth_null_and_present_game_tokens_preserve_exact_wire`, `auth_preserves_nested_unknown_extensions` | DTO serialization lossless; conversion intentionally drops extensions when entering existing internal account contract | Optional nullable game-token type exported |
| OAuth unlink `{ok:true}` | `DiscordUnlinkResponse`; `info_and_unlink_preserve_producer_wire` | Existing host discards successful body; typed validation integration pending lead | Export exists; no IPC payload change required |
| Discord community info `{invite_url,linked,display_name:null/string}`; legacy missing optional name | `DiscordInfoResponse`; `info_and_unlink_preserve_producer_wire` exact round-trips | `discord_info` decoding pending lead; producer migration outside scope | `DiscordInfoResponse` exported; Settings wiring pending |
| Malformed auth still contains pending/linked, missing required user fields, wrong nested types | `malformed_auth_cannot_fall_back_to_pending_or_linked`, `malformed_auth_user_and_game_token_types_are_rejected` | Prevents accidental success once lead uses typed decoder; host failure-path fixture pending | Runtime guarantees cannot be inferred from TS annotation alone |
| Missing/wrong required start/config/connection fields, invalid poll false/null/array/object, explicit-null auth pending | `malformed_required_fields_are_not_defaulted` | External validation provided by shared deserializer; IPC integration test pending | Caller error/retry behavior must still be validated |

## Validation and gates

First scoped run: `CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-shared`
passed 17 unit tests (9 new contract tests), zero failures/ignored; doc tests zero.
Two additional cases (legacy omitted info name and nested malformed auth types)
were added afterward. Final scoped rerun passed 18 unit tests (10 new contract
tests), zero failures/ignored; doc tests zero. The final log is
`/tmp/velora-api-schema-tests-final.log`.
Cargo encountered the shared target lock briefly; no broad/native/Panel/store build
was launched. Panel Rust syntax was parsed with rustfmt output to a temporary
file; that is not a Panel compile or route acceptance result.
The Panel file's existing CRLF text was normalized by the patch tool; semantic
review should use `git diff --ignore-space-at-eol` to distinguish this from the
bounded response changes. Scoped whitespace and new-module rustfmt checks pass.

Independent review is pending. Checkpoints 2–4 require actual command/client
integration, final consumer builds/tests, and independent compatibility review.
No claim that all Panel APIs are typed or that VEL-009 is complete.

## Exact worker-owned changed files

- `crates/shared/src/lib.rs` (one module export)
- `crates/shared/src/launcher_contracts.rs` (new DTOs and synthetic fixture tests)
- `panel/server/src/routes/connections.rs` (bounded response adapters)
- `launcher/src/lib/types.ts` (type exports only)
- `reports/api-schema-inventory.md`
- `reports/api-schema-implementation.md`

Other dirty files belong to parallel/previous work and are not this worker's edits.
