# Velora

The canonical public monorepo for the complete Velora product, hosted at
`VeloraMCDev/velora-launcher`. Each segment has its own build and
README; services retain independent deployment lifecycles. Start with
[the boundaries](docs/BOUNDARIES.md) and [the plan](MONOREPO.md).

Public-release cleanup and the automatic CI policy are documented in
[publication preparation](docs/security/PUBLICATION_PREPARATION.md).

| Segment | Entry point |
|---|---|
| Authentication and Rust libraries | [crates](crates/README.md) |
| Panel backend, sites, deployment UI and mobile | [panel](panel/README.md) |
| Desktop launcher | [launcher](launcher/README.md) |
| Minecraft plugins/mods | [integrations](integrations/README.md) |
| SDK and reusable/gameplay packages | [packages](packages/README.md) |
| Browser client and frontend modules | [shared](shared/README.md) |
| Java gameplay | [java](java/README.md) |
| Experience UI | [ui](ui/README.md) |
| Cloudflare control plane and native agent | [infra](infra/README.md) |
| Guides and documentation site | [docs](docs/README.md) |
| Frozen source and provenance | [migration](migration/README.md) |

## Local validation

Use Rust 1.98.1, Node 24 and each Java segment's supported Gradle/JDK toolchain.

```sh
node scripts/check-boundaries.mjs
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 --workspace --exclude scopenet-launcher --no-fail-fast
cd panel/web
npm ci --no-audit --no-fund
npm run check
npm run check:runes
npm run build
```

On PowerShell set `$env:CARGO_PROFILE_TEST_DEBUG='0'` before Cargo. Native desktop
packaging and loader-specific Java builds have additional platform requirements.
Use synthetic temporary stores; never open live data with a second writer.

## Deployment and cost

Cloudflare hosts the deployment authority and selected static sites. The complete
Rust/SQLite Panel and Minecraft workloads currently need native hosting. See
[deployment targets](docs/DEPLOYMENT_TARGETS.md) for domains and remaining gates.
Libraries and agents do not each require a domain or separate server.

Source CI uses a standard Ubuntu runner on public pull requests and main pushes.
Deprecated repos remain disabled. No paid plan or capacity was added.
The manual immutable Panel workflow requires
an explicit manual dispatch and tests the exact Docker image before publishing.

## Publication

The owner approved publication of the **complete product source, including gameplay**.
The public repository starts from a reviewed source snapshot. Original history,
PRs and artifacts remain in the private preservation repository.
See [publication status](docs/security/PUBLICATION_STATUS.md) and the
[audit](docs/security/PUBLICATION_AUDIT.md).
`node scripts/check-publication.mjs --public` checks the recorded publication policy
and tracked filenames; `check-secrets.mjs` separately scans tracked source.
See [licensing boundaries](LICENSE.md); MIT does not apply to every file.
