> Historical owner documentation from `VeloraMCDev/launcher` at `404d3b6a007f7b795e2d3efbabd3cb2308d9e362`. Current segment instructions are in the monorepo READMEs.

<p align="center">
  <img src=".github/assets/banner.png" width="1200" alt="Velora Launcher — From installation to play" />
</p>

# Velora Launcher

[![CI](https://github.com/VeloraMCDev/launcher/actions/workflows/engine.yml/badge.svg?branch=scopedd%2Fvelora-migration)](https://github.com/VeloraMCDev/launcher/actions/workflows/engine.yml) [![Code: MIT](https://img.shields.io/badge/Code-MIT-8b5cf6?style=flat-square)](LICENSE) ![Status: Independent engine](https://img.shields.io/badge/Status-Independent%20engine-6d28d9?style=flat-square) ![Stack: Rust · desktop target](https://img.shields.io/badge/Stack-Rust%20%C2%B7%20desktop%20target-334155?style=flat-square)

Velora’s game installation and launch layer. The extracted engine handles local Minecraft installations, loader selection, Java acquisition and launch preparation independently of the Panel and private gameplay.

**Velora owns the platform. Each instance owns the experience.**

[Migration progress](https://github.com/VeloraMCDev/docs/blob/scopedd/velora-migration/migration/CURRENT_PROGRESS.md) · [Execution plan](https://github.com/VeloraMCDev/docs/blob/scopedd/velora-migration/migration/VELORA_EXECUTION_PLAN.md) · [Checks](https://github.com/VeloraMCDev/launcher/actions/workflows/engine.yml)

## Available today

- **[velora-engine](https://github.com/VeloraMCDev/launcher/blob/404d3b6a007f7b795e2d3efbabd3cb2308d9e362/engine)** — Minecraft/Java installation, vanilla and loader metadata, modpack synchronization, repair, launch construction and local options.
- **Offline state** — existing cache and persisted installation behavior stays with the engine.
- **[SDK snapshot](https://github.com/VeloraMCDev/launcher/blob/404d3b6a007f7b795e2d3efbabd3cb2308d9e362/vendor/sdk)** — reviewed contract/utility sources with pinned revision and hashes; builds need no sibling repository or private credentials.
- **Engine CI** — independent formatting, build and offline regression checks.

## Develop

Use Rust with rustfmt and Node.js 24 for the snapshot check. From the repository root:

```sh
node scripts/verify-sdk.mjs
cargo fmt --all -- --check
cargo test --workspace --locked
```

Offline tests validate local behavior. Online installation fixtures remain opt-in/ignored in the normal suite and are not evidence that every current upstream distribution has been downloaded successfully.

## Desktop and mobile scope

The Tauri/Svelte shell, generic experience UI composition, native installers/updater wiring and mobile/web packaging are still being extracted. There is no replacement application installer from this checkpoint. Existing applications continue to use the compatibility host.

The product will use operator-supplied hosts rather than an invented Velora service domain. Existing native app IDs, keyring/updater identities and saved configuration require compatibility checks before any rename. Legacy configuration remains readable.

## Extension boundary

The public launcher hosts generic UI contributions through SDK contracts. Private gameplay pages and presets belong to their owner and must not become a public build dependency. The Panel distributes instances; it does not embed this installation engine as a backend dependency.

## Adoption and releases

Review SDK changes and refresh their verified snapshot intentionally. Package publication, signing and updater activation remain release gates. See [engine](https://github.com/VeloraMCDev/launcher/blob/404d3b6a007f7b795e2d3efbabd3cb2308d9e362/engine) and the migration tracker for source-level adoption evidence; preserved package names/paths may still carry legacy compatibility identifiers.

## Migration and provenance

This is an active migration from the private SCOPENET-MC application. Frozen source revision: `57daa92cb6cb4027982d06bfa1c692ab9edb88ff`. Library/segment availability described above is distinct from complete feature-parity, upgrade and deployment acceptance. The current report records the actual validation checkpoint and remaining work.

## License and brand

Platform code uses the [MIT license](https://github.com/VeloraMCDev/launcher/blob/404d3b6a007f7b795e2d3efbabd3cb2308d9e362/LICENSE). See [CONTRIBUTING.md](https://github.com/VeloraMCDev/launcher/blob/404d3b6a007f7b795e2d3efbabd3cb2308d9e362/CONTRIBUTING.md) for contribution expectations. Official Velora artwork and trademarks are excluded from MIT; no separate artwork reuse grant is provided. Banner provenance is in [.github/assets](https://github.com/VeloraMCDev/launcher/blob/404d3b6a007f7b795e2d3efbabd3cb2308d9e362/.github/assets/README.md). Repository preparation does not change visibility or publish packages.

## Explore the public platform

[Infra](https://github.com/VeloraMCDev/infra) · [Docs](https://github.com/VeloraMCDev/docs) · [SDK](https://github.com/VeloraMCDev/sdk) · [Authentication](https://github.com/VeloraMCDev/authentication) · [Minecraft Integrations](https://github.com/VeloraMCDev/minecraft-integrations) · [Panel](https://github.com/VeloraMCDev/panel) · [Launcher](https://github.com/VeloraMCDev/launcher)
