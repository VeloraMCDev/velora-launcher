> Historical owner documentation from `VeloraMCDev/authentication` at `5162abd28300300667ea2b029fc63551cb966977`. Current segment instructions are in the monorepo READMEs.

<p align="center">
  <img src=".github/assets/banner.png" width="1200" alt="Velora Authentication — One identity across every experience" />
</p>

# Velora Authentication

[![CI](https://github.com/VeloraMCDev/authentication/actions/workflows/core.yml/badge.svg?branch=scopedd%2Fvelora-migration)](https://github.com/VeloraMCDev/authentication/actions/workflows/core.yml) [![Code: MIT](https://img.shields.io/badge/Code-MIT-8b5cf6?style=flat-square)](LICENSE) ![Status: Game auth runtime](https://img.shields.io/badge/Status-Game%20auth%20runtime-6d28d9?style=flat-square) ![Stack: Rust · Axum · SQLite](https://img.shields.io/badge/Stack-Rust%20%C2%B7%20Axum%20%C2%B7%20SQLite-334155?style=flat-square)

Velora’s identity authority: account credentials, launcher sessions, game authentication and player cosmetics. Existing account identities and signing assets remain compatible as ownership moves out of the legacy host.

**Velora owns the platform. Each instance owns the experience.**

[Migration progress](https://github.com/VeloraMCDev/docs/blob/scopedd/velora-migration/migration/CURRENT_PROGRESS.md) · [Execution plan](https://github.com/VeloraMCDev/docs/blob/scopedd/velora-migration/migration/VELORA_EXECUTION_PLAN.md) · [Checks](https://github.com/VeloraMCDev/authentication/actions/workflows/core.yml)

## What lives here

- **[velora-auth-core](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/crates/core)** — Argon2 password rules, legacy HS256 claims, RSA signing, live admission and shared login throttling.
- **Identity storage** — account/group queries, UUID and email/name lookups, transaction-bound identity mutations and session revocation.
- **[Owned persistence](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/SCHEMA.md)** — transactional fresh-store initialization and authority-only checkpoint import, with preserved identities and explicit rejection of mixed-store initialization.
- **Game authority** — Yggdrasil tokens, join sessions, texture/cape policy, PNG normalization, head rendering and Minecraft player certificates.
- **[velora-auth-http](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/crates/http)** — Yggdrasil routes, launcher login/registration, password recovery, Discord linking/login, player account workflows and administrator identity/group/cosmetic operations.
- **[Account HTTP composition](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/ACCOUNTS_HTTP.md)** — twelve existing API v1 method/path pairs with mandatory live policy, audit, presence/transaction and email ports; explicit service-library composition keeps the CLI cutover gated.
- **Host ports** — explicit runtime policy, branding/origin, trusted client IP, provider exchange, email delivery, online presence and transactional audit/launcher cleanup.
- **[Public SDK snapshot](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/vendor/platform-contracts)** — pinned wire declarations verified without private Git access.
- **[Offline tools](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/crates/tools)** — preserve byte-exact closed checkpoints, initialize/import stores and carry verified signing/texture assets, with overwrite refusal and protected integrity manifests.
- **Startup composition** — preserve signing bytes on restart, refuse silent replacement of damaged material and arbitrate first-admin creation without resetting existing accounts.
- **[Standalone configuration](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/CONFIGURATION.md)** — typed operator origin/storage/proxy/policy settings and exact secret-file loading, with restart-safe bootstrap credentials.
- **[velora-auth-service](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/crates/service)** — runnable game-authentication process, owned-store startup, protected bootstrap, live health/readiness and graceful shutdown. See [run/configuration guidance](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/SERVICE.md).
- **[Launcher admission](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/LAUNCHER_SESSIONS.md)** — IP/session verification, retention and transactional revocation adapters; Panel retains activity observations.

## Develop

Use Rust 1.98.1 with rustfmt and clippy. Tests create synthetic accounts, stores, keys and provider responses; they do not send real Discord or email requests.

```sh
node scripts/verify-sdk-snapshot.mjs vendor/platform-contracts
cargo +1.98.1 fmt --all -- --check
cargo +1.98.1 test --workspace --all-features --locked
cargo +1.98.1 clippy --workspace --all-targets --all-features --locked -- -D warnings
```

The SQLite and texture features are opt-in for core users. The HTTP library enables the features its routes need.

## Authority boundaries

JWT signature/expiry validation is followed by a live account status/auth-version lookup. Protected game routes accept game tokens. Password changes and renames revoke affected sessions; a failed transactional host audit rolls a rename back rather than leaving credentials and activity inconsistent.

Trusted-proxy IP resolution uses the socket peer and walks forwarded chains from the right. Missing/untrusted peers cannot authorize forwarded headers; malformed hops retain the legacy failure behavior. See [network provenance](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/NETWORK_PROVENANCE.json).

UUIDs, token lifetimes, key bytes, Mojang PEM formatting and Minecraft V1/V2 certificate signatures stay compatible. Password hashes and auth versions are excluded from account JSON; private key records have no automatic wire serialization.

## Runtime status

This checkout supplies independently buildable libraries and a runnable game-authentication service. The complete web/admin/provider service and compatible gateway remain in migration. The legacy application still composes its existing pools, keys and texture paths; it has not switched credential writers.

Fresh-store initialization, closed-database checkpointing, authority-only import, signing/texture file commands and protected game-service bootstrap are available. Operator quiescence/checkpoint acceptance, full restore/rollback, provider/SMTP/presence/audit composition, credential-writer cutover and gateway deployment remain pending. Host-owned private aggregation and multi-experience deletion must retain their behavior during that cutover.

## Traceability

Extraction records include [core identity](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/STORE_PROVENANCE.json), [launcher accounts](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/ACCOUNT_PROVENANCE.json), [player accounts](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/PLAYER_ACCOUNT_PROVENANCE.json), [administrator operations](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/ADMIN_PROVENANCE.json), [cosmetics](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/COSMETICS_PROVENANCE.json), [OAuth](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/OAUTH_PROVENANCE.json) and [recovery](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/RESET_PROVENANCE.json).

## Migration and provenance

This is an active migration from the private SCOPENET-MC application. Frozen source revision: `57daa92cb6cb4027982d06bfa1c692ab9edb88ff`. Library/segment availability described above is distinct from complete feature-parity, upgrade and deployment acceptance. The current report records the actual validation checkpoint and remaining work.

## License and brand

Platform code uses the [MIT license](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/LICENSE). See [CONTRIBUTING.md](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/CONTRIBUTING.md) for contribution expectations. Official Velora artwork and trademarks are excluded from MIT; no separate artwork reuse grant is provided. Banner provenance is in [.github/assets](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/.github/assets/README.md). Repository preparation does not change visibility or publish packages.

## Explore the public platform

[Infra](https://github.com/VeloraMCDev/infra) · [Docs](https://github.com/VeloraMCDev/docs) · [SDK](https://github.com/VeloraMCDev/sdk) · [Authentication](https://github.com/VeloraMCDev/authentication) · [Minecraft Integrations](https://github.com/VeloraMCDev/minecraft-integrations) · [Panel](https://github.com/VeloraMCDev/panel) · [Launcher](https://github.com/VeloraMCDev/launcher)
