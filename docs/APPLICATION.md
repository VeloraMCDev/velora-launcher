# Local application development

The complete public source includes the Panel backend, admin/player websites,
desktop launcher, gameplay and Minecraft integrations. No private source checkout
is required. Use Rust 1.98.1 and Node 24; see segment READMEs for Java and mobile
toolchains.

## Panel

From the repository root:

```sh
cd panel/web
npm ci --no-audit --no-fund
npm run check
npm run check:runes
npm run build
cd ../..
cargo build --locked -p scopenet-panel
```

Before running the backend, set ADMIN_PASSWORD to a local development password,
VELORA_BIND to 127.0.0.1:8080 and VELORA_DATA_DIR to a fresh ignored development
directory. Then run `cargo run --locked -p scopenet-panel` and open
http://localhost:8080. Supply a private JWT_SECRET for a persistent installation.
The public-safe examples intentionally contain no production credentials.

Only one credential writer may open a store. Existing installations must explicitly
retain their existing data directory and signing material. VELORA_* aliases and
legacy environment names, API routes and SQLite/volume identities remain supported.
The root Dockerfile and Compose definition build the same complete application;
Docker image acceptance is a separate release gate.

For fictional players and sample content, use [the isolated demo](https://github.com/VeloraMCDev/velora-launcher/blob/main/scripts/demo/README.md).

## Desktop launcher

From launcher/:

```sh
npm ci --no-audit --no-fund
npm run dev
```

Open http://localhost:1420/?mock=main for the browser demo. It uses in-memory
fixtures; launching Minecraft and native OS integration require Tauri.
Use `npm run tauri dev` for desktop development and `npm run tauri build`
for packaging on the target OS. See [launcher](https://github.com/VeloraMCDev/velora-launcher/blob/main/launcher/README.md).
Installed app and credential-store IDs remain stable. Production updater signing
and notarization must be configured and tested before claiming trusted releases.

## Player mobile shells and acceptance

Mobile shell source is under [panel/mobile](https://github.com/VeloraMCDev/velora-launcher/blob/main/panel/mobile/README.md). Supply the
deployed HTTPS player origin; platform packaging has its own prerequisites.

After building the backend and web assets, run from the repository root:

```sh
node scripts/application-acceptance.mjs target/debug/scopenet-panel
```

On Windows add the .exe suffix. The check uses fresh temporary stores and verifies
cold backup/restore, credentials, roles and point-in-time state. It never restores
an operator installation. Passing it does not certify production hosting or desktop
upgrade behavior. See [deployment gates](DEPLOYMENT_TARGETS.md).
