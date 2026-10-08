> Historical owner documentation from `VeloraMCDev/panel` at `a5ebb0e81c653a22095893dc71e7e400e5b0e6cc`. Current segment instructions are in the monorepo READMEs.

<p align="center">
  <img src=".github/assets/banner.png" width="1200" alt="Velora Panel — The control plane for your network" />
</p>

# Velora Panel

[![CI](https://github.com/VeloraMCDev/panel/actions/workflows/activity.yml/badge.svg?branch=scopedd%2Fvelora-migration)](https://github.com/VeloraMCDev/panel/actions/workflows/activity.yml) [![Code: MIT](https://img.shields.io/badge/Code-MIT-8b5cf6?style=flat-square)](LICENSE) ![Status: Gateway and libraries](https://img.shields.io/badge/Status-Gateway%20and%20libraries-6d28d9?style=flat-square) ![Stack: Rust · Axum · SQLite / Svelte target](https://img.shields.io/badge/Stack-Rust%20%C2%B7%20Axum%20%C2%B7%20SQLite%20%2F%20Svelte%20target-334155?style=flat-square)

Velora’s operator control plane: instance distribution, platform settings, server credentials, public downloads and the administration interface. Its streaming gateway, activity-reporting and communications libraries build independently; the complete backend and interface remain in migration.

**Velora owns the platform. Each instance owns the experience.**

[Migration progress](https://github.com/VeloraMCDev/docs/blob/scopedd/velora-migration/migration/CURRENT_PROGRESS.md) · [Execution plan](https://github.com/VeloraMCDev/docs/blob/scopedd/velora-migration/migration/VELORA_EXECUTION_PLAN.md) · [Checks](https://github.com/VeloraMCDev/panel/actions/workflows/activity.yml)

## Planned platform responsibilities

- **Instances and distribution** — versions, loaders, files, visibility, access groups and clean-update revisions.
- **Operator administration** — branding, platform settings, server registration and scoped capabilities.
- **Compatible gateway** — stable launcher/API routes, Authentication forwarding, callbacks and signed/public asset origins.
- **Generic interface host** — platform navigation and reviewed local extension contributions.
- **Lifecycle coordination** — activity/reporting ports and retryable account cleanup across independently owned stores.

## What this checkout contains

**[Development deployment screen](https://github.com/VeloraMCDev/panel/blob/a5ebb0e81c653a22095893dc71e7e400e5b0e6cc/deployment-ui/README.md)** provides a separate,
dependency-free operator view for repositories, candidates, agent health, one-use
enrollment and revocation. It packages source-stamped static assets independently;
live hosting and Hermes enrollment remain pending.

**[Instance metadata library](https://github.com/VeloraMCDev/panel/blob/a5ebb0e81c653a22095893dc71e7e400e5b0e6cc/INSTANCES.md)** supplies neutral instance/file records,
pool-only queries, registry/file mutations, transactional pack replacement, revision
updates and reserved-name generation. Hosts retain live
authorization, private experience projections and existing database/schema ownership.

**[Settings library](https://github.com/VeloraMCDev/panel/blob/a5ebb0e81c653a22095893dc71e7e400e5b0e6cc/SETTINGS.md)** supplies generic auth-model settings, original
validation/defaults and pool-only KV operations. Hosts retain credential masking,
scopes, private branding and live authorization.

**[Distribution models](https://github.com/VeloraMCDev/panel/blob/a5ebb0e81c653a22095893dc71e7e400e5b0e6cc/DISTRIBUTION.md)** supplies neutral download/FAQ/theme
records and installer/repository/version/display-name helpers. Private landing
presets, stored artifact identities and live upload/download routes stay host-owned.

**[Login UI](https://github.com/VeloraMCDev/panel/blob/a5ebb0e81c653a22095893dc71e7e400e5b0e6cc/ui/README.md)** independently packages the original sign-in,
registration/approval, recovery/reset and Discord polling page. The host supplies
the typed SDK client, token lifecycle callback and shared theme styles.

**[Pack import pipeline](https://github.com/VeloraMCDev/panel/blob/a5ebb0e81c653a22095893dc71e7e400e5b0e6cc/PACKS.md)** independently supports Modrinth, CurseForge and
plain zip imports, provider downloads/checksums, override precedence, loader versions
and file cleanup through supplied client/path/pool/key/clock ports.

**[Activity library](https://github.com/VeloraMCDev/panel/blob/a5ebb0e81c653a22095893dc71e7e400e5b0e6cc/crates/activity)** supplies metadata writes and platform/server report projections. It preserves response fields, nullable records, unknown legacy kinds, original IDs, filtering, Unicode detail sanitization and pagination. It consumes supplied pools; callers retain live authorization, verified identity, timestamps and domain projections. See [boundaries](https://github.com/VeloraMCDev/panel/blob/a5ebb0e81c653a22095893dc71e7e400e5b0e6cc/ACTIVITY.md) and [extraction provenance](https://github.com/VeloraMCDev/panel/blob/a5ebb0e81c653a22095893dc71e7e400e5b0e6cc/ACTIVITY_PROVENANCE.json).

**[Communications library](https://github.com/VeloraMCDev/panel/blob/a5ebb0e81c653a22095893dc71e7e400e5b0e6cc/crates/communications)** supplies masked connection settings, secret-preserving updates, plain-text/HTML mail, generic branded rendering and template validation, and Discord OAuth provider transport. Callers own authorization, protected persistence, resolved placeholders, catalogs, recipient scope, callback state and account lifecycle. Synthetic MIME, frozen HTML fixtures and loopback HTTP tests preserve rendering bytes, validation rules, delivery errors and provider request ordering. See [boundaries](https://github.com/VeloraMCDev/panel/blob/a5ebb0e81c653a22095893dc71e7e400e5b0e6cc/COMMUNICATIONS.md) and [provenance](https://github.com/VeloraMCDev/panel/blob/a5ebb0e81c653a22095893dc71e7e400e5b0e6cc/COMMUNICATIONS_PROVENANCE.json).

**[HTTP gateway](https://github.com/VeloraMCDev/panel/blob/a5ebb0e81c653a22095893dc71e7e400e5b0e6cc/GATEWAY.md)** is a runnable public component with fixed game-authentication/texture forwarding, trusted proxy/origin handling, streaming bodies, health/readiness and graceful drain. Independent tests and a shared Infra real-process checker cover routing, outages, fresh bootstrap and closed checkpoint/assets restoration. It owns no database or private implementation.

The complete platform backend and Svelte application have not yet been imported here. The gateway provides an entrypoint for game-authentication composition; it does not replace the complete Panel application.

The functional application remains in the private compatibility host while mixed platform/private routes and pages are split. Publishing the mixed monorepo into this repository would disclose private gameplay; extraction proceeds through explicit SDK ports instead.

## Service boundaries

Authentication owns identity, credentials, game sessions and player cosmetics. Launcher owns installation and launch. Minecraft Integrations owns game-runtime transport/adapters. Private implementations supply their own gameplay behavior and interface contributions.

The Panel must build and boot with public components and a neutral experience configuration. Private capability/module presets are not required to use the platform. Unavailable authority or missing permissions must not grant access through cached state.

## Developer entry points

Use Rust 1.98.1 with rustfmt and clippy. Tests use generic synthetic metadata and never read a production store.

```sh
cargo +1.98.1 fmt --all -- --check
cargo +1.98.1 test --workspace --locked
cargo +1.98.1 clippy --workspace --all-targets --locked -- -D warnings
```

For the independent login component, use Node.js 24 from a fresh repository-root terminal:

```sh
cd ui
node ../scripts/verify-snapshots.mjs vendor/http
npm ci --ignore-scripts --no-audit --no-fund
npm run check
npm test
```

Read [the execution plan](https://github.com/VeloraMCDev/docs/blob/scopedd/velora-migration/migration/VELORA_EXECUTION_PLAN.md), then the [current progress report](https://github.com/VeloraMCDev/docs/blob/scopedd/velora-migration/migration/CURRENT_PROGRESS.md). Preserve existing routes, data paths, instance scopes, permissions and database behavior when moving code.

## Upcoming acceptance

Platform/private backend and page separation; Authentication service/gateway adoption; generic extension composition; fresh public builds; data/upgrade fixtures; and operator deployment documentation must pass before this repository is presented as a deployable replacement.

Contributors should attach validation evidence for each extracted boundary and keep unchecked release gates visible.

## Migration and provenance

This is an active migration from the private SCOPENET-MC application. Frozen source revision: `57daa92cb6cb4027982d06bfa1c692ab9edb88ff`. Library/segment availability described above is distinct from complete feature-parity, upgrade and deployment acceptance. The current report records the actual validation checkpoint and remaining work.

## License and brand

Platform code uses the [MIT license](https://github.com/VeloraMCDev/panel/blob/a5ebb0e81c653a22095893dc71e7e400e5b0e6cc/LICENSE). See [CONTRIBUTING.md](https://github.com/VeloraMCDev/panel/blob/a5ebb0e81c653a22095893dc71e7e400e5b0e6cc/CONTRIBUTING.md) for contribution expectations. Official Velora artwork and trademarks are excluded from MIT; no separate artwork reuse grant is provided. Banner provenance is in [.github/assets](https://github.com/VeloraMCDev/panel/blob/a5ebb0e81c653a22095893dc71e7e400e5b0e6cc/.github/assets/README.md). Repository preparation does not change visibility or publish packages.

## Explore the public platform

[Infra](https://github.com/VeloraMCDev/infra) · [Docs](https://github.com/VeloraMCDev/docs) · [SDK](https://github.com/VeloraMCDev/sdk) · [Authentication](https://github.com/VeloraMCDev/authentication) · [Minecraft Integrations](https://github.com/VeloraMCDev/minecraft-integrations) · [Panel](https://github.com/VeloraMCDev/panel) · [Launcher](https://github.com/VeloraMCDev/launcher)
