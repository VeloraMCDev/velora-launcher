# Admin guide

See [kits, custom item rewards, map claims and server assets](custom-content.md) for the in-game and panel workflows.

## Finding your way around

The sidebar groups the admin panel by what you are doing, and remembers which groups you folded away:

| Group | Pages |
|---|---|
| **Overview** | Dashboard, Activity |
| **Launcher & site** | Instances, Launcher design, Landing page, Capes |
| **Community** | Players, Guilds & claims, Discord, Emails |
| **Game** | Servers, Quests, Achievements, Leveling & XP, Progression, Content Studio, Commands & kits, Admin claims, Chat format |
| **Economy** | Economy, Casino |
| **System** | Scheduled tasks, Settings |

Press **Ctrl+K** (or **Cmd+K**) anywhere to open the quick search: it jumps to any page and also finds in-game commands. A breadcrumb bar at the top shows where you are, with search, the notification bell and a shortcut to the [player panel](player-panel.md). On a phone the sidebar becomes a drawer behind the menu button.

## Deploying the panel

```bash
cp .env.example .env         # set ADMIN_PASSWORD
docker compose up -d --build
docker compose logs -f panel # first start prints the admin account
```

Everything the panel stores lives in the `panel-data` volume (`/data`):

| Path | What |
|---|---|
| `panel.db` | SQLite database (users, instances, settings, stats) |
| `files/<instance>/` | Configs and uploads hosted for launchers |
| `uploads/` | Logos, backgrounds, icons |
| `jwt.secret` | Token signing key (unless `JWT_SECRET` is set) |

**Backups:** stop the container (or use `sqlite3 panel.db ".backup backup.db"`) and copy the volume.

**Updating this checkout:** `docker compose up -d --build`. Database migrations run automatically.

### HTTPS

Launchers talk to the panel over the internet, so serve it over HTTPS. Example with Caddy:

```caddyfile
panel.example.com {
    reverse_proxy localhost:8080
}
```

Large modpack uploads go through the proxy — raise its body-size limit if needed (nginx: `client_max_body_size 2g;`).

## Instances

An instance = one playable profile: a Minecraft version, an optional mod loader, files (mods, configs…) and an optional server.

- **Version** — choose any Minecraft version and Vanilla / Fabric / Quilt / Forge / NeoForge. Leave the loader version empty for the latest recommended build (it's pinned when you save).
- **Modrinth** — search, pick a version, *Use*. Mods are downloaded by players straight from Modrinth's CDN; configs (`overrides/`) are hosted by the panel.
- **CurseForge** — needs a free API key from [console.curseforge.com](https://console.curseforge.com) (Settings → Integrations, or `CURSEFORGE_API_KEY`). Some authors block third-party downloads; those files show up under **Files** with a link — download them yourself and upload them into the same folder.
- **Upload .zip** — a `.mrpack`, a CurseForge export, or any zipped instance folder (with `mods/`, `config/`, … or a `.minecraft/` inside). For plain zips, pick the Minecraft version and loader.
- **Files** — add or remove individual files at any time. Files players add themselves are never touched; files you remove are deleted from players' instances on their next launch.

Every save bumps the instance **revision**; launchers re-verify files when it changes, otherwise launching is instant.

### Built-in server

On the **Server** tab: name, address, port.

- *Add to multiplayer list* writes the server into `servers.dat` (keeping servers players added).
- *Join automatically* connects on start (Quick Play on 1.20+, `--server` on older versions).

Official servers use `online-mode=true`, authlib-injector and the SCOPENET server integration. See [server setup](server-integration.md) for the startup flag, tokens and supported versions.

### Access

- **Everyone** — any launcher, including local offline accounts.
- **Signed-in players** — any panel account.
- **Specific groups** — members of the selected groups (create groups on the Players page). Admins see everything.

## Players

- **Sign-ups:** Settings → *Closed* (admins create accounts), *Needs approval*, or *Open*.
- Each account has a permanent UUID. New accounts receive a random UUID; existing accounts keep theirs. Username changes preserve inventory and reserve prior names. Players change username, skin and permitted capes from the launcher or the [player panel](player-panel.md).
- Disabling an account signs it out everywhere on the next request.
- Ten failed logins lock an account for five minutes.
- **Username blacklist:** Settings → Username blacklist. Each entry matches a complete name; wrap it as `*word*` to block it inside longer names. The short starter list covers obvious offensive terms and can be edited. New registrations, admin-created accounts, Discord-created accounts, and username changes all use it.

### Discord and password reset email

Set **Settings → Auth server → Public address** to the HTTPS URL players use. In **Settings → Discord**, save the application client ID and client secret, and add `https://your-panel.example/api/v1/auth/discord/callback` to the Discord application's OAuth2 redirect URLs. Players can sign in with Discord in the launcher or connect an existing account under **Accounts → Connections → Discord**. With registration closed, Discord sign-in works for linked accounts only; open or approval mode can create new player accounts.

In **Settings → Resend SMTP**, save a Resend API key and a sender email from a verified domain. The panel sends password reset links through `smtp.resend.com` over TLS. A reset link expires after 30 minutes and works once. Use **Send a test email** and check the destination inbox and Resend activity log: SMTP acceptance confirms submission, while the inbox confirms delivery. Credentials are stored in the panel data volume and are not returned to the browser after saving; protect the volume and restrict access to the Admin Panel.

## Economy

**Admin panel → Economy** is where you watch and steer the money. Pick a server at the top (servers in one economy group share balances, listings and the ledger) and use the tabs:

* **Overview** — money in circulation, number of accounts, average and median balance, the richest player, money held in guild banks, trading volume today / 7 days / 30 days with a 14-day chart, the top earners and spenders of the week, the market at a glance (buy-now listings, auctions, total value, auctions ending in the next 24 hours, sales volume) and the casino's wagered / paid / house result when the casino is on.
* **Players** — search and sort every balance. **Adjust** adds, removes or sets a balance with a reason. It is one atomic step: the new balance and a ledger entry (`Admin: <reason>`) are saved together, you cannot remove more than someone has, and the player gets a notification.
* **Guilds** — every guild's bank balance with the same adjust action. The change also appears in the guild bank's own history.
* **Market** — every buy-now listing and auction. **Remove** takes a listing down: the item goes to the seller's vault, and if an auction had a leading bid the bidder is refunded and told. You can also **extend** an auction (1 to 168 hours) or **change the price** of a buy-now listing that has no bids.
* **Ledger** — the latest transactions with a search box (names or description).

Every admin action is recorded on the **Activity** page like the rest of the admin panel.

## Scheduled tasks

**Admin panel → Scheduled tasks** lists everything the panel does by itself: purging players whose account was deleted, refreshing rank titles, pruning old notifications, settling auctions, applying earned rewards, Discord announcements and role sync. Each shows when it last ran, how long it took, what it did and when it runs next. You can pause or retune a task, change its settings, and press **Run now**.

**Purge deleted players** runs daily. It finds players whose account no longer exists (the panel remembers every UUID that ever had one) and removes their leftover levels, stats, quests, economy and guild rows so they stop appearing in leaderboards. Turn on *Dry run* first to see how many it would clean without deleting anything. *Also clean players who never had an account* is for offline-mode servers and only touches players inactive for the number of days you set.

## Notifications

Everyone sees a bell in the launcher's title bar, the player panel's top bar and the admin panel's top bar. Guild renames and disbands, join requests and approvals, role changes, kicks, announcements and auction results land there, and players in game also see them in chat. Read ones are removed after 30 days.

## Chat format, commands and kits

* **Chat format** — design the in-game chat line with live preview: `{guild}{title}{group}{prefix}{name}{suffix}&7: &f{message}`. LuckPerms prefixes and suffixes (including `&#RRGGBB`) are read live from LuckPerms; empty parts disappear without stray brackets.
* **Commands & kits** — configure `/heal`, `/feed` (amount and cooldown), `/fly`, `/vault` (how many vaults, rows, which are free) and `/echest`, and create **kits** with a cooldown, one-time option, LuckPerms groups and any items — including custom items.
* **Custom items** — design items with a name, lore, enchantments up to level 255 and attribute bonuses, then hand them out with `/customitem give <player> <id>` or put them in kits.
* **Content Studio** — import ItemsAdder, Nexo, Oraxen, CraftEngine, MythicMobs and ModelEngine packs as items, blocks, chests, decorations, NPCs, vehicles, crops and mobs, with a setup wizard, 3D preview and an advanced editor. See [Content Studio](content-studio.md).
* **Casino** — Slots, Wheel, Plinko, Mines, Blackjack, Crash, Dice and Coin Flip, Double or Nothing after wins, a free daily spin, bounties on players' heads and bets on what players will do, all configurable, with the real return-to-player shown next to every control. Players use it in the launcher and in the [player panel](player-panel.md). See [Casino](casino.md). **Buy orders** and **Contracts** (Economy → Orders & contracts) let players request items with escrowed money and take random kill / resource tasks for pay; see [Buy orders and contracts](orders-and-contracts.md).
* **Admin claims** — protected server land (spawn, shops, arenas) with its own name, description and map colour. Create them in the panel or in game with `/adminclaim`. Players see the name and description above their hotbar when they walk in; staff with `scopenet.claims.bypass` can still build.
  **Flags** decide what is allowed inside: build, interact, containers, entry, PVP, player/animal damage, mob spawning, mob griefing, explosions, fire spread, fluid flow, flying, ender pearls, hunger and item drops. Set them in the panel (Admin Claims → Flags) or in game with `/adminclaim flag <name> [flag] [on|off]`. Flags are enforced by the Paper plugin; the Fabric server mod has no claim protection.

Permissions: `free.fly` (fly anywhere) and `guild.fly` (fly inside your own guild's land) must be granted; `scopenet.vault.2`, `.3`, … unlock vaults beyond the free ones; kits restricted to groups check `group.<name>`.

## Launcher design

Everything on this page is pushed to launchers when they start or refresh — no reinstall. The **Features** tab lets you allow or block players from changing the theme or Java settings. **Custom CSS** is injected last; handy variables are `--accent`, `--accent-2`, `--surface`, `--radius`.

## Phone apps

Upload the Android `.apk` and iOS `.ipa` under **Settings → Launcher & apps** and players can install them from the player panel; the iOS app also gets an AltStore source URL. See [Phone apps](mobile-apps.md).

## Releasing the launcher

See the README's *Build your branded launcher*. Players get updates automatically: the launcher checks GitHub Releases on start and offers the new installer.

**Uploading installers:** *Settings → Launcher & apps* (and *Landing page → Downloads*) has one row per platform: Windows, macOS, Linux, Android and iOS. Choose the file, check the version it detected from the file name, and press Publish. Linux can hold an AppImage and a .deb at once. Files appear on the website with short names such as `SCOPENET-Setup-1.0.1.exe`, `SCOPENET-1.0.1-macOS.dmg` and `SCOPENET-1.0.1.apk`, and download under those names whatever they were called when uploaded.

**Live map on the website:** new and existing landing pages get an *Explore the World* section showing the live map with guild land, spawn, warps and shops, never player positions (you can change which layers are public, or remove the section, in the landing builder). It hides itself while no server has a map.

**Website downloads:** the public landing page and the player panel list Windows, macOS and Linux. Anything you upload by hand under *Landing page → Downloads* wins; otherwise the panel reads the installers from the latest GitHub release of the repository set as the *external download URL* (cached for ten minutes), so every release shows up without uploading anything. A platform with no installer yet shows "Coming soon".

**macOS and Linux:** each release also carries `scopenet-launcher-<version>-macos-universal.dmg`, `…-linux-x64.AppImage` and `…-linux-x64.deb`. They are unsigned: on macOS right-click → Open the first time (or sign and notarize with an Apple Developer ID); on Linux `chmod +x` the AppImage. These builds check GitHub Releases for updates and open the new download; only the Windows build installs updates by itself.

**Code signing:** unsigned installers trigger a Windows SmartScreen warning ("More info → Run anyway"). To avoid it, sign the installer with a code-signing certificate (e.g. Azure Trusted Signing) — Tauri supports this via `bundle.windows.signCommand`.

## Progression (quests, XP, levels)

**Progression** in the sidebar controls how players advance:

* **Quest limits** – how many daily and weekly quests each player is given (0 = every
  enabled quest), and whether everyone draws their own set or shares one. Pin quests
  on the Quests page to always include them. A player's set is stored when they first
  open their quests for the period, so edits never swap out quests they've started.
* **XP & levels** – the level curve (`round(base × L^exponent)`, optional max level),
  XP per playtime hour / kill / block / message, and panel-wide multipliers that stack
  with each server's own. Saving a new curve recalculates stored levels; nobody loses XP.
* **Players** – search a player, then give, take, set or reset XP (global or per
  server) or set their level. Changes are recorded in the Activity log with your reason.

### Default money rewards

Every quest, achievement and level now pays money by default, scaled by difficulty and always a flat number ending in 5 or 0:

* **Quests and achievements** pay their XP (the difficulty you gave them) times a rate: daily 0.35, weekly 0.5, other quests 0.4, achievements 0.6 dollars per XP.
* **Reaching a level** pays `base × level^exponent` (default 20 × L^1.4), so a rank in the hundreds is worth chasing. **Rank milestones** (titles, badges) add 3× that level's pay on top.

Tune all of it in **Progression → Money rewards**, with a live preview of what each rate pays. Saving applies the rates to everything straight away, an hourly background job (Scheduled tasks → *Default money*) covers anything you add later, and **Apply to everything now** forces it. An amount you type on a quest, achievement or reward yourself is never overwritten; the automatic ones are marked `auto` and follow the rates. Turn **Pay default money** off to remove the automatic amounts again.

### Bulk editor

**Game → Bulk editor** shows every quest, achievement or level reward as one row. Tick rows (shift-click for a range), then **set, multiply or add** to their money, XP or level, round money to 5s, switch back to the automatic amount, or enable and disable quests. You can also type straight into any cell. Nothing is saved until you press **Save changes** (Ctrl+S).

## Icon library and rank badges

* **Icon library** — the quest and achievement editors have a **Browse icon library** button: about 139,000 open-source icons from 37 Iconify packs (Lucide, Phosphor, Tabler, Game Icons, Material, Font Awesome, Fluent emoji and more, the same sets WebLander uses). Pick a colour for single-colour icons; the launcher draws them from the panel. The sets are packed by `panel/icons/build.mjs` (the Docker image and release workflow do this). Outside Docker, run `cd panel/icons && npm ci && node build.mjs` and set `SCOPENET_ICONS_DIR`. Check each pack's licence, since CC BY sets need credit.
* **Rank badge designer** — on a title reward, **Design a badge** draws pixel-style PNG badges (gradient plate, chunky outlined lettering, optional icon) in the browser and attaches the upload. Presets: Member, Founder, Staff, VIP.

### PNG rank titles in game

Turn on the server resource pack (Custom items → Server assets → *Offer pack to players*, ideally *Require acceptance*). Every title reward with an **uploaded** PNG (use the badge designer or upload your own) gets its own private-use character; the pack maps that character to the image through `assets/minecraft/font/default.json`, merged with any font file you imported. In **Chat format**, put `{rank_title}` (or `%rank_title%`) in the layout: it draws the player's current rank as the badge, and falls back to the text `{title}` for ranks without a PNG. PlaceholderAPI users get the same through `%scopenet_rank_title%` (tab lists, scoreboards, other chat plugins). The badge is drawn 9 pixels tall; the designer makes its output a multiple of 9 so it stays crisp. Images linked from other websites can't be packed — upload them.


## Clean update (reset everyone's modpack files)

Open **Instances → your instance → Source** and press **Push clean update**. On their next launch every player's launcher will:

1. delete every file the panel ever put in that instance (mods, configs, resource packs),
2. move any mod the player added by hand into `.scopenet/removed/<n>/mods/` (so nothing is lost, but nothing can conflict with the new modpack),
3. empty the cached server resource pack (`server-resource-packs/`), and
4. download every file again.

Worlds, screenshots, options and everything else in the game folder are left alone. Each press cleans each player once; a brand-new install has nothing to clean. A normal update (publishing changes) still only adds, changes and removes the files that differ.

## Resource pack loading

Each pack revision is sent under one fixed pack id, so a new revision *replaces* the old pack instead of stacking on top of it (the cause of mixed or broken textures for players who stayed connected through an update or moved between servers behind a proxy). Paper 1.20.3+ and newer use this; older servers only ever hold one pack. If a client reports a failed download or apply, the plugin offers the pack again after a few seconds, up to twice per session. Declining the pack is left alone.


## Players without a bank account

*Economy → Players* shows a banner when players have no account on the selected economy. **Create all missing accounts** opens one for each at the starting balance; an unclaimed account with the same name (money left under an old or offline-mode UUID) is handed to the player instead, balance and history included. For one player you can pick which unclaimed account they should take over, or open a new one. The same fix runs daily as the scheduled job **Create missing bank accounts** (*Scheduled tasks*, where you can also run it now).
