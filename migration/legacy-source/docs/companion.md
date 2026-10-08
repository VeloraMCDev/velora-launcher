# SCOPENET Companion for Fabric 26.3

The companion gives SCOPENET players a native Minecraft interface while Paper and the panel own gameplay, inventories, permissions and money. Its client entrypoint is separate from the existing Fabric server integration and the legacy 1.20.1 client.

## Install and distribute

1. Update the panel and SCOPENET Paper plugin together from this branch. The panel adds `/api/server/v1/companion` and a migration for mutation receipts. Keep `clientlink.enabled` enabled in Paper.
2. Create a Fabric **26.3** launcher instance using Java **25**, Fabric Loader **0.19.5 or later**, and Fabric API **0.161.0+26.3 or later for 26.3**. Fabric API prerelease builds with the same `+26.3` suffix are not interchangeable with the release API.
3. Build/download `scopenet-client-fabric-26.3-<version>.jar`. Upload it and Fabric API as instance files in the panel, targeting the instance's `mods/` directory, then publish the instance revision. The launcher synchronizes these files through its existing pack system. Do not put the client JAR in Paper's plugins directory or use the Fabric server integration as a client mod.
4. Deliver the server's matching 26.3 resource pack through the existing pack configuration. Item models and display-entity models continue to come from that pack.
5. In Launcher **Settings > Companion**, configure modules, scale, opacity, notifications and claim outlines. In **Settings > Controls**, enable launcher-managed bindings and choose quick actions. Settings apply on the next launch.

The release workflow builds and collects the companion with the 26.3 artifacts. It remains manually triggered; this PR does not publish a release or modify a live instance.

## Player interface

Press **K** for the overview, or open **SCOPENET** from the pause menu. Other quick-action bindings start unbound so players can choose keys that fit their modpack. The launcher provides bindings for vaults, map, guild, market, buy orders, contracts, quests, travel, friends and the HUD manager.

Every domain uses the same vanilla-style dark panels with the launcher accent, Minecraft font, standard narrated widgets, searchable rows with scrolling, player heads and confirmation prompts. Labels are capitalised consistently ("Ender Chest", "Pending Incoming"). The casino is not part of the in-game companion: it lives in the launcher and the web panel, where its animations belong. The server shop screen was removed from the companion (the `/shop` and `/sell` chat commands still work). Server-disabled economy, guild and social features disable their navigation entries. Screens do not pause multiplayer.

- **Overview and HUD:** personal/global level, server level, wallet, guild, territory, daily quest totals, tracked quest and local clock. Modules look like vanilla tooltips with an item icon, and the tracked quest shows its actual task with a progress bar. They can be hidden, dragged and resized individually in game (scroll over a module, drag its corner handle, or use the Smaller/Larger buttons; the size is saved per module, from 50% to 250%, and can also be set in the launcher) with the **HUD Manager** (pause menu entry, Overview, or its key binding; changes save immediately) as well as in the launcher; both edit the same file. Placement uses normalized anchors so it adapts to the game's resolution and GUI scale. F1 hides the overlay.
- **Quest journal:** assigned daily/weekly goals with their full task text, progress bars, XP rewards, claim actions and pinning; achievement browsing uses the same journal. Rewards are granted by the existing panel quest handler. Claimed quests refresh automatically; the open journal and pinned HUD goal refresh every 30 seconds.
- **Guild hall:** guild description, MOTD, members, posts, directory/join requests, claim and invite actions, treasury balance/history and confirmed transfers. Officers can review join requests; existing guild role and Paper command permissions govern decisions.
- **Marketplace:** listings with the item icon, stack size, price per item, the seller's head and guild, auction bids and time left; sort (newest, price, ending soon) and filter (buy now, auctions); your own listings with cancellation, held-item listing, mailbox collection and transaction history. Buying/bidding/delivery continue through Paper's existing operation ledger and commands.
- **Orders:** the buy-order board (request items at a price, pick one up, hand the items in, drop or cancel), your orders, history and the board's rules. Handing items in and requesting use the same `/orders` commands as chat, so Paper checks the held items.
- **Contracts:** your randomly generated jobs with progress bars, rewards, bonuses and time left. Gathering contracts have a Submit button that opens the server's hand-in; kill contracts count automatically. Drop a contract to re-roll it.
- **World Map:** the same Live Map as the panel and launcher, ported from the shared engine: the same tile pyramid and zoom levels (with blurry-parent and finer-child fallbacks while tiles load), chunk grid and block outline when zoomed in, claim fills with holes and name chips, pins with item icons, and players with their real skins and facing, plus layer toggles, dimension switching, "Find Me", double-click zoom and a details card on click. Overlays refresh every eight seconds while open.
- **Travel:** spawn/back/random teleport, your homes and warps with their world and distance, home creation and teleport/trade requests.
- **Vaults and kits:** authorized vault selection, ender chest, kit claims and perk shortcuts. Selecting inventory-based actions opens the server container.
- **Friends and inbox:** friends with their skins, online status and server, TPA, messages through the existing `/msg` command, friend requests (including accepting incoming ones), personal notifications and marking them read. Player heads come from the panel's avatar endpoint by UUID or name, exactly like the launcher.
- **Dialogue:** message-only Content Studio NPCs open a native dialogue screen; their chat remains available as fallback. NPCs configured to run commands retain their existing server behavior. (The searchable content collection was removed.)
- **Branding:** launcher brand/accent in native panels and title-screen badge, SCOPENET pause-menu entry and the existing launcher icon in mod metadata.

Vaults, trades, crafting and Content Studio chests retain Minecraft container screens and server slot synchronization. This companion adds native navigation and presentation around them. It does not introduce Fabric-only registry entries that Paper could not recognize. Custom models still require correctly authored resource-pack assets; the client does not generate models or replace the server's content definitions.

## Launcher preferences

On launching a Fabric 26.3 instance, the launcher writes `config/scopenet-companion.json` atomically after pack synchronization. This file contains presentation preferences only: `enabled`, `notifications`, `claimBorders`, `scale`, `opacity`, `widgets`, `brand`, `accent`, and `reduceMotion`. It contains no account token, password, panel credential or server token. The client reloads it every two seconds and clamps scale, opacity and module coordinates.

The launcher customizer supports pointer dragging, keyboard arrows, individual module toggles and reset, and so does the in-game HUD Manager. The game only rewrites the keys it manages (`widgets`, `scale`, `opacity`, `notifications`, `claimBorders`, `reduceMotion`); everything else in the file is preserved. Bindings use the existing `options.txt` synchronization; Minecraft's own Controls can also edit them when launcher-managed bindings are disabled.

## Protocol and authority

The existing `scopenet:c2s` / `scopenet:s2c` VarInt UTF-8 framing is retained. A protocol-2 hello negotiates companion capability; old HUD clients continue to receive protocol-1 state messages. The updated hello also supplies the server's configured public panel origin for terrain requests. Map requests use the panel-issued map token, never a game-server credential.

The player sends an operation name, a request UUID and arguments. Paper supplies the connected player's UUID independently, checks feature/command gates, restricts the operation allowlist, allows one panel request in flight per player and enforces a 400ms request interval. Completion is scoped to that connection's session, so disconnecting cannot deliver an old callback to a new session.

The panel's server-token extractor authenticates the bridge. It resolves an active account and binds casino/map/economy scope to the authenticated game server; the client cannot choose another server or administrative endpoint. Local travel and storage reads stay on Paper's main thread. Market responses omit opaque delivery serialization.

Mutation receipts are keyed by server, player and request UUID. Repeating a completed mutation returns its saved result; reusing the UUID for different arguments is rejected. A process crash between execution and recording leaves the request reserved and prevents it running again. Errors/timeouts direct the player to inspect the result before repeating an action; financial operations are never automatically retried.

Responses split into at most 125 fragments of at most 4,000 UTF-16 code units, without splitting surrogate pairs. They cap total response size and remain below Paper's messaging limit even when JSON contains escaped characters. The client limits reassembly and times out requests after 12 seconds. Terrain decoding accepts bounded 256px PNGs and player heads bounded 8–128px PNGs, limits queued requests and cached textures, and releases screen-owned resources on close.

## Build and validation

With Java 25 and the repository's Gradle 9.6 wrapper:

```powershell
cd integrations
.\gradlew.bat -Ploader=fabric-client :fabric-client:build :common:test :paper:build
```

The modern JAR is in `integrations/fabric-client/build/libs/`. For the unchanged legacy client, use `-PmcVersion=1.20.1` with its Gradle/toolchain requirements.

Verified locally:

- Fabric 26.3 client and Paper builds, shared integration tests including identity substitution rejection and Unicode/payload-size round trips.
- Panel companion authentication, active-account/operation restrictions, notification isolation, duplicate financial receipts, launcher marketplace delivery tests.
- Launcher Rust compilation, Svelte checks and runes checks; browser verification of module dragging.
- Minecraft 26.3 launch with release Fabric API and native overview/quest/Mines rendering using disposable screen fixtures. The fixture code is excluded from the shipped sources. A separate native graphics startup attempt exited with `0xC0000005`; the subsequent bounded-heap/worker fixture run succeeded.

Screenshots below use fixture data, not a connected production account. Icons depend on the connection's bound item-component registry in 26.3 and are absent in these disconnected fixtures.

[Historical screenshot retained in the private source archive.]

[Historical screenshot retained in the private source archive.]

[Historical screenshot retained in the private source archive.]

[Historical screenshot retained in the private source archive.]

Before deploying to players, validate the complete connected path on a staging Paper 26.3 server: handshake/reconnect; inventory slot transfers and trades; buy/bid/cancel/mailbox delivery; quest reward receipt; active/finalized Mines rounds; resource-pack item/block/NPC models; terrain tiles and optional claim outlines; disabled feature/permission states; and the intended modpack's GUI scales. These multiplayer/resource-pack paths were not exercised against a live server in this local pass. Slots/wheel/Plinko currently show authoritative results in the native screen rather than bespoke animated game boards; community feed/DM history and administrative tooling remain in the launcher/panel.

Version references: [Fabric's 26.3 migration notes](https://fabricmc.net/2026/09/15/263.html), [Fabric API releases](https://github.com/FabricMC/fabric-api/releases).
