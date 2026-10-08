# Architecture

SCOPENET owns the platform. Each instance owns the experience. See [Instance experiences](instance-experiences.md) for ownership boundaries, request context, migration and module extension contracts.

```
┌──────────────────────────┐        HTTPS (JSON)        ┌──────────────────────────┐
│  Launcher (Tauri)        │ ─────────────────────────▶ │  Panel (Axum + SQLite)   │
│  Svelte UI ⇄ Rust shell  │   manifest, instances,     │  Svelte admin UI         │
│        │                 │   auth, stats              │  /files, /uploads        │
│  scopenet-core engine    │                            └──────────────────────────┘
│   ├─ Mojang / Java       │ ──▶ piston-meta, libraries.minecraft.net, resources
│   ├─ Fabric/Quilt/Forge  │ ──▶ meta.fabricmc.net, maven.minecraftforge.net, …
│   └─ modpack files       │ ──▶ Modrinth/CurseForge CDNs, panel /files
└──────────────────────────┘
```

- **`crates/shared`** defines every JSON type exchanged, so the panel and launcher can't drift apart.
- **`shared/`** (TypeScript) holds UI code used by more than one front end: `shared/casino` (every casino game, with an adapter so the launcher calls it through Tauri and the website through `fetch`), `shared/map` (the live map viewer) and `shared/commands` (the command guide that feeds the launcher, the player panel, both Ctrl+K palettes and `docs/commands.md`).
- **`panel/web`** is one Svelte app with two faces: the **admin panel** (`#/instances`, `#/instance/<id>/<page>`) and the mobile-first **[player panel](player-panel.md)** (`#/play/...`, lazily loaded). Both use the same sign-in.
- **`crates/core`** has no UI dependencies. It's unit-tested, and CI runs real installs of vanilla, legacy, Fabric, Quilt, Forge and NeoForge against the live servers.
- The **panel** also uses `core` for version and loader lookups.

## Launch pipeline

1. Resolve the account and validate its private Yggdrasil session.
2. Fetch the instance from the panel (cached for offline play).
3. `install`: vanilla version JSON → Java runtime (Mojang's own builds) → client jar, libraries, assets → loader profile (Fabric/Quilt), or replay the Forge/NeoForge installer's processors → merge → natives.
4. `sync`: download the admin's files; remove files the admin removed; never touch the player's own files. With an unchanged revision, only existence is checked.
5. Write `servers.dat` and keybinds, build the Java command (quick play / `--server`, memory, GC presets, `@argfile` for long classpaths), and start the game.
6. Stream logs to the console; on a crash, show the tail of the log with hints.

## Disk layout (launcher)

```
%APPDATA%\net.scopenet.launcher\
  settings.json, accounts.json, secrets.bin (encrypted), manifest.json (cache)
  minecraft\
    versions\  libraries\  assets\  runtimes\  natives\  cache\
    instances\<id>\        ← game directory (saves, mods, options.txt, …)
```

## HTTP API

Public (used by launchers). Send `Authorization: Bearer <token>` to see restricted instances:

| Method | Path | |
|---|---|---|
| GET | `/api/v1/launcher/manifest` | Branding, auth options, visible instances |
| GET | `/api/v1/launcher/instances/{id}` | Instance + file list |
| POST | `/api/v1/launcher/events` | Launch stats |
| POST | `/api/v1/auth/login` · `/register` | Panel accounts |
| GET | `/api/v1/auth/me` | Current user |
| GET | `/api/v1/economy/me` · `/economy/balance/{server}` · `/economy/transactions` | The player's balances (all servers, or one) and recent transactions |
| GET/POST | `/api/v1/market/{server}` · `/mine` · `/buy` · `/bid` · `/cancel` | Browse and trade listings and auctions |
| GET/POST | `/api/v1/casino/{server}/...` | Lobby, history, slots, wheel, plinko, daily, mines, bounties, bets |
| GET | `/api/v1/servers/public` · `/leaderboard` · `/levels/me` · `/quests/my` ... | Everything the player panel and launcher show |
| GET | `/files/{instance}/{path}` | Hosted instance files |
| GET | `/healthz` | Health check |

Admin (`role = admin`): `/api/admin/{stats,users,groups,branding,settings,instances,uploads}`, the economy console under `/api/admin/economy/{overview,players,players/adjust,guilds,guilds/adjust,market,market/{id}/remove,market/{id}/extend,market/{id}/price,transactions}` plus `instances/{id}/{import/modrinth,import/curseforge,import/upload,files,reset}` and meta/search proxies under `/api/admin/{meta,modrinth,curseforge}`.

## Security notes

- Passwords are hashed with Argon2id; tokens are HS256 JWTs (30 days); disabled accounts are rejected on every request.
- Server-provided paths are sanitised (`safe_join`) on both sides — no `..` or absolute paths.
- The panel never returns the CurseForge key; SVG uploads are refused (script risk).
- The launcher's CSP only allows scripts from the app itself; media may load from any HTTPS host (admin-chosen images).
- Hosted `/files` are public URLs. Don't put secrets in instance files.
