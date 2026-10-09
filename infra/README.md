# Deployment infrastructure

Cloudflare deployment authority, native outbound agent, immutable service/artifact
contracts and recovery recipes. Infra has its own npm/Cargo workspaces and lockfiles;
sharing the repository does not couple service promotions.

| Component | Purpose |
|---|---|
| [Single host (VPS)](vps/README.md) | Production compose, backups and routing for one Docker host |
| [Control plane](control-plane/README.md) | Optional multi-host coordination with Workers, D1, R2 and Workflows |
| [Native agent](agent/README.md) | Outbound jobs on personally hosted machines and VPS hosts |
| [Targets](../docs/DEPLOYMENT_TARGETS.md) | Hosting/domain assignments and remaining gates |
| [Deployment guide](../docs/deployment/DEPLOYMENT_GUIDE.md) | Preparation, promotion and recovery |
| [Tests](tests/README.md) | Synthetic protocol and acceptance checks |

From this directory:

```sh
npm ci --no-audit --no-fund
npm test
npx --no-install tsc -p control-plane/tsconfig.json
cargo test --locked --workspace -j 1
```

Root source CI runs automatically. The root deployment-baseline, release-probe and
panel-release workflows are manual, with short artifact retention and separately
gated OIDC registration. Registration requires an exact monorepo repository ID,
main workflow path and reviewed source SHA in the control-plane registry. Nested
.github/workflows contains historical references and does not execute here.
Native acceptance requires its documented
toolchain/runtime prerequisites. Live keys and operator state stay ignored.
Runtime D1 SQL migrations are maintained service inputs, not migration archives.
