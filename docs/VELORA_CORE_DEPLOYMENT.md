# Deploying and updating Velora Core

Velora Core reaches game servers the same way every other Velora build does: a manual GitHub Actions workflow builds and signs it, an admin approves it in the Panel, and only then does it go anywhere. Nothing deploys automatically and no paid service is involved.

```text
Velora Core release workflow ──► GitHub release (signed jars)
                                       │  admin approves
                                       ▼
                         Velora Panel serves the jars
                                       │  /api/v1/core/latest
                                       ▼
                 Operator downloads Server / Client jars and uploads them manually
```

## 1. Publish a release

1. Bump `releaseVersion` in `integrations/build.gradle` (the default in `getOrElse`) and merge to `main`.
2. Run **Velora Core release** from the main branch in GitHub Actions. It builds both Fabric 1.20.1 jars, signs each with the Velora release key and publishes the GitHub release `velora-core-v<version>` with `core-manifest.json` and `SHA256SUMS`. It refuses a version that is already released.

Signatures cover `velora-core-release:v1`, the version and the jar's SHA-256. They are a different domain from launcher signatures, so one can never be replayed as the other.

## 2. Approve it in the Panel

Open **Instance setup / Mod downloads** in the admin panel and use **Velora Core**. **Check for releases** lists signed releases; **Approve** downloads both jars, verifies size, checksum and signature, and starts serving them. Servers that report an older version show a badge. The previous three approved releases stay downloadable, and approving an older release rolls back.

The Panel exposes two public, read-only endpoints (the jars are public GitHub assets):

| Endpoint | Purpose |
| --- | --- |
| `GET /api/v1/core/latest` | The approved release: version, Minecraft version, and each jar's name, size, SHA-256 and path. `null` before the first approval. |
| `GET /api/v1/core/files/{sha256}/{name}` | One approved jar. |

Admin routes are `GET /api/admin/core/releases` and `POST /api/admin/core/releases/{tag}/approve`.

## 3. Install the jars manually

1. In **Instance setup / Mod downloads**, download the **Server jar** and **Client jar**. Each download includes its filename, size and SHA-256 checksum.
2. Stop the dedicated server. Upload the Server jar to its `mods/` folder, remove the previous Velora Core server jar, then restart.
3. Upload the Client jar to the instance modpack's `mods/` folder in **Version & modpack**, or put it in each player's local Minecraft `mods/` folder. Remove previous client jars.
4. Install Fabric and Fabric API for the release's Minecraft version on both sides.

Approval makes verified jars downloadable; it does not modify a server or an instance modpack. The Calagopus extension has no automatic updater or mod-install endpoint. Existing automatic-update settings are ignored. Ordinary launcher modpack synchronization still installs files that an admin explicitly includes in an instance.

The optional extension reports installed versions, writes `config/velora-core.properties` using the token you paste, and can download authlib-injector and show its startup flag. It does not install or replace mod jars.

## What this does not do

- It does not restart servers or change startup commands. Add the `-javaagent` flag yourself.
- It does not touch other mods, worlds or player data.
- The extension is built, unit-tested and exported against Calagopus 1.2.4 (the supported version) and the next panel, `main`, by the Calagopus extension workflow. It has not yet been installed on a live Calagopus panel or run against a real Wings node, so the file operations (pull, checksum, delete, power state) are unproven end to end. Test on a disposable server first.
