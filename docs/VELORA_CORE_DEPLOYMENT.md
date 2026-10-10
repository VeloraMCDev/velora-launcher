# Deploying and updating Velora Core

Velora Core reaches game servers the same way every other Velora build does: a manual GitHub Actions workflow builds and signs it, an admin approves it in the Panel, and only then does it go anywhere. Nothing deploys automatically and no paid service is involved.

```text
Velora Core release workflow ──► GitHub release (signed jars)
                                       │  admin approves
                                       ▼
                         Velora Panel serves the jars
                                       │  /api/v1/core/latest
                                       ▼
                 Calagopus extension installs / updates them on each server
```

## 1. Publish a release

1. Bump `releaseVersion` in `integrations/build.gradle` (the default in `getOrElse`) and merge to `main`.
2. Run **Velora Core release** from the main branch in GitHub Actions. It builds both Fabric 1.20.1 jars, signs each with the Velora release key and publishes the GitHub release `velora-core-v<version>` with `core-manifest.json` and `SHA256SUMS`. It refuses a version that is already released.

Signatures cover `velora-core-release:v1`, the version and the jar's SHA-256. They are a different domain from launcher signatures, so one can never be replayed as the other.

## 2. Approve it in the Panel

Open **Servers** in the admin panel and use **Velora Core** at the bottom. **Check for releases** lists signed releases; **Approve** downloads both jars, verifies size, checksum and signature, and starts serving them. Servers that report an older version show a badge. The previous three approved releases stay downloadable, and approving an older release rolls back.

The Panel exposes two public, read-only endpoints (the jars are public GitHub assets):

| Endpoint | Purpose |
| --- | --- |
| `GET /api/v1/core/latest` | The approved release: version, Minecraft version, and each jar's name, size, SHA-256 and path. `null` before the first approval. |
| `GET /api/v1/core/files/{sha256}/{name}` | One approved jar. |

Admin routes are `GET /api/admin/core/releases` and `POST /api/admin/core/releases/{tag}/approve`.

## 3. Install it on a Calagopus server

The extension in [`integrations/calagopus`](https://github.com/VeloraMCDev/velora-launcher/blob/main/integrations/calagopus/README.md) adds a **Velora Core** page to every server. It:

- shows the installed and approved versions;
- installs the server jar into `mods/`, checks its SHA-256 on the node, then removes older Velora Core (or `scopenet-fabric`) jars;
- writes `config/velora-core.properties` with the Panel address and the server token you paste (the token goes only to the server's own file; Calagopus does not keep it);
- downloads `authlib-injector.jar` and shows the `-javaagent` start flag;
- optionally updates the server by itself.

### Automatic updates

A server opts in on its Velora Core page. When a newer release is approved, the extension installs it **only while the server is stopped**, so the next restart picks it up and players are never interrupted. It never replaces a jar under a running server. While a server waits for that moment the extension checks every few seconds; otherwise every five minutes. A manual **Update now** needs the server stopped too, except for a first install.

Automatic updates can be switched off for the whole Panel in the extension settings. The extension reports the last thing it did to each server on the page and in the Panel log.

### Players

Velora Core (Client) is not deployed by the extension. Players add `velora-core-client-1.20.1-<version>.jar` and Fabric API to their own `mods` folder; both jars are on the release.

## What this does not do

- It does not restart servers or change startup commands. Add the `-javaagent` flag yourself.
- It does not touch other mods, worlds or player data.
- It has been written against the documented Calagopus extension API and compile-checked, but it has not yet been installed on a live Calagopus panel. Test on a disposable server first, as described in the extension README.
