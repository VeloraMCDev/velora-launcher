# Revitalization baseline — October 12, 2026

## Provenance and scope

- Baseline HEAD: `4f639266a515c3cb05769b055a4772c4dcb89cb8` (PR #29).
- Clean local `main` fast-forwarded 11 commits from `31d3ced` after fetching the canonical origin. No divergent commits, stash, reset, merge commit, or worktree.
- Initial work began October 11, 2026, America/New_York; evidence collection continued October 12. Temporary command logs use `/tmp/velora-revitalization-20261011/`.
- Read `AGENTS.md`, `MONOREPO.md`, `docs/BOUNDARIES.md`, `docs/VALIDATION.md`, `docs/DEPLOYMENT_TARGETS.md` and the attached master plan. This batch covers its initial audit/tests/design tasks only, not approval to execute later phases.
- Machine: macOS Darwin arm64; Rust/Cargo 1.98.1. Rustup installed the repository-pinned missing toolchain. Default Node 25.6.1; checks repeated with isolated npm-cache Node 24.21.0. No project dependency/lockfile upgrades.
- `gh pr list` and `gh issue list` returned no open PRs/issues at inspection. No remote write, PR, merge, release, deployment or live-store access.

## Package map

The root Cargo workspace has 18 members: launcher/core/shared/platform utilities, authentication, reusable Panel libraries, private gameplay/storage, and the complete Panel server. Infra is an excluded independent workspace; the standalone Rust contract/utilities packages are excluded too. Launcher, Panel web, Operations, Docs and Java integrations retain their independent lockfiles and release lifecycles. See `architecture-map.md` for flow ownership and `compatibility-matrix.md` for unverified native surfaces.

## Validation ledger

| Check | Result / limitation |
| --- | --- |
| Launcher `npm ci --no-audit --no-fund` | Pass; existing lockfile unchanged |
| Launcher `npm run check` | Pass, zero errors/warnings; default Node and prescribed Node 24 |
| Launcher `npm run check:runes` | Pass on both Node versions |
| Launcher `npm run build` | Pass; existing >500 kB chunk warning; see performance baseline |
| `node scripts/check-boundaries.mjs` | Pass; includes identity check |
| `node scripts/verify-identity.mjs` | Pass |
| `node infra/scripts/check-workflows.mjs .` | Pass after Infra `npm ci`; initial missing-module environment failure resolved |
| `node scripts/check-publication.mjs --public --checkout` | Pass / REVIEWED |
| `node scripts/check-secrets.mjs` | Pass with checksum-verified Gitleaks 8.30.1; six reviewed findings, zero unreviewed; tracked files only |
| `node docs/scripts/check-links.mjs .` | Pass after report additions: 123 Markdown files, 101 local links |
| Signing + mobile configuration Node tests | Nine pass, zero fail |
| Infra `npm test` | 28 pass, zero fail on Node 24 |
| Infra TypeScript | Pass under Node 24 using installed `typescript/bin/tsc`; initial nested `npx` tool-resolution failure resolved without source changes |
| Shared capability/presentation/navigation tests | 12 pass on Node 24; no frontend behavior changes |
| Core Rust `CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher-core` | Pass: 36 library tests, five ignored known-gap contracts; seven existing online tests ignored |
| Native host Rust `CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher` | Pass: 22 tests, three ignored known-gap contracts; no default test failures |

Final fetch reconfirmed `HEAD...origin/main` divergence `0 0`. Gitleaks separately scanned untracked reports with no findings; tracked-file scan does not cover untracked additions. `git diff --check` passes. Normalized production prefixes before the test modules match HEAD in all four edited Rust files; existing lockfiles and production behavior remain unchanged.

Rust builds ran while test-only workers were adding regressions, so their totals are **working-tree validation**, not a certified pristine-HEAD test count. Existing remote/download integration tests are ignored by default; that is not native install acceptance. The full multi-segment CI workflow was not reproduced locally: Panel/Operations/Docs builds, broad Rust workspace, Infra Rust and Java are separate pending checks.

## Initial findings and gates

Confirmed hotspots supersede the plan's estimated counts: baseline launcher commands 1,859 lines, Guilds 1,243, Social 1,191, Settings 874. Line count includes a trailing split element and is not complexity or shipped-byte attribution. Existing credential-key failure protections and some sync preservation tests already exist; do not rebuild them from assumptions. Operation identity and transactional sync remain primary risks. Gate 0 is pending human acceptance of the evidence and outstanding checks. Gate 1 requires real command/event tests and independent data-safety review before structural implementation.
