# Segment boundaries

The single source of truth is the public `VeloraMCDev/velora-launcher`. The owner approved public access
to the complete product source, including gameplay. Directory and dependency
boundaries remain in place; publication does not change package licenses.
See [publication policy](security/PUBLICATION_STATUS.md).

| Segment | Maintained locations | Allowed relationships |
|---|---|---|
| Authentication | `crates/auth-*` | SDK/platform libraries; no gameplay implementation or Panel host |
| Panel platform | `crates/panel-*`, `packages/panel-ui` | Auth and SDK/platform libraries; no gameplay implementation |
| Complete Panel application | `panel/server`, `panel/web` | Platform libraries plus the existing gameplay composition |
| Launcher | `launcher`, `crates/core` | Neutral contracts/utilities and platform APIs |
| SDK | `packages/platform-*`, `packages/rust-platform-contracts`, `packages/java-platform-client`, `packages/java-legacy-api`, `shared/http` | Neutral contracts and public registry dependencies |
| Minecraft integration | `integrations`, `packages/java-map-producer` | SDK; gameplay only through explicitly composed host integration |
| Gameplay | `packages/private-*`, `java`, `ui`, gameplay frontend modules under `shared` | Platform ports and SDK; never a dependency of reusable platform libraries |
| Deployment | `infra`, `panel/deployment-ui` | Versioned service/artifact contracts; separate build and operational lifecycle |
| Documentation | `docs` | Build independently; review the selected content before publishing |

`node scripts/check-boundaries.mjs` enforces the reusable Rust dependency boundary,
workspace isolation for Infra and installed-client identity checks. It is a
structural check, not a substitute for reviewing runtime imports or Java/TypeScript
dependency changes. Segment READMEs describe their local checks.

The existing Panel host still imports gameplay directly. Introducing an
extension interface is an explicit remaining refactor; documentation must not
claim that boundary is already complete. Persisted database IDs, paths, volume
names, protocol IDs, launcher identity and updater signing keys remain stable.

The root Cargo workspace owns the application crates. Infra keeps a nested Cargo
workspace with its own lockfile and is excluded from the root workspace. Frontend
packages retain separate npm lockfiles to keep independent, reproducible builds.
Java builds remain separated by toolchain/loader. Sharing a repository does not
couple service promotion or require every library to get its own domain.
