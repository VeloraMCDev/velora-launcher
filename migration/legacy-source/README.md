<div align="center">

<img src="launcher/src-tauri/icons/128x128.png" width="96" alt="" />

# SCOPENET

**A Minecraft launcher, a self-hosted admin panel, and a player panel that fits in your pocket.**

Publish vanilla versions or modpacks, run a real in-game economy, and let players grind from the launcher, the website or their phone.

</div>

<p align="center">
  [Historical screenshot retained in the private source archive.]
  [Historical screenshot retained in the private source archive.]
</p>

SCOPENET owns the platform. Each instance owns the experience: its systems, identity, navigation and progression. See [the instance architecture and migration guide](docs/instance-experiences.md).

## One project, three faces

| | |
|---|---|
| **Launcher** (Windows) | Tauri 2 + Rust + Svelte. Installs Java, the game and your modpack, then plays. Uses the system WebView2, so the installer stays small. |
| **Admin panel** (Docker) | Rust (Axum) + SQLite + Svelte. Instances, players, economy, casino, quests, Discord, branding. Ships as a 14 MB image. |
| **Player panel** (website, Android, iOS) | The launcher's everyday features on any device: market, casino, friends, guilds, quests, leaderboards and the live map. |
| **Engine** | `scopenet-core` installs Minecraft, Java and Fabric / Quilt / Forge / NeoForge and launches the game. |

## The player panel: your on-the-go grind

Open `https://your-panel/#/play`, sign in with your launcher account and everything is there, phone first. Install it to your home screen, or grab the **Android app** from the release page.

<p align="center">
  [Historical screenshot retained in the private source archive.]
  [Historical screenshot retained in the private source archive.]
  [Historical screenshot retained in the private source archive.]
  [Historical screenshot retained in the private source archive.]
  [Historical screenshot retained in the private source archive.]
</p>

- **Market**: buy, bid and watch auctions with live countdowns; purchases land in your in-game vault.
- **Casino**: slots, wheel, plinko, mines, a free daily spin, bounties and bets, with real animations and optional sound.
- **Friends**: friend requests, a messenger-style DM view, game invites, profiles and posts.
- **Guilds, Wallet, Quests, Leaderboards, Live map, Launcher downloads, Commands, Account**, all one tap away; **Ctrl+K** finds anything.

<p align="center">[Historical screenshot retained in the private source archive.]</p>

Details: [docs/player-panel.md](docs/player-panel.md) · phone apps: [docs/mobile-apps.md](docs/mobile-apps.md)

## Casino

One casino, shared by the launcher and the website. The reels spin up and settle with a bounce, the wheel ticks past each peg, Plinko balls bounce peg by peg, Mines tiles flip, and big wins throw coins. The server rolls every result; the animation only reveals it. Odds, limits and the return-to-player are all set from the admin panel.

<p align="center">[Historical screenshot retained in the private source archive.]</p>

## Admin panel

A grouped sidebar (Overview, Launcher & site, Community, Game, Economy, System), a **Ctrl+K** quick search, breadcrumbs and a drawer menu on phones keep it tidy.

<p align="center">
  [Historical screenshot retained in the private source archive.]
  [Historical screenshot retained in the private source archive.]
</p>

- **Economy console**: money in circulation, volume charts, top earners and spenders; add, remove or set any player's or guild's balance with a reason and a ledger entry; remove, extend or re-price market and auction listings (bidders are refunded); search the ledger.
- **Instances**: pick a version and loader, or import a Modrinth / CurseForge / `.zip` modpack. Access is public, signed-in or per group.
- **Players**: approve and manage accounts, roles and groups; sign-ups closed, approval or open.
- **Launcher design**: name, logo, colours, font, background and news with a live preview, pushed to every launcher instantly.
- **Quests, achievements, levels, guilds, Discord, email, landing page, scheduled tasks** and more.

## The launcher

<p align="center">
  [Historical screenshot retained in the private source archive.]
  [Historical screenshot retained in the private source archive.]
</p>

- **Accounts**: private panel accounts (authenticated online-mode servers) and local offline usernames. Tokens are encrypted with a key kept in Windows Credential Manager.
- **One-click play**: downloads the right Java, the game, the mod loader and the modpack with live speed and ETA, verifies files by SHA-1, auto-joins the instance's server.
- **Market, casino, friends, guilds, quests, stats, map**: the same features as the player panel, in the app.
- **Ctrl+K palette and Command guide**: jump anywhere, or find and copy any in-game command.
- **Settings**: memory, resolution, GC presets, JVM args, keybind editor, accent colour, per-instance overrides, storage manager; repair files, crash hints, offline launch, self-updates.
- **Branding from the panel**: no rebuild needed.

## Quick start

### 1. Run the panel

```bash
git clone https://github.com/scopeddlol/SCOPENET-MC.git
cd SCOPENET-MC
cp .env.example .env
# edit .env → set ADMIN_PASSWORD and PUBLIC_URL for your domain
docker compose config --quiet
docker compose up -d --build
```

Open `http://your-server:8080`, sign in as `admin`, then:

1. **Launcher design** → name, logo, colours, news.
2. **Instances** → *New instance* → a version, or a Modrinth / CurseForge / .zip modpack.
3. **Settings** → how players sign in. Share `/#/play` with your players.

> Put the panel behind HTTPS (Caddy, Traefik, nginx, Cloudflare Tunnel…) before sharing it; phones need it to install the app. See [docs/deployment.md](docs/deployment.md).

Want to look around first? `scripts/demo/run.sh` starts a panel filled with realistic demo data ([how](scripts/demo/README.md)).

### 2. Connect a game server

Install the Paper plugin (one JAR for every version from 1.20) or the Fabric / Forge mod, paste the server token from **Servers**, restart. In game, `/scopenet help` lists everything and `/scopenet panel` prints the player panel link. Full guide: [docs/server-integration.md](docs/server-integration.md).

### 2. Build and publish a release

1. In GitHub → **Settings → Secrets and variables → Actions → Variables**, add (all optional):
   - `PANEL_URL` = `https://panel.example.com` (players connect automatically)
   - `LOCK_PANEL` = `1` (hide the "change server" option)
   - `LAUNCHER_NAME` = `MyServer Launcher`
2. **Actions → Release → Run workflow**, enter a **new tag** such as `v1.0.0`. Nothing in this repository runs on push or pull request; the workflow only starts when you press the button.

One run builds and publishes everything:

| Output | Where it goes |
|---|---|
| Windows installer (NSIS) | **Draft** GitHub release |
| Fabric client for 26.3; Fabric and Forge server JARs for 1.20.1, 1.21.1 and 26.3; **one** Paper plugin JAR for every version | The same draft release |
| Player app for phones: Android `.apk` and unsigned iOS `.ipa` ([details](docs/mobile-apps.md)) | The same draft release |
| Panel Docker image (linux/amd64) | `ghcr.io/<owner>/scopenet-mc-panel:latest` |

Review the draft under **Releases** and press **Publish** — installed launchers pick up published releases automatically. Every download file (the installer, the JARs and the phone apps) includes the input release version in its filename. Fabric/Forge metadata and Paper descriptors/manifests are checked against that version, and all Minecraft targets are checked before publication. The workflow stops before publishing anything if the tag is malformed or already used, if any build fails (the two phone-app builds are optional until you have seen them succeed once; see [docs/mobile-apps.md](docs/mobile-apps.md)), or if an artifact is missing or mis-stamped. The Paper plugin uses the stable Bukkit API (no NMS), so a single JAR covers 1.20 and newer.

How it keeps the minute count low: the tag is validated in seconds before any big runner starts; the only Windows job is the installer (Windows minutes cost double); Rust, Gradle and npm are cached between runs; the panel is compiled natively with a warm Cargo cache and Docker only packages the binary; a failing Minecraft version cancels the others; and the image and release are published only after everything has built, so a failure never leaves a half-finished release. Runs are queued one at a time.

> New GHCR packages start private. Make the package public under your profile's **Packages** settings if you want `docker pull` without logging in. Run the workflow from the default branch: GitHub only lists **Run workflow** for workflows that exist there.

## Repository layout

```
crates/shared      Wire types shared by launcher and panel (manifest, branding, auth)
shared/            TypeScript shared by the launcher and the website: casino, map, command guide
crates/core        Launcher engine: Mojang/Java/loaders, file sync, launch, authlib, ping
integrations/      Paper plugin and Fabric/Forge server mods
panel/server       Admin panel API (Axum + SQLite)
panel/web          Admin panel and mobile-first player panel (Svelte 5)
mobile/            Android / iOS app shell around the player panel (Capacitor)
scripts/demo/      One-command demo panel with realistic data (used for the screenshots)
launcher/          Desktop launcher UI (Svelte 5)
launcher/src-tauri Desktop launcher shell (Tauri 2)
Dockerfile         Panel image (static binary on scratch)
docker-compose.yml Panel deployment
.github/workflows Release: installer, server JARs, panel image and a draft release (manual)
```

## Documentation

- [Player panel](docs/player-panel.md) — the mobile-first website: market, casino, friends, guilds, quests
- [Phone apps](docs/mobile-apps.md) — the Android `.apk` and iOS `.ipa`, signing and setup
- [Admin guide](docs/admin-guide.md) — navigation, the Economy console, deploying, HTTPS, modpacks, players, backups
- [Casino](docs/casino.md) — games, odds, safety limits, animations
- [Command guide](docs/commands.md) — every in-game command and permission node
- [Official server setup](docs/server-integration.md) — private authentication, supported versions, activity and UUID protection
- [The SCOPENET Map](docs/map.md) — the built-in world map: terrain, guild land, shops and players
- [Discord, live embeds and email](docs/discord-and-email.md) — customisable announcements, self-updating status boards and emails to players
- [Rewards, limits and permission nodes](docs/rewards-and-limits.md) — what quests can grant, and every setting you can change
- [Architecture](docs/architecture.md) — how the pieces fit, API reference
- [Development](docs/development.md) — running everything locally, tests
- [Deployment](docs/deployment.md) — local Compose build, health check, upgrades and backups
- [Local 0.4.0 artifacts](release-artifacts/0.4.0/README.md) — Windows installer, server jars and checksums

## Configuration reference

**Panel (environment variables)**

| Variable | Default | |
|---|---|---|
| `ADMIN_USERNAME` | `admin` | First admin account (created on first start) |
| `ADMIN_PASSWORD` | random, printed in logs | First admin password |
| `JWT_SECRET` | auto-generated in `/data` | Signs login tokens |
| `CURSEFORGE_API_KEY` | – | Also settable in the panel |
| `MAX_UPLOAD_MB` | `2048` | Largest modpack upload |
| `SCOPENET_BIND` | `0.0.0.0:8080` | Listen address |
| `SCOPENET_DATA_DIR` | `/data` | Database, hosted files, uploads |

**Launcher (build-time, via repository variables)** — `PANEL_URL`, `LOCK_PANEL`, `LAUNCHER_NAME`.

## Notes

- Minecraft is a trademark of Mojang Studios. SCOPENET is not affiliated with Mojang or Microsoft. Offline mode is intended for your own community servers.
- No license has been chosen yet — add a `LICENSE` file before distributing.
