# Development

**Requirements:** Rust (stable), Node 22. On Linux, also the Tauri deps: `libwebkit2gtk-4.1-dev librsvg2-dev libayatana-appindicator3-dev`.

## Panel

```bash
cd panel/web && npm install && npm run build && cd ../..
ADMIN_PASSWORD=adminadmin cargo run -p scopenet-panel     # http://localhost:8080
# UI hot reload: (cd panel/web && npm run dev)            # http://localhost:5173, proxies /api
```

## Launcher

```bash
cd launcher && npm install
npm run tauri dev          # full desktop app
npm run dev                # UI only in a browser, with a mock backend (http://localhost:1420)
```

Browser mock scenarios: `?mock=setup`, `?mock=login`, `?mock=progress`.

## Shared UI code

`shared/casino`, `shared/map` and `shared/commands` are imported by both `launcher` and `panel/web` through the `@scopenet/casino`, `@scopenet/map` and `@scopenet/commands` aliases (see each `vite.config.ts` and `tsconfig.json`). The casino talks to the panel through a small host adapter (`setCasinoHost`) so the same components work over Tauri and over `fetch`. After editing `shared/commands/index.ts`, regenerate the command docs:

```bash
node --experimental-strip-types scripts/gen-commands-doc.mjs   # writes docs/commands.md
```

The player panel lives in `panel/web/src/play/` (shell, design system `play.css`, one lazily loaded file per page in `pages/`).

Build-time options (environment variables when building): `SCOPENET_PANEL_URL`, `SCOPENET_LOCK_PANEL=1`, `SCOPENET_REPO=owner/repo`.

## Tests

```bash
cargo test --workspace                                         # unit + panel API tests
cargo test -p scopenet-core --test online -- --ignored        # real installs (needs internet, ~2 GB)
(cd panel/web && npm run check) && (cd launcher && npm run check)
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
```

## Releasing

Push a tag `vX.Y.Z` or run the **Release** workflow. It stamps the version into `Cargo.toml` and `tauri.conf.json`, builds the NSIS installer on Windows (plus a Linux AppImage/.deb and a universal macOS .dmg, which are optional and never block a release), pushes the multi-arch panel image to GHCR, and publishes the release.

## Server integrations

See [server integration builds and validation](server-integration.md#build-and-verify). The Java modules live under `integrations/` and the release workflow publishes the per-loader/per-version JARs.

## Demo data

`scripts/demo/run.sh` starts a throwaway panel on `127.0.0.1:18080` and fills it with a lively demo community (players, guilds, market, casino, social, quests) through the real HTTP APIs - handy for trying the player pages and taking screenshots. Sign in as `admin` / `demo-admin-pass` or `Alex_Miner` / `demo-pass-1234`; `scripts/demo/stop.sh` stops it. See [scripts/demo/README.md](../scripts/demo/README.md).

## App icons

All platform icons are generated from `branding/icon.svg` and `branding/icon-fullbleed.svg`: run `node scripts/icons/build.mjs` (needs Playwright with Chromium and the launcher's `npm ci`). See `branding/README.md` for what it writes.
