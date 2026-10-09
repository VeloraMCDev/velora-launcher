# Panel

server is the compatibility backend; web contains admin, player and landing apps; [operations](operations/README.md) is the dashboard for health, backups, updates and launcher release approval; mobile contains the companion shell.

## Local checks

```sh
cargo test --locked -j 1 -p scopenet-panel
cd panel/web
npm ci --no-audit --no-fund
npm run check
npm run check:runes
npm run build
```

The complete backend still composes private gameplay. Reusable panel-* crates must remain independent of private gameplay. /health supplies version, source revision and schema status; /healthz retains legacy liveness.

See the root README for ownership, deployment and publication status.
