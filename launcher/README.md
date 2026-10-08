# Launcher

Tauri desktop frontend and src-tauri shell; the Rust engine is ../crates/core.

## Local checks

```sh
npm ci --no-audit --no-fund
npm run check
npm run check:runes
npm run build
# Native desktop packaging requires the documented platform toolchain
npm run tauri build
```

Run frontend commands here. Preserve net.scopenet.launcher, credential-store identities and update signing continuity. Installers are downloadable artifacts; the desktop app does not need a server to run its UI.

See the root README for ownership, deployment and publication status.
