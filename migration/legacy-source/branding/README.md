# SCOPENET app icon

One mark, used everywhere. Edit these two files and run `node scripts/icons/build.mjs`.

- `icon.svg` — the rounded app icon (desktop, website, mod list).
- `icon-fullbleed.svg` — the same mark edge to edge with a little more padding, for iOS and Android, which round and crop it themselves.

The script regenerates every platform's files from them:

| Where | Files |
| --- | --- |
| Windows / macOS / Linux launcher | `launcher/src-tauri/icons/*` |
| Android / iOS app | `mobile/assets/icon.png`, `mobile/assets/splash.png` |
| Website and installable web app | `panel/web/public/favicon.svg`, `icon-192.png`, `icon-512.png`, `icon-maskable-512.png`, `apple-touch-icon.png` |
| Fabric companion mod | `integrations/fabric-client/src/26/resources/assets/scopenet_client/icon.png` |
