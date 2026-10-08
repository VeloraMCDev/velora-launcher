> Historical owner documentation from `VeloraMCDev/minecraft-integrations` at `410bad40189c3af73d24f7cf37be57a4a86f14b9`. Current segment instructions are in the monorepo READMEs.

<p align="center">
  <img src=".github/assets/banner.png" width="1200" alt="Velora Minecraft Integrations — Connect the game to Velora" />
</p>

# Velora Minecraft Integrations

[![CI](https://github.com/VeloraMCDev/minecraft-integrations/actions/workflows/producer.yml/badge.svg?branch=scopedd%2Fvelora-migration)](https://github.com/VeloraMCDev/minecraft-integrations/actions/workflows/producer.yml) [![Code: MIT](https://img.shields.io/badge/Code-MIT-8b5cf6?style=flat-square)](LICENSE) ![Status: Independent map library](https://img.shields.io/badge/Status-Independent%20map%20library-6d28d9?style=flat-square) ![Stack: Java 17 · Gradle](https://img.shields.io/badge/Stack-Java%2017%20%C2%B7%20Gradle-334155?style=flat-square)

The public Minecraft integration layer. It connects game runtimes to platform services through neutral SDK contracts and provides vanilla world-map production without importing private gameplay policy.

**Velora owns the platform. Each instance owns the experience.**

[Migration progress](https://github.com/VeloraMCDev/docs/blob/scopedd/velora-migration/migration/CURRENT_PROGRESS.md) · [Execution plan](https://github.com/VeloraMCDev/docs/blob/scopedd/velora-migration/migration/VELORA_EXECUTION_PLAN.md) · [Checks](https://github.com/VeloraMCDev/minecraft-integrations/actions/workflows/producer.yml)

## Available components

- **[Map producer](https://github.com/VeloraMCDev/minecraft-integrations/blob/410bad40189c3af73d24f7cf37be57a4a86f14b9/producer)** — region/NBT decoding, palette lookup, tile rendering and PNG generation, including the original block-color resource.
- **Upload lifecycle** — queued tiles, restart/retry handling, player snapshots and opaque caller-supplied overlays.
- **Local provider** — explicit API version and class-loader discovery through the SDK map factory/lifecycle/position ports.
- **[Reviewed SDK transport](https://github.com/VeloraMCDev/minecraft-integrations/blob/410bad40189c3af73d24f7cf37be57a4a86f14b9/vendor/platform-client)** — strict panel URL handling, bounded HTTP requests and wire ports; hashes are checked before builds.

Claims, gameplay settings, reward rules and private overlay interpretation are supplied by their owning extensions. The producer treats overlay payloads as opaque data.

## Build and test

Use Java 21 and Node.js 24. The output targets Java 17. The checked-in Gradle wrapper pins the toolchain distribution; tests generate region fixtures and use a synthetic local HTTP panel.

```sh
node scripts/verify-sdk.mjs
./gradlew build --no-daemon
# Windows: gradlew.bat build --no-daemon
```

See [producer](https://github.com/VeloraMCDev/minecraft-integrations/blob/410bad40189c3af73d24f7cf37be57a4a86f14b9/producer) for implementation and generated build outputs. Maven publication is disabled during migration; no released plugin/mod download is advertised by this checkpoint.

## Integrate a map provider

Applications install a reviewed local provider implementing the SDK’s versioned factory interface. Use the API class loader for discovery and preserve one shared API class identity. No manifest-triggered remote code loading is used. Missing providers must be handled by the application’s optional-contribution path.

## Compatibility

The existing net.scopenet.worldmap namespace, renderer version, cache locations, dimensions, upload wire format and scheduling/retry behavior are retained. Display branding changes do not rename saved protocol IDs or permissions.

## Remaining loader work

Paper, Fabric, Forge/NeoForge adapters; generic server admission and companion composition; loader/provider lifecycle acceptance; and complete public/private plugin release packaging remain pending. This repository currently provides the map library boundary, not the complete replacement integration distribution.

Gradle wrapper/distribution notices are retained in [gradle](https://github.com/VeloraMCDev/minecraft-integrations/blob/410bad40189c3af73d24f7cf37be57a4a86f14b9/gradle).

## Migration and provenance

This is an active migration from the private SCOPENET-MC application. Frozen source revision: `57daa92cb6cb4027982d06bfa1c692ab9edb88ff`. Library/segment availability described above is distinct from complete feature-parity, upgrade and deployment acceptance. The current report records the actual validation checkpoint and remaining work.

## License and brand

Platform code uses the [MIT license](https://github.com/VeloraMCDev/minecraft-integrations/blob/410bad40189c3af73d24f7cf37be57a4a86f14b9/LICENSE). See [CONTRIBUTING.md](https://github.com/VeloraMCDev/minecraft-integrations/blob/410bad40189c3af73d24f7cf37be57a4a86f14b9/CONTRIBUTING.md) for contribution expectations. Official Velora artwork and trademarks are excluded from MIT; no separate artwork reuse grant is provided. Banner provenance is in [.github/assets](https://github.com/VeloraMCDev/minecraft-integrations/blob/410bad40189c3af73d24f7cf37be57a4a86f14b9/.github/assets/README.md). Repository preparation does not change visibility or publish packages.

Bundled third-party build bootstrap files retain their upstream licenses. See [artifact provenance](https://github.com/VeloraMCDev/minecraft-integrations/blob/410bad40189c3af73d24f7cf37be57a4a86f14b9/PUBLIC_ARTIFACTS.json) and [Gradle wrapper attribution](https://github.com/VeloraMCDev/minecraft-integrations/blob/410bad40189c3af73d24f7cf37be57a4a86f14b9/third-party/gradle-wrapper/README.md); repository MIT terms do not replace them.

## Explore the public platform

[Infra](https://github.com/VeloraMCDev/infra) · [Docs](https://github.com/VeloraMCDev/docs) · [SDK](https://github.com/VeloraMCDev/sdk) · [Authentication](https://github.com/VeloraMCDev/authentication) · [Minecraft Integrations](https://github.com/VeloraMCDev/minecraft-integrations) · [Panel](https://github.com/VeloraMCDev/panel) · [Launcher](https://github.com/VeloraMCDev/launcher)
