# Velora Paper Plugin (Paper 26.3+)

Minecraft Paper server integration plugin for the Velora platform.

## Features

1. **Admin Panel Linking & Custom Authentication Acceptance**
   - Direct integration with Velora Admin Panel via HTTP API (`/api/server/v1/hello` & `/api/server/v1/login`).
   - Secure fail-closed pre-login verification (`AsyncPlayerPreLoginEvent` at `HIGHEST` priority).
   - Dynamic linking command (`/velora link <token>`) and status inspector (`/velora status`).
   - Works seamlessly with authlib-injector / custom Yggdrasil authentication setups (`online-mode=true`).

2. **Server Analytics and Tracking**
   - Real-time rolling TPS calculation.
   - Live session tracking: online player roster and playtime seconds calculation.
   - Comprehensive telemetry: block breaks/placements, player deaths, PvP kills, mob kills, and command executions.
   - Periodic 30-second asynchronous heartbeat and sync batching (`POST /api/server/v1/sync`).
   - Remote account session revocation enforcement (kicks banned/deactivated accounts immediately).

3. **Simple Chat Formatting**
   - Fully customizable chat template in `config.yml`.
   - Supports placeholders: `{player}`, `{displayname}`, `{prefix}`, `{suffix}`, `{group}`, `{world}`, `{message}`.
   - Soft-dependency integration with **LuckPerms** for prefixes, suffixes, and primary groups.
   - Color code translation: supports `&0-9a-fk-or` and RGB hex colors (`&#RRGGBB` and `<#RRGGBB>`).
   - Permission check: only players with `velora.chat.color` (or op) can use colored chat; unpermitted color codes are stripped.
   - Dual support for modern Paper Adventure `AsyncChatEvent` and legacy Bukkit `AsyncPlayerChatEvent`.

---

## Installation

1. Copy `velora-paper-0.7.0.jar` from `integrations/paper/build/libs/` into your server's `plugins/` directory.
2. Start the Paper server.
3. Link the server using either:
   - Run `/velora link <token>` in the server console or in-game (requires `velora.admin` permission).
   - Or set `panel-url` and `token` in `plugins/Velora/config.yml` and run `/velora reload`.

---

## Commands & Permissions

| Command | Permission | Description |
| :--- | :--- | :--- |
| `/velora status` | `velora.admin` | View server linking status, panel URL, TPS, and player count |
| `/velora link <token>` | `velora.admin` | Set panel token and perform handshake with panel |
| `/velora reload` | `velora.admin` | Reload `config.yml` and reconnect to panel |

| Permission | Default | Description |
| :--- | :--- | :--- |
| `velora.admin` | `op` | Allows access to `/velora` management commands |
| `velora.chat.color` | `op` | Allows using `&` color codes and hex colors in chat |

---

## Configuration (`config.yml`)

```yaml
panel-url: "http://localhost:3000"
token: ""

auth:
  enabled: true
  timeout-seconds: 6
  unavailable-message: "Velora sign-in is unavailable. Please try again shortly."
  unauthorized-message: "You are not authorized to join this server. Please use the Velora launcher."

analytics:
  enabled: true
  sync-interval-seconds: 30
  track-blocks: true
  track-combat: true
  track-commands: true

chat:
  enabled: true
  format: "{prefix}&f{player}{suffix}&7: &f{message}"
  allow-colors: true
```

---

## Building from Source

```bash
cd integrations
./gradlew :paper:build
```

The resulting JAR will be generated at:
`integrations/paper/build/libs/velora-paper-0.7.0.jar`
