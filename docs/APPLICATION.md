# Maintained Velora application composition

This private workspace contains the complete current admin/player website,
backend, desktop launcher and Minecraft host composition. It preserves private
gameplay while the reusable packages are maintained by their public owners.
The frozen archive is separate and unchanged. This is maintained application
source, not an archive or placeholder. No SCOPENET-MC checkout is needed to build.

The original-source import record and adaptations are kept in
`migration/application-host/` for reference. Run from the repository root:

```sh
node scripts/verify-identity.mjs
cd panel/web
npm ci --no-audit --no-fund
npm run check
npm run build
```

Then, from the repository root, `cargo run --locked -p scopenet-panel` starts the complete
site at port 8080. Set the admin bootstrap password through the process environment,
and use a fresh development data directory. For existing installations explicitly
supply their existing data directory; do not open two credential writers on it.
Legacy environment names, API routes, SQLite schema/data paths and volume names
remain compatible. `VELORA_*` aliases continue to work. `docker compose up --build`
builds the same full application with a non-root runtime and persistent storage.

For desktop development, run `npm ci` and `npm run tauri dev` inside `launcher/`.
`npm run tauri build` uses Windows NSIS, Linux AppImage/deb, or macOS app/DMG bundles
on the corresponding OS. Installed application/credential-store IDs remain stable.
Cross-platform build validation does not replace real desktop/game upgrade tests.
Installers are built without paid signing accounts. OS trust prompts and the
custom updater's checksum validation remain distinct from a signed updater trust
chain; do not claim production updater signing/notarization is configured.

Android/iOS player shell source belongs to the independent Panel repository in
`mobile/`. Supply the deployed HTTPS origin there; the complete player site here
provides its routes and features. Android direct distribution uses a persistent
self-signed key at no certificate/store cost. Users can sign the iOS IPA locally
with their free Apple Account using AltStore Classic on Windows or macOS, refreshing
free provisioning within seven days. See the Panel mobile instructions; no paid
developer membership or store publication is required for this sideloading route.

From the repository root, after building the backend and web assets, run
`node scripts/application-acceptance.mjs target/debug/scopenet-panel`
(use the `.exe` suffix on Windows). This uses fresh local temporary data only and
checks cold backup/restore, preserved credentials/roles and point-in-time state.
It never opens an operator data directory or restores an existing installation.

Public application separation is still in progress. This private composition can
preserve all current features now; it does not make private gameplay public or
certify live data cutover, full release parity or production deployment.
