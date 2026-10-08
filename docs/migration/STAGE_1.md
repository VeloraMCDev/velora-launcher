# Stage 1 — foundations and initial public contracts

Authorization: user requested step 1 on 2026-10-04, confirmed repository URLs for
`authentication` and `infra`, and selected MIT for seven public platform repositories.
Logical plan labels `auth` and `infras` map to these actual names.
All eight destinations were empty when checked. Work is on
`codex/stage-1-foundation`; the monorepo tracker branch is
`codex/velora-stage-1-foundation`.

This is a historical Stage 1 report. The complete migration authorization and
current repository/merge state are tracked in [CURRENT_PROGRESS.md](CURRENT_PROGRESS.md)
and [MERGE_CHECKPOINT.md](MERGE_CHECKPOINT.md); the original resume order below is
retained as historical evidence.

## Delivered

- Neutral Velora repository README, migration instructions and ignore rules in all eight.
- MIT and contribution guidance in the seven platform repositories; no license grant
  or gameplay import in experiences. Visibility has not been changed or verified.
- Initial independent Rust platform contract subset in sdk, with provenance and
  synthetic legacy serializer compatibility fixtures; no private package imports.
- Complete hashed source inventory in infra: `57daa92cb6cb4027982d06bfa1c692ab9edb88ff`. Final split decisions,
  final paths and archive licenses remain review tasks, not completed migration.
- Plans and resume instructions in docs. Running application code is unchanged.

## Compatibility and privacy

No production data, game assets, gameplay implementations, historical binaries,
installer identities, runtime IDs, data paths or volumes have been moved or renamed.
API v1 wire names and offline UUIDs stay stable. Only the new SDK's default display
brand and generic empty experience defaults change; consumers have not adopted it.
Adoption must migrate old implicit SMP configuration into explicit private profiles.

User supplied a galaxy V emblem and Velora graffiti wordmark in chat. Their original
files are not available in this cloud checkout. Asset intake/checksums, derivatives,
trademark terms and palette approval remain BRAND-01–03; no substitute art was made.

## Resume order

1. Review platform contract disclosure and extension/auth/codegen decisions
   (GOV-04–07), repo visibility and remaining package/domain/identifier policies.
2. Complete BASE-02 mixed-file disposition and BASE-03–07 API/data/upgrade fixtures.
3. Expand SDK contracts after canonical schema and compatibility decisions; test
   public builds with no experiences access before extracting panel/auth/launcher/Java.
4. Extract applications in tracker order, retain authorized private behavior, then
   implement host-neutral Compose/native/control-panel deployments and upgrades.

Current branches are review checkpoints, not usable replacement deployments.

## Validation evidence

Verified in this cloud checkout with Rust 1.99.0, Cargo lockfile and four build jobs:

```sh
cargo +1.99.0 fmt --all -- --check
cargo +1.99.0 test --workspace --locked --offline
cargo +1.99.0 clippy --workspace --all-targets --locked --offline -- -D warnings
cargo +1.99.0 package --allow-dirty --offline
```

Results: six tests passed; formatting and clippy passed. Cargo packaged the crate
and verified a build from its unpacked archive. Cargo metadata contains one local
workspace package and registry dependencies only, with no sibling/private dependency.
All 796 inventory SHA-256 values were independently compared with the frozen Git tree.
Seven platform LICENSE files are MIT; experiences contains only neutral foundation
documentation/configuration, no gameplay or MIT license.

Application/end-to-end/database/Compose upgrade checks were not run for this stage;
application code has not changed. CI workflow was added but has not run on GitHub.
API requests to api.github.com returned 403, so visibility and PR creation could
not be verified. Empty target repositories have no separate base branch for PRs.
Pushes were attempted for all eight targets and rejected due to unavailable Git
authentication; using the existing GitHub credential helper also returned HTTP 401.
These branches exist locally only. The monorepo remote accepts writes, so all eight
foundation commits are preserved as recoverable patches in `migration/stage-1` on
`codex/velora-stage-1-foundation`. No Velora branch was published.
Enable the cloud GitHub connection's write access to VeloraMCDev before retrying.
Establishing base branches and publishing repositories remains a later action.

Reusable library workflow instructions were saved to the environment configuration
draft's `start_skill`. This does not publish the environment or verify restoration
of the eight local-only checkouts. Review/save and publish the draft in environment
settings to activate it; the monorepo patches provide a recoverable remote copy.
