# Official servers and private authentication

Use **online-mode=true**. The server's authlib-injector agent redirects Minecraft's session verification to your panel. The plugin/mod then checks account status, groups and optional launcher access before admitting the player. Offline mode cannot verify identity and the integration refuses it.

## Install

1. Deploy the updated panel and set **Settings → Public address** (or `PUBLIC_URL`) to its HTTPS URL.
2. Create a server under **Servers**, choose **Members** or **Groups** for a private community, and copy its `sn_…` token. Give each server its own token.
3. Install one server integration from Releases:
   - Paper/Purpur/Spigot: `scopenet-paper-VERSION.jar` in `plugins/`. It is **one JAR for every Minecraft version from 1.20 up** (1.20.1, 1.21.x, 26.x): it uses only the stable Bukkit API (a 1.20.1 baseline, Java 17 bytecode) and no NMS. Install only one SCOPENET Paper JAR and replace the older plugin JAR when updating. Folia is not supported.
   - Fabric: `scopenet-fabric-MC-VERSION.jar` in `mods/`. Sign-in, stats, claims and the Live Map need no Fabric API or client mod. The 1.20.1 build adds commands, permissions, placeholders and the client link, and needs Fabric API; see [fabric.md](fabric.md).
   - Forge: `scopenet-forge-MC-VERSION.jar` in `mods/`. Server-only installation; clients do not need SCOPENET.
   - The Fabric and Forge builds are **exact-version** artifacts for **1.20.1, 1.21.1, 26.1.2, 26.2 and 26.3**. Do not install one on a different Minecraft version. Future 26.x releases need their own verified builds.
4. Configure Paper at `plugins/SCOPENET/config.yml`:

   ```yaml
   panel-url: "https://panel.example.com"
   token: "sn_COPY_THE_SERVER_TOKEN_HERE"
   ```

   Fabric/Forge use `config/scopenet.properties`:

   ```properties
   panel-url=https://panel.example.com
   token=sn_COPY_THE_SERVER_TOKEN_HERE
   ```

5. Download `https://panel.example.com/api/v1/launcher/authlib-injector.jar` beside the server JAR. Add the JVM flag **before** `-jar`:

   ```sh
   java -javaagent:authlib-injector.jar=https://panel.example.com/api/yggdrasil -Xmx4G -jar server.jar nogui
   ```

   For Forge's generated `run.sh`/`run.bat`, put the same `-javaagent:…` line in `user_jvm_args.txt`.
6. Keep `online-mode=true` in `server.properties`, restart, and sign in through the launcher. Java requirements: 17+ for 1.20.1, 21+ for 1.21.1, and 25+ for 26.x.

Missing/invalid configuration blocks new logins (Paper) or aborts startup (mods). Network errors, invalid responses, exhausted login workers and timeouts deny admission. Login requests are bounded and do not block the server tick. Existing players are not disconnected solely because of a temporary panel outage; revocations resume when heartbeats recover.

## Permanent identity and impersonation protection

- New panel accounts receive a random UUID once. The database rejects subsequent UUID changes.
- Existing accounts retain their original UUID, including UUIDs migrated from the old offline-account scheme. **Never delete player data or recalculate UUIDs when renaming.**
- Launcher **Settings → Skin & cape** supports skin upload/reset, arm model, permitted capes and password-confirmed username changes. Renames keep the account UUID, permissions, skins/capes and UUID-keyed Minecraft inventory/advancements. Old names stay reserved to that identity, even after account deletion.
- Renaming invalidates old panel/game sessions and creates a new session for the requesting launcher. Players must close Minecraft first. Plugins that store data by *name* instead of UUID may need their own migration; SCOPENET cannot rewrite third-party storage.
- Server login authorization uses the session-authenticated UUID. A matching name never grants another account's access. Tokens are server-specific; rotate one from the panel if exposed.
- Use HTTPS for the panel. Keep its database, JWT secret and `yggdrasil-signing.pem` private and backed up. Do not expose an offline-mode backend through a proxy: this release supports directly authenticated servers, not proxy-forwarded identities.
- Forwarded IP headers are ignored unless the socket peer is listed in `SCOPENET_TRUSTED_PROXIES` (comma-separated literal IPs). Configure your reverse proxy to replace/append forwarded addresses and restrict direct access to the backend. Set `PUBLIC_URL`; do not derive production URLs from untrusted request headers.
- **Require launcher** checks for a recent authenticated launch from the same IP. It is an additional access rule, not proof of an unmodified launcher: a modified authenticated client can reproduce its HTTP request. Password/session verification is the identity boundary.

## Activity recorded

The panel's **Activity** page filters by source and username/UUID. **Servers** shows online players, estimated TPS, playtime, leaderboards and recent server events.

- All integrations: joins/leaves, elapsed playtime, deaths, player/mob kills, mined/placed blocks, **chat counts only**, command **names only**, inventory-click activity, advancement activity and vanilla statistic increments (such as crafting, item use/pickup/drop, movement and damage). Repeated statistic increments are grouped per reporting interval.
- Paper additionally records inventory action/slot/item metadata, advancement names and teleport causes/destinations through Bukkit events. Fabric/Forge record advancement criteria and inventory-click counts through Minecraft hooks.
- Launcher: opening/closing through the launcher UI, account selection/sign-out, settings changes, install/repair start/completion, launch/cancel/failure, game exit/crash/termination, instance deletion, cache clearing, opening folders/links and update installation. Account/profile/admin changes and successful sign-ins are audited by the panel. Launcher identity comes from the bearer token, never a submitted username.
- No chat text, private messages, command arguments, passwords, tokens, game logs or local paths are uploaded. Launcher records are client-reported; server records are authenticated with the server token.

This is an activity log, not a packet/world replay or block rollback system. Actions performed internally by other mods/plugins without vanilla statistics or the covered event hooks need additional adapters. Entity AI, redstone ticks and every movement packet are not individually recorded.

Heartbeats run every 30 seconds. Failed server batches are retried with a stable ID and transactionally deduplicated by the panel. Queues are bounded; overload emits `telemetry_gap` rather than silently pretending coverage is complete. A clean stop attempts a final flush. Unsent data is memory-resident, so crashes or a prolonged outage can lose queued activity. Launcher reports are best-effort, and game-exit/crash reports are unavailable if **Close launcher** was selected. Server event retention is 60 days; sync receipts remain until the server entry is deleted.

## Build and verify

Use Java 21 + Gradle 8.10.2 for legacy builds, Java 25 + Gradle 9.6.0 for 26.x:

```sh
gradle -p integrations :common:test :paper:build
gradle -p integrations -Ploader=fabric -PmcVersion=1.20.1 :fabric:build
gradle -p integrations -Ploader=forge -PmcVersion=1.21.1 :forge:build
gradle -p integrations -Ploader=fabric -PmcVersion=26.3 :fabric:build
```

The optional Fabric client mod builds with `-Ploader=fabric-client :fabric-client:build`. Substitute `fabric`/`forge` and the exact version as required. JARs are in the selected module's `build/libs/`. `-PreleaseVersion=X.Y.Z` stamps the artifact and metadata. The locally built 0.4.0 matrix for 1.20.1, 1.21.1, and 26.3 plus Paper (built per version then, a single JAR from 0.6 on) is in [`release-artifacts/0.4.0`](../release-artifacts/0.4.0/README.md), with SHA-256 checksums. The **Server integrations** workflow can rebuild the matrix when Actions usage is available again.

Before production, test a valid login, wrong password, disabled account, denied group, missing launcher launch, panel outage, name-change/reconnect with the same inventory, chat count, heartbeat retry, and restart on a copy of your server world. Build/API tests do not replace a live Minecraft client/server compatibility check with your modpack.
