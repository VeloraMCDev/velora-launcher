# Fabric: server mod features and the client mod

Two jars, both for **Fabric 1.20.1**:

| Jar | Install on | Needs |
|---|---|---|
| `scopenet-fabric-1.20.1.jar` (server mod) | the server (and a single-player world if you want the commands there) | Fabric API. Optional: LuckPerms, Placeholder API |
| `scopenet-client-fabric-1.20.1.jar` (client mod) | players who want the HUD, claim borders and shop/market windows | Fabric API |

The client mod is optional and display-only. The server stays in charge of claims, money and permissions; every button in the client is an ordinary command. A player without it loses nothing. You can install both jars on the same machine (for example a single-player world), because the client mod shares no code with the server mod.

Other Minecraft versions keep the original behaviour (sign-in, stats, claims, Live Map) and do not get the commands yet.

## Build

```sh
gradle -p integrations -Ploader=fabric -PmcVersion=1.20.1 :fabric:build        # server mod
gradle -p integrations -Ploader=fabric-client :fabric-client:build              # client mod
```

The core logic (`integrations/common`, package `net.scopenet.core`) is platform-neutral and unit-tested with `gradle -p integrations :common:test`. The Fabric classes are thin adapters over it.

## What the server mod adds (same behaviour as the Paper plugin)

* **Essentials:** `/spawn /home /sethome /delhome /back /tpa /tpaccept /tpdeny /rtp /warp /playtime` (homes and warps saved in `config/scopenet/essentials.json`). `/back` remembers where you were before SCOPENET teleports and where you died.
* **Economy:** `/balance /pay /baltop /shop /sell hand /market (sell|buy) /orders /transactions`. Selling and buying go through an escrow file (`economy-pending.json`) so a crash or a panel outage never loses an item or a payment.
* **Guilds:** `/guild` (info, create, leave, claim, unclaim, map, chat, sethome, home, members, bank, pay, sell, market), `/claim`, `/unclaim`. Claim checks use the same local index as the rest of the mod.
* **Info and admin:** `/scopenet help | panel | status | reload` (alias `/sn`). `/scopenet panel` prints the link to the web [player panel](player-panel.md); `status` and `reload` are admin only.
* **Permissions:** the same `scopenet.command.*` nodes as Paper. With the LuckPerms mod installed its answer wins; otherwise `permissions.default_level` decides, and admin nodes need operator level 2. `scopenet.claims.bypass` lets staff build in other guilds' claims.
* **Placeholder API:** `%scopenet:level%`, `%scopenet:guild%`, `%scopenet:balance%`, `%scopenet:playtime%`, `%scopenet:quest_progress%` and the rest of the Paper list (see `Placeholders.KEYS`).
* **Economy for other mods:** `net.scopenet.fabric.api.ScopenetEconomy` (Fabric's stand-in for Vault): `balance`, `deposit`, `withdraw`, all asynchronous and safe to retry.

### Claim rules (Fabric and Forge)

The shared Minecraft mod code (used by both Fabric and Forge) enforces the same guild and admin-claim rules as the Paper plugin for **building, using doors and buttons, opening chests, buckets, PVP and player damage, mob spawning, explosions, fire spreading and fluids flowing**. Visitors only get what the claim's rules allow; members and staff with `scopenet.claims.bypass` are never held back.

The core guild checks (build, interact, containers) are required hooks. The rest live in a separate optional mixin config, `scopenet.claimrules.mixins.json`, so if a Minecraft version differs from what a hook expects, that one rule is skipped with a warning instead of stopping the server. Check the log for `scopenet.claimrules` warnings after a Minecraft update.

Paper-only for now: keeping visitors out (`entry`), flying, hunger, ender pearls, dropping items, hurting animals and mob griefing.

Not on Fabric yet: `/trade` (needs a chest window), item sorting of homes/warps into a menu (they are listed in chat), and teleports by other mods are not remembered for `/back`.

### Economy mods

SCOPENET keeps the ledger in the panel, so its money is one number per player across your network. Other Fabric mods can use it through `ScopenetEconomy`. Adapters that register SCOPENET as a provider inside third-party economy mods (Impactor, Common Economy API and similar) are **not included**: their APIs change between versions and I could not build against them, so shipping guesses would risk silent money bugs. They are small to add once the exact versions are chosen; the extension point is `ScopenetEconomy`.

## The client mod

* **HUD widgets:** Level (with XP bar and server level), Balance, Guild, Claim (who owns the chunk you are in) and Quests. Each can be on, compact or off.
* **Claim borders:** green boxes around your guild's land and red around other guilds', within 4 chunks.
* **Windows:** press **K** for the menu, then **Shop** (every price, your held item and what it is worth, one-click sell), **Market** (browse, one-click buy, list your held item) or **Settings**.
* **HUD layout editor:** Settings → *Edit HUD layout*. Drag widgets, right-click to hide or show, scroll to resize. Positions snap to screen anchors, so they hold on any window size.
* **Toasts:** level-ups, achievements and guild changes, each switchable.
* **Keys:** *Open SCOPENET menu* (K) and *Show or hide SCOPENET HUD* (unbound) in Controls → SCOPENET.

Settings are saved to `config/scopenet-client.json` (see `examples/fabric-client/`).

### How it talks to the server

Two plugin-message channels, `scopenet:s2c` and `scopenet:c2s`, carrying JSON in a Minecraft UTF string (`net.scopenet.core.Wire`). The client says hello; the server then sends its state, the claims around the player, and answers market and shop requests. The same channels work on the Paper plugin (`clientlink.enabled` in `config.yml`), so the client mod works on both. Turn the link off with `clientlink.enabled=false`.

## Status

The shared core, the escrow flow, the client-link protocol and the placeholders are covered by unit tests that run in CI (`CommandsTest`, `LinkTest`, `EssentialsServiceTest`). **The Fabric server mod, the client mod and the Paper additions have not been compiled or run**, because the build environment they were written in cannot download Minecraft, Fabric or Paper. Build them, then report any compile errors.
