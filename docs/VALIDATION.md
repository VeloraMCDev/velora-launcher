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
| Root Rust | cargo test --locked -j 1 --workspace --exclude velora-launcher --no-fail-fast |
| Panel web | npm run check; npm run check:runes; npm run build |
| Launcher | npm test; npm run check; npm run check:runes; npm run build |
| Infra | npm test; npx --no-install tsc -p control-plane/tsconfig.json; cargo test --locked --workspace -j 1 |
| Docs | npm test; npm run check; npm run build |
| Java | Loader/toolchain checks in each integration/package README |

Run npm commands inside the matching segment after npm ci --no-audit --no-fund.
For memory-constrained Rust tests set CARGO_PROFILE_TEST_DEBUG=0; on PowerShell
use `$env:CARGO_PROFILE_TEST_DEBUG='0'`.

For the launcher/domain/contract source work, a focused root command is:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher -p velora-launcher-core -p velora-platform-utils@0.5.0 -p velora-shared@0.5.0
CARGO_PROFILE_DEV_DEBUG=0 cargo check --locked -j 1 -p velora-panel
```

For Panel library regressions, run `CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked
-j 1 -p velora-panel --lib` separately. Panel also retains a legacy utility
dependency; qualifying the launcher utility version avoids ambiguous package
selection. Separate runs avoid unnecessary feature-unified artifacts on
disk-constrained hosts. The scoped sweep is not a full workspace/native release
matrix; see `reports/final-local-sweep.md` in the repository root for its limits.

Launcher `npm test` includes the captured 124-command IPC surface, production-store
stale-event/context/settings fixtures and common Rust/TypeScript wire fixtures.
These are synthetic source contracts, not native webview, OAuth, game/loader,
keystore or installed-data upgrade acceptance. The Panel check validates producer
module integration without running a server or opening its live database.

The core's normal library suite includes a small synthetic preparation-profile
fixture calling actual sync with temporary instance files and loopback HTTP.
The optional timing run is explicit-only:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher-core --lib sync::performance_tests::controlled_preparation_profile -- --ignored --exact --nocapture
```

It emits a `SYNC_PROFILE` JSON record with five cold-logical-instance, warm,
deep-verification and one-file-repair samples. This does not run Java or a game,
use installed/player data, control OS caches, establish native performance or
assert timing thresholds. Helper probes are not a decomposition of sync stages.
Scope and baseline results are recorded in `reports/preparation-path-profile.md`.

Focused synthetic recovery/policy checks from the repository root:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher-core --lib sync::
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher-core --lib content_cache::
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher --lib game::manifest_cache::
```

Recovery fixtures create only temporary synthetic stores and child test processes.
The isolated cache is Unix-only; non-Unix use is explicitly unsupported. These
checks do not certify native keystores, installed-store upgrades, supported-OS
recovery or power-loss behavior. Current scope and performance tradeoffs are in
`reports/recovery-progress.md` in the repository root.
The sync suite includes batched-intent pending/partial/committed process exits and
fail-preserved changed evidence. `reports/sync-batch-optimization.md` records the
matched descriptive timing run; it does not assert native speed or timing thresholds.
Actual-method rollback reverse/pop/persist exits, execution guard mutations and
exclusive journal-write failure are documented in `reports/sync-fault-coverage.md`.
Their instrumentation is test-only; they do not simulate every I/O failure or power loss.

For rendered frontend-domain checks, run the launcher dev server with Node 24 and
open `/tests/domain-browser.html?mock=main` on that local server. This test-only
entry supplies a synthetic IPC facade before loading the production app/store;
it does not use a native launcher or real Panel account. Do not package it as a
production entry or infer native acceptance from its browser state/DOM checks.

The separate guarded native harness is documented in `launcher/README.md`.
Its `native-acceptance` Cargo feature is opt-in/debug-only and refuses to run
without a preseeded private marked temporary store. It bypasses neither native
release gates nor credential-store acceptance: the fixture deliberately uses
its own existing fallback key and never asks for the installed keychain identity.
Run guard tests with `cargo test --locked -j1 -p velora-launcher --lib --features
native-acceptance native_acceptance`. Default builds and default IPC stay unchanged.
Never enable this feature in packaging/release commands.

## GitHub Actions

The root .github/workflows/ci.yml runs automatically on public pull requests and main pushes.
It uses read-only permissions, pinned actions, one bounded Ubuntu runner, no
deployment secrets and no artifact upload. Packaging, native application acceptance
and release workflows remain manual to keep compute/storage costs low.

The root deployment-baseline, release-probe and panel-release workflows are manual.
Their registration jobs require successful artifact validation, main, a repository
opt-in variable and an exact immutable workflow policy in the control plane.
Workflow templates inside Infra remain historical references.
No local test substitutes for exact-image Docker acceptance, native desktop/game
upgrade testing or production rollout/restore validation.
