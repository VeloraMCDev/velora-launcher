# Local consolidation validation — 2026-10-08

GitHub Actions was disabled across the eight Velora repositories before pushes
and merges. These are local results, not successful hosted CI claims.

Publication preparation subsequently re-enabled Actions for the canonical repo.
The new Source CI job skips private repositories and runs after publication;
the four existing workflows remain manual. Current-tree credential and filename
checks pass. No public visibility change or shared-history rewrite has occurred.
The cleanup follow-up passed all 32 Infra tests after limiting concurrent test
files to two, Worker TypeScript, three documentation tests/link checks/build,
all five root workflow policies, Rust boundaries and compatibility identities.
The fresh tracked-source scan accepted nine exact reviewed findings and found
zero unreviewed matches. Runtime Rust and desktop builds were not repeated;
runtime source and compatibility IDs are unchanged.

| Area | Result |
|---|---|
| Root Rust workspace | `cargo test --locked -j 1 --workspace --exclude scopenet-launcher --no-fail-fast` passed; seven external-network tests remain intentionally ignored |
| Health endpoint | All three new readiness/identity/legacy-liveness tests passed |
| Native application acceptance | Fresh synthetic login/API/SPA, restart credential continuity, cold backup and separate restore passed |
| Panel web | Type/runes checks and production build passed; 16 existing Svelte warnings |
| Launcher frontend | Type/runes checks and production build passed, zero diagnostics |
| Private UI | Typecheck and three behavior tests passed after canonical SDK import |
| Reusable login UI | Typecheck and seven behavior/provenance/rendering tests passed |
| SDK Rust contracts | Four unit and five compatibility tests passed with standalone lockfile |
| Pack utilities | Four unit tests passed with standalone lockfile |
| Java client/API/map producer | Standalone Gradle tests/builds passed, including provider compatibility |
| Private Java gameplay | Gradle build passed with the canonical SDK source directory |
| Default Minecraft integrations | API/common/Paper Gradle build passed; existing deprecation notices remain |
| Infra | 30 Node tests passed, plus the release-generator/metadata-contract integration test; strict Worker TypeScript passed |
| Native agent | Three Windows Rust tests passed in the independent Infra workspace |
| Documentation | Three tests, local link validation and static build passed; artifact source is the canonical repo |
| Workflow policy | All four root workflows passed the pinning/permissions/timeout guard and are manual-only |
| Boundaries/identity | Cargo local dependency traversal and installed-client identity checks passed |
| Publication | Intentionally BLOCKED; see the audit |

The native desktop packaging/signing matrix, each opt-in loader/version build,
Linux agent installer/permissions rehearsal, and Docker image build/execution were
not rerun here. Docker is unavailable on this Windows machine. Immutable image
acceptance and live application deployment remain release gates. No live data was
opened, no infrastructure was provisioned, and no application cutover was performed.
