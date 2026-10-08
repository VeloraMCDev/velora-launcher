> Historical owner documentation from `VeloraMCDev/sdk` at `de3b8fd7539c3b9d20fbc8a3b58317f61a321faa`. Current segment instructions are in the monorepo READMEs.

<p align="center">
  <img src=".github/assets/banner.png" width="1200" alt="Velora SDK — Contracts that connect the platform" />
</p>

# Velora SDK

[![CI](https://github.com/VeloraMCDev/sdk/actions/workflows/contracts.yml/badge.svg?branch=scopedd%2Fvelora-migration)](https://github.com/VeloraMCDev/sdk/actions/workflows/contracts.yml) [![Code: MIT](https://img.shields.io/badge/Code-MIT-8b5cf6?style=flat-square)](LICENSE) ![Status: Independent libraries](https://img.shields.io/badge/Status-Independent%20libraries-6d28d9?style=flat-square) ![Stack: Rust · Java · Svelte](https://img.shields.io/badge/Stack-Rust%20%C2%B7%20Java%20%C2%B7%20Svelte-334155?style=flat-square)

The shared vocabulary and reusable mechanisms behind Velora. Build platform extensions, integrate Minecraft servers and compose interfaces without depending on a private gameplay implementation.

**Velora owns the platform. Each instance owns the experience.**

[Migration progress](https://github.com/VeloraMCDev/docs/blob/scopedd/velora-migration/migration/CURRENT_PROGRESS.md) · [Execution plan](https://github.com/VeloraMCDev/docs/blob/scopedd/velora-migration/migration/VELORA_EXECUTION_PLAN.md) · [Checks](https://github.com/VeloraMCDev/sdk/actions/workflows/contracts.yml)

## What lives here

- **Rust contracts** — API v1 wire declarations, account/instance models and neutral experience envelopes in [rust/platform-contracts](https://github.com/VeloraMCDev/sdk/blob/de3b8fd7539c3b9d20fbc8a3b58317f61a321faa/rust/platform-contracts).
- **[Browser HTTP client](https://github.com/VeloraMCDev/sdk/blob/de3b8fd7539c3b9d20fbc8a3b58317f61a321faa/typescript/http-client/README.md)** — fifteen typed launcher/auth/account endpoints, generic API v1 transport, live session/scope/lifecycle ports, deadlines/cancellation and frozen-client/loopback compatibility tests.
- **Rust utilities** — verified downloads, Minecraft/loader metadata, safe filesystem paths and authlib-injector artifact metadata in [rust/platform-utils](https://github.com/VeloraMCDev/sdk/blob/de3b8fd7539c3b9d20fbc8a3b58317f61a321faa/rust/platform-utils).
- **Java transport and ports** — neutral HTTP communication, map lifecycle/position/factory ports and provider version checks in [java/platform-client](https://github.com/VeloraMCDev/sdk/blob/de3b8fd7539c3b9d20fbc8a3b58317f61a321faa/java/platform-client/README.md).
- **Java extension API** — existing API/provider declarations, optional Bukkit events and a compile-only consumer example in [java](https://github.com/VeloraMCDev/sdk/blob/de3b8fd7539c3b9d20fbc8a3b58317f61a321faa/java/README.md).
- **Svelte widget host** — branding declarations and a local component registry supplied by the application in [ui/experience](https://github.com/VeloraMCDev/sdk/blob/de3b8fd7539c3b9d20fbc8a3b58317f61a321faa/ui/experience/README.md).
- **[Map viewer](https://github.com/VeloraMCDev/sdk/blob/de3b8fd7539c3b9d20fbc8a3b58317f61a321faa/ui/map/README.md)** — canvas rendering, live feed, API v1 map DTOs and Svelte viewer with injected URLs/data; private claim policy stays with the experience.
- **[Skin viewer](https://github.com/VeloraMCDev/sdk/blob/de3b8fd7539c3b9d20fbc8a3b58317f61a321faa/ui/skin/README.md)** — original flat skin/cape canvas, classic/slim and legacy geometry, overlays and stale-image cancellation, with caller-owned URLs.
- **Official artwork** — exact original emblem/wordmark files, checksums and derived wrappers in [branding](https://github.com/VeloraMCDev/sdk/blob/de3b8fd7539c3b9d20fbc8a3b58317f61a321faa/branding/README.md).

Gameplay DTOs describe messages crossing a boundary. Payout rules, XP curves, private presets, catalogs and application state belong to their implementations.

## Develop

Use Rust 1.98.1, Java 21 (Java 17 output) and Node.js 24. Each segment can be checked independently.

```sh
cargo +1.98.1 fmt --all -- --check
cargo +1.98.1 test --workspace --locked
cargo +1.98.1 clippy --workspace --all-targets --locked -- -D warnings
```

```sh
cd java
./platform-client/gradlew build --no-daemon
# Windows: platform-client\gradlew.bat build --no-daemon
```

```sh
cd ui/experience
npm ci --ignore-scripts --no-audit --no-fund
npm run check
npm test
```

```sh
cd typescript/http-client
npm ci --ignore-scripts --no-audit --no-fund
npm run check
npm test
```

Run each segment block from a fresh repository-root terminal. CI checks Rust, Java, UI and HTTP transport. Rust packages resolve public registry dependencies; Java and UI builds require no private checkout.

Check `ui/map` and `ui/skin` with the same npm commands as `ui/experience`.
Their tests exercise actual canvas/image controllers and standalone Svelte rendering
with synthetic data; the map package also tests polling.

## Integrate

Keep exactly one runtime identity for the Java API/provider classes. The example consumer uses compileOnly; bundling another API copy breaks provider discovery. Widgets use locally installed components and reject unknown registry names; manifests do not download executable code.

Published registry coordinates, canonical schema generation, additional client bindings and the remaining reusable form/skin primitives are migration work. Current consumers use reviewed, pinned snapshots with drift checks; these snapshots are a bridge to package releases.

## Compatibility

Legacy API v1 field names, existing UUIDs and Java protocol/provider namespaces remain stable. See [PROVENANCE.md](https://github.com/VeloraMCDev/sdk/blob/de3b8fd7539c3b9d20fbc8a3b58317f61a321faa/PROVENANCE.md), [API provenance](https://github.com/VeloraMCDev/sdk/blob/de3b8fd7539c3b9d20fbc8a3b58317f61a321faa/java/API_PROVENANCE.json) and segment READMEs for extraction details.

## Migration and provenance

This is an active migration from the private SCOPENET-MC application. Frozen source revision: `57daa92cb6cb4027982d06bfa1c692ab9edb88ff`. Library/segment availability described above is distinct from complete feature-parity, upgrade and deployment acceptance. The current report records the actual validation checkpoint and remaining work.

## License and brand

Platform code uses the [MIT license](https://github.com/VeloraMCDev/sdk/blob/de3b8fd7539c3b9d20fbc8a3b58317f61a321faa/LICENSE). See [CONTRIBUTING.md](https://github.com/VeloraMCDev/sdk/blob/de3b8fd7539c3b9d20fbc8a3b58317f61a321faa/CONTRIBUTING.md) for contribution expectations. Official Velora artwork and trademarks are excluded from MIT; no separate artwork reuse grant is provided. Banner provenance is in [.github/assets](https://github.com/VeloraMCDev/sdk/blob/de3b8fd7539c3b9d20fbc8a3b58317f61a321faa/.github/assets/README.md). Repository preparation does not change visibility or publish packages.

Bundled third-party build bootstrap files retain their upstream licenses. See [artifact provenance](https://github.com/VeloraMCDev/sdk/blob/de3b8fd7539c3b9d20fbc8a3b58317f61a321faa/PUBLIC_ARTIFACTS.json) and [Gradle wrapper attribution](https://github.com/VeloraMCDev/sdk/blob/de3b8fd7539c3b9d20fbc8a3b58317f61a321faa/third-party/gradle-wrapper/README.md); repository MIT terms do not replace them.

## Explore the public platform

[Infra](https://github.com/VeloraMCDev/infra) · [Docs](https://github.com/VeloraMCDev/docs) · [SDK](https://github.com/VeloraMCDev/sdk) · [Authentication](https://github.com/VeloraMCDev/authentication) · [Minecraft Integrations](https://github.com/VeloraMCDev/minecraft-integrations) · [Panel](https://github.com/VeloraMCDev/panel) · [Launcher](https://github.com/VeloraMCDev/launcher)
