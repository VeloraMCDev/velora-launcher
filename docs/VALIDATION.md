# Validation

Use Node 24 and Rust 1.98.1. Each segment retains its reproducible lockfiles.
Run checks relevant to the change; do not open live stores for tests.

## Source checks

From the repository root:

```sh
node scripts/check-boundaries.mjs
node infra/scripts/check-workflows.mjs .
node scripts/check-publication.mjs --public --checkout
node scripts/check-secrets.mjs
node docs/scripts/check-links.mjs .
```

The credential scanner requires Gitleaks 8.30.1 on PATH or the GITLEAKS environment
variable pointing to that executable. It scans tracked files, so stage additions
before running it. Exact synthetic/provenance findings are documented in
[security](security/README.md); new findings must be reviewed.

## Segment checks

| Segment | Checks |
|---|---|
| Root Rust | cargo test --locked -j 1 --workspace --exclude scopenet-launcher --no-fail-fast |
| Panel web | npm run check; npm run check:runes; npm run build |
| Launcher | npm run check; npm run check:runes; npm run build |
| Infra | npm test; npx --no-install tsc -p control-plane/tsconfig.json; cargo test --locked --workspace -j 1 |
| Docs | npm test; npm run check; npm run build |
| Java | Loader/toolchain checks in each integration/package README |

Run npm commands inside the matching segment after npm ci --no-audit --no-fund.
For memory-constrained Rust tests set CARGO_PROFILE_TEST_DEBUG=0; on PowerShell
use `$env:CARGO_PROFILE_TEST_DEBUG='0'`.

## GitHub Actions

The root .github/workflows/ci.yml runs automatically on public pull requests and main pushes.
It uses read-only permissions, pinned actions, one bounded Ubuntu runner, no
deployment secrets and no artifact upload. Packaging, native application acceptance
and release workflows remain manual to keep compute/storage costs low.

Workflow templates inside Infra are references, not active monorepo workflows.
They require canonical source-policy and working-directory review before activation.
No local test substitutes for exact-image Docker acceptance, native desktop/game
upgrade testing or production rollout/restore validation.
