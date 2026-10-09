# Velora launcher

Tauri desktop app with instance selection, community features and client settings.
The native Rust engine lives in ../crates/core; browser UI development can run
without a Panel backend through its built-in synthetic fixture.

## Browser preview

From this directory with Node 24:

```sh
npm ci --no-audit --no-fund
npm run dev
```

Open http://localhost:1420/?mock=main. Supported fixtures: main, setup, login,
update and progress. They are UI demos; native launching requires Tauri.

## Validation and packaging

```sh
npm run check
npm run check:runes
npm run build
npm run tauri dev
npm run tauri build
```

Native commands require Rust 1.98.1 and the target OS's Tauri toolchain. Preserve
net.scopenet.launcher, credential-store identity and signing continuity. Desktop
installers share the existing download origin; the UI needs no separate server.
See [application development](../docs/APPLICATION.md), [validation](../docs/VALIDATION.md)
and [deployment targets](../docs/DEPLOYMENT_TARGETS.md).
