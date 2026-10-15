# VEL-009 launcher-consumed Panel API ownership inventory

Bounded schema worker, 2026-10-12. The root monorepo, boundaries, validation and
deployment instructions were read. Inventory uses current extracted command
modules and the actual Panel producers, not frontend annotations as a schema
authority. All paths below are relative to `/api/v1` unless otherwise stated.
No stores or provider endpoints were contacted.

## Endpoint families and response ownership

| Launcher owner | Consumed endpoints / families | Panel producer owner | Existing contract / remaining dynamic boundary |
| --- | --- | --- | --- |
| `commands/bootstrap.rs`, core API client | `/launcher/manifest`, instance file/preparation metadata | `routes/public.rs` | Shared manifest/instance/file contracts; branding and experience remain existing contracts |
| `accounts.rs`, `commands/accounts.rs` | `/auth/login`, `/auth/register`, `/account/username` | `routes/account.rs` host adapter → `velora_auth_http::web` | Existing `AuthResponse`, `PublicUser`, `YggdrasilTokens`, login/register requests; error extraction still dynamic |
| `commands/accounts.rs` | `/auth/discord/start`, `/auth/discord/start?kind=link`, `/auth/discord/poll` | `routes/connections.rs` → `velora_auth_http::oauth` | Selected bounded migration: start, pending, linked and authenticated poll DTOs |
| `commands/accounts.rs` | `/account/connections/discord` GET/DELETE | `routes/connections.rs` → `velora_auth_http::oauth` | Selected nullable connection and unlink DTOs |
| Sign-in/public configuration consumer | `/auth/connections/config` | `routes/connections.rs` → `velora_panel_communications::settings::public_config` | Selected boolean configuration DTO; public endpoint client wiring still needs lead verification |
| `commands/accounts.rs` | `/account/discord`, `/auth/forgot-password` | `routes/discord.rs::public_info`, `routes/connections.rs` → auth reset | Discord info DTO exported only; reset remains dynamic `{message}` extraction, out of migration scope |
| `commands/profiles.rs` | `/account/profile`, `/account/skin`, `/account/skin/model`, `/account/cape` | `routes/account.rs` auth/profile adapters | Shared `PlayerProfile`; multipart, model/cape mutation and local saved skin profiles are distinct boundaries |
| `commands/profiles.rs` | `/profiles/{uuid}`, `/profiles/me`, profile posts, `/posts/{id}/like` | `routes/social.rs` | Shared profile/post models where used; error JSON remains dynamic |
| `commands/social.rs` | `/friends`, `/friends/request`, `/friends/respond`, `/friends/{uuid}`, `/social/members`, `/messages/{uuid}`, `/invites`, `/invites/{id}/respond` | `routes/social.rs` | Shared friend/message/invite/member models; mutation/error JSON is not all typed |
| `commands/social.rs` | `/notifications`, `/notifications/read` | `routes/notifications.rs` | Notification response remains dynamic; not migrated |
| `commands/progression.rs` | `/levels/me`, `/reward-bundles`, `/quests/my`, `/quests/{id}/claim`, `/achievements/my`, `/collections/me`, `/collections/equip` | `routes/leveling.rs`, `rewards.rs`, `quests.rs`, `achievements.rs`, `collections.rs` | Level/quest/achievement types coexist with dynamic reward summaries, collections and errors |
| `commands/economy.rs` | `/economy/me`, `/servers/{id}/economy/baltop`, `/economy/transactions`, `/account/stats`, leaderboard/public server responses | `routes/economy.rs`, `servers.rs`, `public.rs` | Shared balance/baltop/transaction types; statistics/server/leaderboard responses remain dynamic |
| `commands/guilds.rs` | `/guilds` and membership, primary, relations, claim-flags, requests, roles, claims, wallet, invites, upgrades subresources | `routes/guilds.rs`, `guild_flags.rs`, `guild_bank.rs`, `factions.rs` | Shared types in some commands; several host-owned/dynamic gameplay payloads remain |
| `commands/market.rs` | `/market/{server_id}`, `/market/{server_id}/mine`, action subresources | `routes/market.rs` | Dynamic marketplace JSON; not migrated |
| `commands/casino.rs` | `/casino/{server_id}{path}`, `/vaults/{server_id}`, vault actions, `/board/{server_id}{path}` | `routes/casino.rs`, `vaults.rs`, `orders.rs` | Deliberately dynamic routed gameplay/action JSON; not migrated |
| `commands/maps.rs`, texture commands | `/servers/{id}/map`, `/servers/{id}/map/overlay`, `/servers/{id}/map/actions`, resource/icon/texture endpoints | `routes/worldmap.rs`, `mc_assets.rs`, `resource_assets.rs`, `icon_library.rs` | Map/action JSON and binary/resource boundaries are separate; not migrated |
| Update, telemetry and core preparation owners | Launcher/core update metadata and artifacts, telemetry, Java/authlib resources | `routes/launcher_updates.rs`, `core_releases.rs`, `public.rs`, `account.rs` | Existing signed release/core and preparation contracts stay untouched; binary responses are not JSON DTOs |

The table is an owned family inventory, not a complete OpenAPI catalog or a claim
that every per-game action has a fixed shared schema. The generic Panel client
still returns `serde_json::Value` in several areas. Extension points must not pull
gameplay implementations into neutral contracts.

## Selected producer facts

- OAuth `start` returns exactly `{url, state}`. The optional query `kind` changes
  authorization behavior, not the success shape; callback is HTML, not JSON.
- OAuth `poll` returns `{pending:true}`, `{linked:true}`, or serialized existing
  `AuthResponse`. Pending approval and unfinished OAuth both use the same pending
  shape and are intentionally not distinguishable by this schema.
- OAuth `linked` returns `null` or `{id,name}`; Discord IDs remain strings.
  OAuth `unlink` returns `{ok:true}`.
- Public configuration returns both boolean flags. Discord public info returns
  `{invite_url,linked,display_name}` with explicit null when no name is linked.
- The host transforms authenticated OAuth into `{account}` over Tauri IPC;
  Panel authentication tokens must not be forwarded to the frontend. Existing
  credential storage/save ownership is not part of this worker.

## Required lead integration / independent review

Keep HTTP paths, status checks, error handling, query names, IPC command names and
account persistence ownership unchanged. Decode Panel poll as `DiscordPollResponse`,
convert its authenticated DTO with `into_auth_response()`, then save through the
existing account owner before returning the existing `{account}` IPC shape.
Use the start/nullable connection/info exports in extracted command modules and
replace inline annotations in `SignIn.svelte` and `Settings.svelte` with shared TS
exports. No command, store or frontend component was modified by this worker.

Review endpoint-client wiring, Panel compilation/routing, frontend typechecking,
host IPC/account compatibility and malformed-response handling independently.
Inventory supports checkpoint 1 only. Checkpoints 2–4 remain conditional on lead
consumer integration and compatibility validation; this report does not award them.
