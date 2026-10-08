# Deployment infrastructure

Independent deployment authority inside the Velora monorepo: Cloudflare control plane, D1/R2/Workflows, native outbound agent, service/artifact contracts and recovery recipes. It retains its own npm/Cargo lockfiles and operational lifecycle.

From this directory:

```sh
npm ci --no-audit --no-fund
npm test
npx --no-install tsc -p control-plane/tsconfig.json
cargo test --locked --workspace -j 1
```

The Node suite uses synthetic local Worker/D1 stores. Live configuration and private keys stay in ignored operator state. Never substitute a synthetic test for Production acceptance. Root Actions is disabled; nested .github/workflows files are retained reference and do not execute automatically in this monorepo.

Start with [the control plane](control-plane/README.md), [the native agent](agent/README.md), [implementation evidence](deployment/IMPLEMENTATION.md) and [current target assignments](../docs/DEPLOYMENT_TARGETS.md). Legacy recipes/catalogs refer to the former owner repos and remain historical; canonical candidate repository/workflow policies must be registered before release.

[Historical introduction](HISTORICAL_README.md) preserves previous evidence and recipes. [Publication audit](../docs/security/PUBLICATION_AUDIT.md) applies to this segment too.
