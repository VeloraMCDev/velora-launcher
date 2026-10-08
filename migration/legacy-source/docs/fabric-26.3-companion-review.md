# SCOPENET MC: Fabric 26.3 companion review

Reviewed October 1, 2026. This is a source review and product/architecture proposal, not a claim that the current client or Paper plugin has been compiled or exercised on 26.3. Existing behavior below comes from the launcher, panel routes, Paper handlers, shared integration/core code, companion client, build configuration, tests, and documentation. Proposed features are explicitly described as additions.

## Recommendation

Build **SCOPENET Companion**, a client-only Fabric 26.3 mod distributed through your launcher. Keep the Paper plugin as the gameplay authority and the panel as the account, progression, economy, guild, and community backend. Make the companion the in-game interface to those systems.

The strongest product improvement is that players can manage quests, guilds, trading, travel, and friends without leaving Minecraft. You already control distribution and already have most of the underlying rules and persistence. The missing piece is a complete in-game application and the APIs to drive it.

Paper does not load Fabric server mods. A Fabric client can connect to Paper when its installed mods preserve compatible networking and do not require server-side Fabric content. New screens, HUDs, client rendering, and vanilla-backed custom items fit this model. Arbitrary new registered blocks/items/entities, new world generation, and Fabric-only gameplay do not become available on Paper simply by installing a client mod. Gameplay changes still need Paper implementation using server-recognized content, or a separate Fabric server offering.

## What already exists

- `integrations/fabric-client` already contains a **1.20.1** client-only companion: five configurable HUD widgets, claim borders, Shop and Market screens, a menu, settings, a draggable/scalable HUD editor, keybindings, and selected notification toasts.
- `ClientLinkBridge` connects that companion to Paper through `scopenet:s2c` and `scopenet:c2s`. The player says hello, Paper sends feature flags and state, and shopkeepers/shop blocks can open native client screens with chest GUI fallback for players without the companion.
- `ClientLink` and `Wire` hold the platform-neutral protocol. Client actions currently send ordinary commands; Paper still checks the gameplay action.
- State includes global/server level summaries, title/rank, balance, guild summary, daily/weekly completion totals, playtime, kills, and deaths. Claims are a **9 × 9** local grid with three cell states, plus the current chunk owner.
- The panel and launcher support substantially more detail than that protocol exposes. Expanding the protocol is essential; building screens alone will not unlock those systems.
- The repository's **26.3 Fabric server integration is a different artifact**. It includes shared authentication/activity/claims/map integration but does not include the 1.20.1 Fabric feature module or this companion UI.

Primary evidence: [client initializer](../integrations/fabric-client/src/main/java/net/scopenet/client/ScopenetClient.java), [client state](../integrations/fabric-client/src/main/java/net/scopenet/client/ClientState.java), [shared link](../integrations/common/src/main/java/net/scopenet/core/ClientLink.java), [Paper bridge](../integrations/paper/src/main/java/net/scopenet/paper/ClientLinkBridge.java), [modern Fabric build](../integrations/fabric/modern.gradle).

## Launcher and platform feature review

### 1. Instances, installs, modpacks, and repair

**Existing:** published instances with access restrictions; vanilla and loader installation; automatic Java; Modrinth/CurseForge/ZIP pack imports; hosted files/configuration; shared downloads and hashes; progress, cancellation, repair, cache/offline launch, per-instance options, and game lifecycle tracking.

**Enhancement:** publish a managed 26.3 Fabric instance containing the companion and its exact compatible Fabric API. Add a preflight that checks Minecraft, Java, loader, companion protocol range, required pack revision, and server compatibility before Quick Play. Show a useful missing/incompatible companion diagnosis in the launcher. Preserve personal HUD/keybind preferences during managed config updates.

**Required work:** companion metadata in the instance/release manifest; build and package the client JAR; define required versus optional files and distinguish managed defaults from user configuration. A curated client modpack can follow after compatibility testing. This proposal does not assume particular third-party mods are 26.3 compatible.

### 2. Accounts, private authentication, and access control

**Existing:** panel login, registration/approval modes, groups/roles, encrypted launcher tokens, offline local accounts, authlib-injector, stable UUID identity, reserved previous names, disabled-account/revocation handling, server access rules, and optional authenticated-launch checks. Official servers require online mode under the private authentication setup.

**Enhancement:** welcome/onboarding screen, current account/server identity, access explanations, and a connected-services status view. Hide unavailable features using server-issued capabilities.

**Required work:** bind all gameplay requests to the authenticated Minecraft connection. Never give the companion the game server's panel token or copy the launcher's persistent account credentials into mod configuration. A mod-version claim is compatibility information, not proof that a client is trusted. Offline play must not expose private panel data.

### 3. Play flow, server status, and multiple servers

**Existing:** server list injection, Quick Play/auto-join, MOTD/ping/player status, linked instances, public server discovery, game invites, network-wide identities, global/server progression, and configurable shared economy groups.

**Enhancement:** branded title/pause screen entry, server browser showing friends and compatibility, reconnect guidance, and an in-game invitation that offers an appropriate join path. Show the active server and economy scope in every financial screen.

**Required work:** resolve instance/server IDs explicitly. Use normal disconnect/connect for compatible destinations; use a launcher handoff when another server needs a different instance. Seamless transfers need proxy/network implementation; a client button alone cannot provide them. Balances are shared only for configured economy groups, not automatically across every server.

### 4. Branding, news, and community information

**Existing:** remote launcher branding, logos, palettes, fonts, corners, backgrounds, custom CSS, news/social links, About content, title/rank PNGs, achievement and quest icons, and a configurable public landing page.

**Enhancement:** native themed navigation, news drawer, guild banners, rank emblems, achievement artwork, first-join guide, and community links. Allow players to reduce animation and adjust contrast/text scale.

**Required work:** add a bounded native theme schema and asset manifest. Svelte components and arbitrary launcher CSS cannot be reused as Minecraft screens. Prefer packaged/resource-pack artwork; validate and cache any remote images with size limits and a known source. Keep decorative launcher backgrounds optional in game.

### 5. Settings, controls, performance, and diagnostics

**Existing:** memory recommendations, Java/GC/JVM settings, resolution/fullscreen/FOV, global Minecraft keybind editing, per-instance overrides, UI preferences, storage tools, console/crash hints, repair, self-update, and launch window behavior. Companion has its own JSON preferences and HUD editor.

**Enhancement:** per-server HUD layouts, safe keybinding defaults, localization, keyboard navigation, adjustable UI scale, configurable notification channels, claim overlay distance, optional performance widget, and an exportable redacted support summary including companion/protocol/pack versions.

**Required work:** teach the launcher editor about companion keybindings; reconcile preferences without overwriting them every launch. JVM/memory/storage/update controls stay in the launcher. Client FPS/ping and server TPS are distinct metrics; expose server performance only when authorized. New JARs activate on the next game start rather than hot-reloading executable code.

Evidence: [launcher commands](../launcher/src-tauri/src/commands.rs), [game launch](../launcher/src-tauri/src/game.rs), [shared manifest types](../crates/shared/src/lib.rs), [launcher pages](../launcher/src/pages), [panel routing](../panel/server/src/routes/mod.rs).

## Gameplay feature review

### 6. Global XP, server XP, ranks, titles, stats, and leaderboards

**Existing:** activity ingestion, global and per-server XP, configurable multipliers and milestones, title rewards, rank/permission integration, playtime/kills/deaths and other recorded activity, personal stats and leaderboard views. Current companion shows a small level/balance/guild summary.

**Enhancement:** character sheet with separate global/server progress, reward milestone track, rank artwork, detailed stats, filtered leaderboards, and optional recent XP gains. Keep vanilla experience visibly separate from SCOPENET progression.

**Required work:** detailed read endpoints/payloads, next reward summaries, server scope, pagination, and event updates. Server-recorded activity remains authoritative; the mod must not report client-observed actions as proof that XP was earned.

### 7. Daily/weekly quests and achievements

**Existing:** panel editors, objectives/progression, reward bundles, player quest retrieval, launcher claim actions, achievements/icons, `/quests`, selected unlock toasts. Current mod receives completion counts rather than objective lists.

**Enhancement:** quest journal, objective-specific pinned tracker, reset countdowns, reward preview, claim button, achievement collection with filters and artwork. Example: pin “Mine 64 iron” and see its authoritative progress while playing.

**Required work:** quest definitions, per-objective progress, server timestamps/reset deadlines, claim eligibility, claim result, and achievement metadata. The authenticated launcher quest-claim route already exists, but the game-server route set currently needs a suitable authenticated-player bridge for claiming. Display countdowns from server deadlines rather than the client timezone.

### 8. Reward bundles and delivery

**Existing:** XP, money, additional guild claim allowance, badges, ordinary/custom items, permissions, groups, messages, optional console-command rewards, queued server delivery with acknowledgements/retry logs.

**Enhancement:** reward reveal screen and delivery history with **earned**, **queued**, **delivered**, and **failed** states. Link earned kits/items/perks to their relevant screen. Celebrate only confirmed completion.

**Required work:** player-safe reward history and status events. Preserve the existing delivery ledger/retry rules. UI retries must reuse operation identity; opening a reward animation must never issue another grant. A reward mailbox with inventory-capacity handling would be a separate delivery change—the current custom item reward path can drop overflow at the player.

### 9. Homes, spawn, back, RTP, and warps

**Existing:** local saved homes/warps, home limits, cooldowns, spawn/back/death return, safe random teleport, chest menus, public warp requests and staff approvals.

**Enhancement:** travel screen with saved destinations, dimension labels, favorites, home slots, cooldown timers, and confirmation for deleting a home. Add approved warp search and a staff review queue. A death waypoint can help navigation without authorizing an otherwise unavailable `/back`.

**Required work:** Paper-backed destination/cooldown/eligibility reads and typed actions. Keep teleport safety and destination validation on Paper. Screenshots/warp categories are new metadata, not currently available. Expose private homes only to their owner or explicitly authorized staff.

### 10. Teleport requests

**Existing:** `/tpa`, accept/deny and expiration; launcher map actions can enqueue a teleport request.

**Enhancement:** request popup with sender profile, accept/decline buttons, expiration timer, and outgoing request state.

**Required work:** request ID/lifecycle events and authoritative expiration. Client acceptance does not bypass cooldown or server rules. Distance disclosure should match existing visibility policy.

### 11. Personal economy, payments, shops, and selling

**Existing:** scoped balances, player payments, baltop, configurable currency symbol, server shop/sell GUIs, held-item selling, transaction records in the panel, and inventory/payment operations with durable pending-operation handling. The companion has a simple shop screen and held-item price.

**Enhancement:** searchable catalog with full item tooltips, quantity and total-price preview, personal wallet history, payment confirmation, and a server-issued sell quote before committing a batch sale.

**Required work:** expose authoritative catalog/price/stock rules and transaction history, stable item previews, quote revision, typed outcomes, and money representation that matches backend precision. Server prices and inventory checks win over all displayed values. Preserve the existing escrow/operation flow.

### 12. Player market, auctions, active orders, and mailbox

**Existing:** fixed-price listings, auction creation wizard, bids with held funds/outbid refunds, late-bid extension, cancellation rules, mailbox item delivery, join reminders, and listing snapshots in Paper GUIs.

**Enhancement:** market browser with search/sort/filter/pagination, auction-specific bid controls, end times, minimum bid, your listings, purchase confirmation, outbid notifications, and mailbox with exact item previews.

**Required work:** extend companion listing DTOs with listing type, expiry, bid state, revision, and safe item metadata. Current companion treats every row as a Buy row and does not model auctions. `/orders` currently only prints usage guidance; replace it with a genuine listing query/view. `/transactions` currently shows balance and points to the launcher, rather than returning a history. These are backend/UI completion tasks, not just cosmetic screens.

### 13. Player-to-player trade

**Existing:** Paper has a shared inventory trade session with side restrictions, readiness, item transfer, and cancellation/return logic. The companion has no native trade screen.

**Enhancement:** begin with an improved presentation of the existing server-managed container. Later add explicit invitations, exact item previews, changes-reset-acceptance behavior, and a clear final confirmation.

**Required work:** preserve server container IDs, slot rules and click semantics. For a fully native trade UI, implement a server-owned session ID and offer revisions before replacing inventory interaction. Money offers, a confirmation countdown, invitations, and durable crash recovery are additional work; do not assume they exist because the command is called secure trade. Test cancellation, disconnects, shift-clicks, drag operations and stale acceptance before shipping.

### 14. Guild discovery and membership

**Existing:** guild creation/directory/details, join requests, invites/accept/decline, member roster, kicks, promotions/demotions, custom role assignment, leadership transfer, MOTD/description, posts, guild chat, and launcher icon/banner settings.

**Enhancement:** full guild hub with roster/presence, invite/request inbox, permissions-aware role controls, announcement feed, identity artwork, and explicit confirmations for leadership transfer/disband.

**Required work:** detailed guild/member/role DTOs and actions, including new game bridge operations where only authenticated launcher endpoints exist. Current companion only knows guild name, tag, role and claim count. Do not infer every permission from a role label: ask the authority for allowed actions. Alliances, wars and diplomacy need a separate review/implementation rather than being assumed from command descriptions.

### 15. Guild bank and guild commerce

**Existing:** bank balance/history, deposits/withdrawals, guild-funded market operations, guild sales proceeds and inter-guild payments; network economy group scoping.

**Enhancement:** treasury screen with ledger, personal-versus-guild spending selector, bank permissions and explicit payer/recipient confirmation.

**Required work:** bank DTOs and action results with guild/server/economy scope. Current client market's balance is personal and its guild tag presentation does not provide a complete guild-purchase authorization model. Show the account actually being charged and let Paper/panel enforce membership and spending rules.

### 16. Guild land, autoclaim, admin claims, and protection

**Existing:** authoritative panel claims, local server claim index, block/interaction/environment protection listeners, guild limits, launcher territory painter, autoclaim with failure stop, admin-owned claims and area editing, claim-entry banners. Companion renders nearby claim borders.

**Enhancement:** claim planning mode with contiguous border outlines, owner/relation/permission labels, allowed action indicators, cost/limit preview, autoclaim status, and a native territory editor. Add staff-only admin-claim naming/description tools.

**Required work:** distinguish owner from permission. Current link uses `allowed()` as both the green cell and `owner.mine`, which can label bypass-accessible land as “Your land.” Add owner type/ID, relationship and effective permissions separately. Admin claim descriptions/colors and guild relations are not represented by the three-character grid. Previewing/selecting chunks must not claim them until the server validates the action. Rendering should be capped and optional; protect gameplay on Paper whether the overlay is present or not.

### 17. Live map, pins, dimensions, and player visibility

**Existing:** region-file terrain rendering, tile uploads/caching, zoom levels, world overlays, player positions, guild/admin claims, spawn/warps/guild homes/shop points, public map configuration, launcher profile/teleport/message/guild-invite actions.

**Enhancement:** in-game full-screen map and optional minimap sharing SCOPENET tiles, destination waypoints, travel links and claim layers. Server tiles provide broader coverage than local observation, subject to access policy.

**Required work:** authorized player-scoped tile access, explicit world IDs, visible-player/pin filtering, bounded asynchronous texture decoding/cache and revisions. Use short-lived scoped URLs for large tiles; avoid dumping image data into plugin messages. Existing tile API authentication cannot simply be bypassed by a mod URL. Server terrain may lag while unsaved/changed chunks wait for rendering; show freshness. Audit extra-world handling: current Paper position snapshots collapse worlds into environment-based dimension names while other paths use world keys. Match player positions and tiles to actual world identity. Private homes, vanished players and protected information need explicit visibility rules before richer overlays expose them.

### 18. Custom items, models, textures, sounds, and fonts

**Existing:** panel item definitions with vanilla base, name/lore, enchants, attributes, flags, model data, texture/model; captured kit item metadata; custom-item grants/rewards; generated resource packs, asset uploads and revision polling. Pack generation emits legacy predicates plus modern item definitions.

**Enhancement:** rich tooltips, collectible catalog, item inspection, 3D previews, consistent market/reward/kit art, optional held-item animation and cosmetic effects. Model resources already provide much of the visible customization without requiring client registration of new item types.

**Required work:** stamp a stable SCOPENET custom item identity/revision into server-controlled item metadata and send a safe render description. `ItemSpec` and the inspected Paper styling path do not establish such an identity; display name/model number alone are insufficient. Current market preview reconstructs a base item from ID/count, losing full custom appearance. Keep authoritative item snapshots on Paper and never send platform serialization for the client to deserialize blindly. Audit 26.3 components and resource-pack metadata; default `pack_format` is 15, so current defaults are not a 26.3 pack contract. Preserve complex vanilla item state behavior such as bows/shields. Cosmetic effects are feasible client-side; abilities/damage/cooldowns/recipes must be server implemented and validated. Genuine new registered items require compatible server support.

### 19. Shopkeepers and NPCs

**Existing:** persistent shop points and shopkeeper mob entities with AI disabled; spawn/link/remove/list commands; protected interaction opening shop/market; map pins. This is a shopkeeper system, not a general quest/dialogue NPC framework.

**Enhancement:** NPC nameplates, hover prompts, profession portraits, a conversation/shop window, quest turn-in NPCs, guild registrars and travel guides. Use real server entities as interaction anchors; enhance their appearance optionally on modded clients.

**Required work:** NPC identity/role/dialogue schema, eligible actions, server entity linkage, visibility, distance and cooldown validation. Branching dialogue and quest-giving are new systems. Complex animated replacement models/skins need a rendering implementation and tested fallback. Client-only entities cannot grant rewards or establish authoritative hitboxes.

### 20. Heal, feed, flight, kits, vaults, and ender chest

**Existing:** configurable utilities, permission/cooldown restrictions, free or guild-land flight, vault permissions/storage, remote ender chest, kits with cooldown/one-time/group rules, in-game kit capture excluding armor/offhand, and metadata-preserving item snapshots.

**Enhancement:** perks screen with eligibility/cooldowns, kit catalog with item previews, vault selector, and flight indicator/warning when leaving permitted land.

**Required work:** player eligibility and kit preview reads. Keep inventory contents and moves on the normal server-managed container path; do not make a separate client vault inventory. Search/sorting must not send illegal inventory operations or reorder shared contents without server support. Permission revocation and guild-bound flight remain server enforced.

### 21. Chat, nicknames, and shared items

**Existing:** configurable format, guild/title/rank prefixes, LuckPerms decorations, supported text styling/colors with permission checks, persistent nicknames, guild chat, clickable `/hand` and read-only item snapshots.

**Enhancement:** channel controls, readable rank/guild decoration, click-to-profile actions, native read-only item inspection and accessibility options. Add mention notifications only as a deliberate new behavior.

**Required work:** structured display metadata rather than scraping rendered prefixes. Preserve command/chat permission checks, message ordering and vanilla chat behavior. Existing shared-item snapshot expiration should remain enforced on Paper. Launcher DMs and in-game chat are different data paths; label them accordingly.

### 22. Friends, DMs, profiles, posts, and game invitations

**Existing:** friends/request approval/removal, member search, online/server status, persistent direct messages/unread handling, invitations tied to instance/server, profiles/badges, profile posts/likes, guild feed, and launcher social sidebar.

**Enhancement:** friends drawer, inbox, read-only profile cards, invitation actions, badge display and optional post/feed browser. Prioritize friends/inbox/invites; full social publishing can remain in the launcher until the core game experience is polished.

**Required work:** new player-scoped game bridge operations for social features. Some game API helpers exist, but most launcher social routes require account authorization and are not exposed through the client link. Bind sender identity server-side, preserve read/unread behavior, authorize recipients/history, and add polling limits or subscriptions. An optional launcher IPC bridge must work when the launcher hides or closes; gameplay should not depend on a live desktop process.

### 23. Notifications, Discord, email, and scheduled tasks

**Existing:** persistent notification inbox, selected in-game panel messages/toasts, Discord linking/sign-in and role sync, announcement templates/live boards/slash commands, email templates/reset/broadcasts, and scheduled backend tasks.

**Enhancement:** one in-game notification drawer for invitations, outbids, reward delivery, quest completion and guild announcements; mute categories; connected Discord status and a link to the existing account connection flow.

**Required work:** notification list/read operations and event IDs for deduplication. The existing companion handles only selected event kinds; it is not the full notification inbox. OAuth/email sending, role synchronization, broadcasting and scheduler execution stay in the panel. Never embed bot credentials or administrator controls in player payloads. Countdown announcements require explicit new task/event wiring if desired.

### 24. Plugin integrations, developer API, and administration

**Existing:** LuckPerms permission/rank sync, PlaceholderAPI expansion, Vault economy provider, CoreProtect/WorldGuard/spark reporting, SCOPENET Java API/events, panel player/server/claim/content administration, account purging, server token management and deployment/release workflows.

**Enhancement:** permission-aware command palette, relevant status/claim information, and an optional staff overlay showing authorized integration/health information. Publish companion extension events so another Paper plugin can contribute a safe screen action or display field through a server adapter.

**Required work:** separate staff/player capabilities; limit extensions to declared typed fields and allowlisted actions. Placeholder strings are not an authorization API. Reporting integrations do not automatically provide rollback, WorldGuard editing, or staff moderation workflows. Account deletion must also invalidate cached companion state; plugin-local homes/vaults require their own cleanup policy. Keep heavyweight editors and system administration in the panel.

## Architecture to build

1. **Panel:** durable identities, content definitions, progression, guilds, scoped economy, social records, notifications and reward queues.
2. **Paper:** authenticated player identity, live world/inventory state, commands/actions, permissions, claim protection, NPC interactions and item delivery. Relay appropriate panel information; never expose the server token.
3. **Fabric companion:** native screens, presentation state, HUD/world rendering, local preferences and requests for authorized actions.
4. **Launcher:** install/version/repair/update, private sign-in, instance selection and preflight. Optional convenience IPC may help with another instance, but the companion-to-Paper connection must function independently.

Use plugin messages for small gameplay requests and responses; Fabric's modern custom payload codecs must encode/decode the same bytes as Paper. The old VarInt-prefixed JSON format is a compatibility starting point, not an instruction to use Java `DataOutput.writeUTF`, which has different framing. Register only payloads the Paper bridge actually supports; avoid mandatory server-Fabric registries/configuration dependencies.

Proposed protocol v2:

- `hello`: protocol range, companion build, Minecraft version, supported UI modules; response selects protocol and sends server identity, capabilities, theme/content revision and limits.
- `request`: request ID, allowlisted operation, bounded typed parameters, expected revision where needed.
- `response`: request ID, success/error code, readable explanation and updated authoritative state.
- `event`: event ID, domain revision and bounded state delta; deduplicate and resync after gaps.
- Paged domain reads: quests/achievements, guild/roster, market/mailbox, wallet/history, travel, perks, friends/notifications, map metadata and item catalog.
- Action operations include quest claim, listing/bid/cancel, mailbox claim, payment, guild management, claim edit and teleport response. Reuse command/service implementations; do not create another implementation of the rules.

For v1-compatible migration, ordinary commands can power early buttons. Native workflows need structured results so the UI can distinguish failure, stale data, timeout and success. Financial actions need idempotency and authoritative price/offer revisions. Trade needs a separate server-owned revisioned session.

Read requests run asynchronously with bounded concurrency; Bukkit world/inventory access returns to the main thread. Subscribe only to domains in use; cache/paginate shared reads; push changed data and invalidate after actions. Current profile cache refreshes around every 20 seconds and the link ticks about once a second, so those are not instantly fresh by default. Add explicit unavailable/loading/stale states rather than showing cached balances as confirmed.

Set conservative payload/page limits below the most restrictive supported channel limit and test actual 26.3 transport. The current wire accepts up to 98,301 JSON bytes; a codec's string limit alone does not establish that every Paper channel and client version accepts that payload. Bulk images/models belong in resource packs or scoped downloads.

## Concrete issues to address first

1. **26.3 client build absent.** Client Gradle pins 1.20.1/Java 17/Loom 1.8.13/old Fabric API; modern build selection applies to Fabric/Forge server projects, not `fabric-client`. Release workflow builds those server loaders and Paper, omitting the companion client.
2. **Version negotiation absent.** Server sends `Wire.VERSION`, client sets connected without checking it; client advertises a hardcoded `0.1.0`. Incompatible peers can appear connected.
3. **Auction UI incomplete.** Client listing record omits auction state and renders Buy for every row. Native market needs bid-aware presentation and results.
4. **In-game history incomplete.** `/orders` prints instructions; `/transactions` directs players to launcher history. Their command guide descriptions overstate the in-game detail.
5. **Ownership/access conflated.** `allowed()` feeds `owner.mine` and green claims; bypass access is not ownership. Add explicit ownership and permission fields.
6. **Shopkeeper fallback incomplete.** Paper considers the client present after hello; client ignores an `open` when disabled or another screen is up. Add an enabled-screen capability and open acknowledgement/decline so the player still gets an interface.
7. **Hello flood exemption.** `hello` bypasses the current 400 ms request rejection and can trigger repeated handshake/claims output and refresh attempts. Rate-limit handshakes too; negotiate once per connection with bounded retries. Client actions sent as commands are outside this read-request limiter and still need normal operation throttling.
8. **Custom item preview/identity incomplete.** Base ID/count preview does not preserve item metadata; inspected item styling has no stable custom-definition identity. Introduce safe item DTOs and identity before a collectible economy relies on them.
9. **Pack defaults and Paper compatibility unverified.** Default pack format is 15; Paper compiles against Spigot 1.20.1. Intended broad API compatibility is not runtime evidence for all item components, reflection, containers and resource packs on Paper 26.3.
10. **World/visibility contract needs definition.** Position snapshots use dimension-by-environment whereas other code uses world keys. Resolve multiple worlds, private homes and player visibility before making the map a central UI.
11. **Trade needs server hardening before a richer UI.** `TradeSession` ignores the overflow maps returned by `Inventory.addItem` on completion and cancellation, creating an item-loss risk with a full receiving inventory. Offer edits reset only the editing player's ready flag; a player who accepted the earlier offer can remain ready while that offer changes. Invalidate both acceptances on every offer revision. The inspected handler also needs explicit drag/shift-click coverage rather than relying on raw clicked-slot checks. These concerns exist in the Paper trade implementation and cannot be fixed by client presentation alone.

These findings are scope/implementation observations, not an exhaustive security audit. No application code was changed by this review.

## 26.3 port requirements

Fabric's September 15, 2026 guidance identifies Loom 1.17, Gradle 9.6.0 and Loader 0.19.5 at publication. Use Java 25 as your repository's 26.x build/runtime target; choose and pin a matching Fabric API 26.3 build during implementation. Reconfirm exact versions then.

26.3 also replaces GLFW with SDL. Your client initializer currently imports GLFW key constants, so input code needs changes rather than copied constants. Custom text input needs correct focus integration. Rendering changes require porting HUD/world overlays; modern networking uses registered payload types/codecs, replacing the old channel/buffer callbacks. Test the generated resource pack and custom item components against the target release.

Official references: [Fabric 26.3 changes](https://fabricmc.net/2026/09/15/263.html), [Fabric networking](https://docs.fabricmc.net/develop/networking), [Paper plugin messaging](https://docs.papermc.io/paper/dev/plugin-messaging/). Networking documentation is versioned/evolving; verify exact 26.3 signatures rather than copying a different version's tutorial verbatim.

## Delivery sequence

### Phase 1 — Prove the platform

Port existing companion HUD/menu/shop/market and claim rendering to 26.3; build and distribute it; verify a real Paper 26.3 connection under private authentication. Add negotiated capabilities, safe reconnect/reset, bounded requests, structured outcomes, item preview schema, disabled-client fallback and typed claim ownership. Keep existing chest/command fallback working.

**Exit check:** modded and vanilla players can join; incompatible/disabled clients fail gracefully; two clients can perform existing market actions without money/item regression; restart/reconnect does not leak previous server data.

### Phase 2 — Make the daily loop usable

Quest journal/pinned objectives, achievements/milestones, travel/TPA, wallet history, real orders, auctions/mailbox and kit/perk catalog. Build one polished hub with navigation rather than a collection of unrelated popups.

**Exit check:** a player can earn and claim a quest, preview/receive its reward, sell/list/bid/collect an item, travel home and respond to TPA without opening the launcher. Server rejection always leaves a clear result and authoritative state.

### Phase 3 — Guild and world experience

Guild roster/roles/requests/feed, bank and guild commerce, territory planning/editing, admin claim presentation, authorized full map/minimap and waypoints.

**Exit check:** role restrictions, shared economy scopes, multiple worlds, claim limits and map privacy match server policy. UI refreshes after actions without making large per-player panel polling loops.

### Phase 4 — Community and identity

Friends/inbox/invitations, profiles/badges, notification drawer, theme/art delivery, full custom-item previews and improved trade presentation.

**Exit check:** DMs and notifications retain correct read state; cross-instance invitations have a valid launcher path; UI works with the launcher closed; trade cancellation and stale acceptance are safe.

### Phase 5 — New content presentation

NPC dialogue/roles, optional cosmetic renderers/animations, polished onboarding, optional staff tools and companion extension API. Add new abilities or quest-NPC rules on Paper as explicit gameplay projects.

**Exit check:** every NPC/content action is server validated, resource-pack fallback remains usable, and client cosmetics do not change authoritative combat or protection rules.

Do not begin by rewriting all backend logic or switching the primary server to Fabric. The reusable investment is your panel and Paper rules; concentrate new work on the client UI, domain read models, action results and compatibility testing.

## Validation for implementation

- Unit/contract tests for framing/codecs, negotiated capabilities, malformed/oversized payloads, handshake throttling, action authorization and stale revisions. Extend existing `LinkTest`, `UtilityHubTest`, reward/map/operation tests where appropriate.
- Run the existing Rust/panel and Java shared-core suites for affected domains; inspect Paper-specific integration behavior rather than assuming shared tests cover it.
- Real dedicated Paper 26.3 with two Fabric clients and a vanilla client: authentication, NPC fallback, pack acceptance/decline, auction bids/outbid refunds, full inventory/mailbox, queued reward retries, duplicate clicks, stale listing IDs, reconnect and panel outage.
- Permissions: owner/member/officer/custom role/staff/bypass; private versus public map; disabled modules; revoked access; old/new protocol.
- UI: small window/large GUI scale, keyboard-only use, non-English text, disconnected loading state, text focus under SDL, HUD overlap, capped overlay rendering and a large market/guild.
- Performance: panel request counts per player and active screen, server tick time, client frame time and texture memory. Do not make every player poll every domain every second.

The best first playable milestone is **one native SCOPENET hub with a quest journal, auction/mailbox browser, travel screen, and guild summary**, backed by the negotiated Paper bridge. That turns your existing server systems into a cohesive game experience while preserving the foundation you already built.
