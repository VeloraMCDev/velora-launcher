# Master-plan completion checklist

October 14, 2026. The earlier conversational 10% was a rough estimate, not a
measured release-readiness claim. This ledger replaces that estimate with explicit
binary checkpoints mapped to the plan's 19 VEL items. Each item has four equally
weighted checkpoints: 76 total; 50% requires 38 verified checkpoints. This measures
engineering deliverables, not elapsed time, dollars, lines changed or production
readiness. Equal weighting is a reporting convention, not an effort estimate.

Only implemented/verified evidence earns a check. Proposals, ignored contracts,
worker assertions and unavailable native runs do not. Preexisting behavior can
earn a check only when revalidated and labeled as such. Reusing evidence is allowed
only for different explicit outcomes (for example host payloads versus frontend
ownership), not counting the same fix twice. The denominator and definitions stay
fixed during this milestone; any later scope correction must explain its effect.

| Item | Checkpoint 1 | Checkpoint 2 | Checkpoint 3 | Checkpoint 4 | Verified |
| --- | --- | --- | --- | --- | ---: |
| VEL-001 baseline | Checked-out HEAD/package/flow inventory | Quality/risk/compatibility inventories | Reproducible local build/test/bundle baseline | Native/runtime performance baseline matrix | 3/4 |
| VEL-002 operations | Owner-aware admission/mutation exclusion | Job-token cancellation with stale-owner regression | Correlated host progress/state/run/log payloads and tests | Native shutdown/background I/O/process-quiescence acceptance | 3/4 |
| VEL-003 downloads | Declared size/digest publication checks | Full-response, exclusive-temp and bounded declared-size handling | Response/body/hash/size/publication/cleanup fault regressions | No-digest ceilings/redirect policy and cancellation cleanup acceptance | 4/4 |
| VEL-004 sync | All replacements verified before current-tree mutation | Atomic state publication plus tested publication-error rollback | User-file/path/collision/corruption safety fixtures | Durable interrupted-commit recovery and restart/OS matrix | 3/4 |
| VEL-005 persistence | Credential denial/corruption preserves saved identity | Atomic settings writes and corrupt-settings preservation | Account/profile persistence and upgrade/failure fixtures | Native keystore/installer upgrade acceptance | 3/4 |
| VEL-006 host modules | Domain-bounded command handler extraction | Registered IPC names/arguments/results preserved by extraction | Extracted host handlers compile and regression suites pass | Independent scoped extraction/compatibility review | 4/4 |
| VEL-007 Panel client | Central typed client/domain response models | Structured redacted network/API failures | Local request/response contract fixtures | Independent client/cache/auth compatibility review | 4/4 |
| VEL-008 frontend domains | Isolated tested preparation-state domain | Production-store stale-response/event regression tests | Remaining auth/instance/profile/social state boundaries | Frontend render/navigation and reconnection acceptance | 4/4 |
| VEL-009 schemas | Owned endpoint/schema inventory | Shared consumer-compatible types | External response validation fixtures | Consumer migration/compatibility matrix | 4/4 |
| VEL-010 Rust boundaries | Reviewed utility/loader dependency cleanup | Shared reusable boundaries preserved in implementation | Loader compatibility regression fixtures | Independent loader/native compatibility acceptance | 3/4 |
| VEL-011 developer workflow | Documented deterministic launcher frontend test command | Frontend checks wired to bounded source CI | Local source/workflow/security/link checks after wiring | Troubleshooting/toolchain/fixture-scope documentation | 4/4 |
| VEL-012 resume/cache | Validated resume/cache contract | Isolated bounded prototype | Cold/warm/interruption benchmark evidence | Recovery/integrity/security acceptance | 3/4 |
| VEL-013 frontend performance | Comparable UI boot/chunk baseline | Evidence-led chunk/boot implementation | Comparable final chunk/boot measurements | Runtime startup/no-regression acceptance | 3/4 |
| VEL-014 preparation performance | Comparable install/Java profile | Evidence-led path optimization | Comparable final measurements | Native launch/loader no-regression acceptance | 3/4 |
| VEL-015 design system | Semantic tokens and state vocabulary | Shared accessible components | Synthetic visual/state snapshots | Keyboard/contrast/reduced-motion review | 3/4 |
| VEL-016 installation UX | Home/instances state UX | Detailed install/repair diagnostics UX | Retry/cancel/failed-state usability fixtures | Native/E2E usability acceptance | 3/4 |
| VEL-017 accessibility | Keyboard navigation/focus coverage | Small-window responsive cleanup | Settings/reduced-motion fixtures | Independent accessibility review | 4/4 |
| VEL-018 feature | Approved bounded diagnostic/health contract | Implemented feature with no speculative claims | Feature unit/integration tests | Native feature acceptance | 3/4 |
| VEL-019 final sweep | Full scoped/local regression sweep | Final independent security/data/compatibility review | Release-OS/native upgrade matrix | Rollback/restore drills and readiness decision | 1/4 |

Initial verified total was **8/76 = 10.53%**; the previous milestone was
**20/76 = 26.32%**, followed by **38/76 = 50%** and **57/76 = 75%**.
The following scoped/local sweep reached **58/76 = 76.32%**.
The subsequent rendered-domain milestone reached **59/76 = 77.63%**.
The complete-install milestone reached **61/76 = 80.26%** on October 13.
Current verified total is **62/76 = 81.58%**, after separate independent acceptance
of VEL-014 checkpoint 3 on October 14. For partially complete
rows above, the first N checkpoints are verified; the later checkpoints are open.
No definitions/denominator were changed during this milestone.

## Accepted evidence

- VEL-001 checkpoints 1–3: `2026-revitalization-baseline.md` plus associated
  inventory/compatibility/performance-baseline reports. Native baseline stays open.
- VEL-002 checkpoints 1–3: preparation/cancellation implementation and independent
  reviews, final host tests, `operation-events-implementation.md` and its review.
- VEL-003 checkpoints 1–4: earlier size/response evidence plus
  `download-policy-implementation.md` and `download-policy-review.md`: bounded
  unknown-size downloads, redirect policy and actual cancellation partial cleanup.
  Local synthetic acceptance does not establish supported-OS/crash performance.
- VEL-004 checkpoints 1–3: staged verification/state-last publication and actual
  rollback/data-preservation faults in `staged-sync-implementation.md` and review.
  Later current-host process restart/cleanup and batched-intent evidence are in
  `sync-recovery-implementation.md`, `sync-batch-optimization.md` and their separate
  reviews. The complete durable restart/OS matrix stays open; no checkpoint 4 credit.
- VEL-005 checkpoints 1–3: upstream credential preservation revalidated by host
  secrets tests, plus settings/persistence implementation/review and final host
  tests, plus `account-profile-persistence-implementation.md` and its review for
  legacy-format round trips, fail-closed account/profile writes and exact encrypted
  credential rollback. Native upgrades and durable cross-file recovery stay open.
- VEL-006 checkpoints 1–4: `host-extraction-implementation.md` and its review;
  124 IPC registrations/signatures/macro exports preserved across 16 modules.
- VEL-007 checkpoints 1–4: `panel-client-implementation.md` and its review;
  shared redacted/bounded HTTP transport and synthetic request/host adapter tests.
  Existing all-error cache fallback is not authorization-revocation acceptance.
- VEL-008 checkpoints 1–3: preparation tests plus
  `frontend-domains-implementation.md` and its review; account/context/settings
  helpers and 29 actual-store tests. Profile/social ownership is inventoried as
  existing component/page state, not a claimed rewrite. Native/render/reconnection
  acceptance was open at that snapshot. Checkpoint 4 now has actual rendered
  production Home/Settings/Stats/Market navigation and synthetic failure/reconnect
  acceptance in `frontend-domain-browser.md`. This is browser DOM/store acceptance,
  not a native bridge, real Panel reconnection or complete runtime/platform matrix.
- VEL-009 checkpoints 1–4: `api-schema-inventory.md`,
  `api-schema-implementation.md` and its review; shared Discord/config DTOs,
  producer/host/TypeScript migration and fixture matrix. Credit is bounded to
  these migrated families; remaining legacy Value endpoints are not fully modeled.
- VEL-010 checkpoints 1–3: `core-boundaries-implementation.md` and its review;
  unused direct dependency edges removed without version changes and four real
  loader merge regressions. Vendor/native loader acceptance stays open.
- VEL-011 checkpoints 1–4: launcher README, package test command, source CI wiring,
  validation docs and final local source/security/workflow/link checks. Native
  tests are not claimed to run in Source CI; hosted CI execution is not certified.

- VEL-012 checkpoints 1–3: `content-cache-contract.md`, implementation and
  `revitalization-75-review.md`; exclusive-root prototype, 19 passing cache tests
  and independently reproduced synthetic benchmark. No downloader integration,
  HTTP resume or complete security/platform acceptance is credited. Later
  Unix-only persistent reopen/process fixtures are separately reviewed in
  `content-cache-recovery.md`; checkpoint 4 remains open.
- VEL-013 checkpoints 1–3: `frontend-bundle-implementation.md`; captured comparable
  baseline, fixed lazy routes and final static closure/total measurements. This
  awards chunk evidence, not native boot-speed acceptance.
- VEL-014 checkpoints 1–2: `install-java-profile.md` and separate
  `install-profile-review.md`; matched complete Vanilla install/automatic-runtime
  fixtures plus actual JDK 21 detection/harmless JVM launch, repeated baseline,
  counted duplicate-resolution elimination and actual second-pass/presence-guard
  regressions. Reviewer independently reran four ordinary tests and the explicit
  five-sample profile. Workload/platform bounds and mixed timings remain explicit;
  no global speed claim. Checkpoints 3–4 are not automatically awarded from the
  same corroborating measurements or synthetic native archives.
- VEL-014 checkpoint 3: separate `install-final-measurements.md`, retained raw
  `.json` and `install-final-measurements-review.md`; independently reconciled
  three records, 120 full-flow samples, 15 JVM outcomes, 48 metric summaries and
  all 88 published timing values. Final-repeat regressions are disclosed; no
  general speedup or no-regression claim. Native checkpoint 4 remains open.
- VEL-015 checkpoints 1–3: shared semantic tokens/dialog/switch/status components,
  final production-component fixtures and six final DOM-matched, visually inspected
  synthetic rasters in `visual-state-validation.md`. Earlier stale captures are
  excluded. All-theme/full rendered keyboard/contrast/motion acceptance stays open.
- VEL-016 checkpoints 1–3: `installation-ux-implementation.md`, prior independent
  review and final operation-presentation/production-store fixtures. Typed retry
  and safe details do not establish native cancellation or reload recovery.
- VEL-017 checkpoints 1–4: final focus/Settings/navigation/motion fixtures,
  bounded keyboard/960-by-600 DOM checks and `accessibility-final-review.md`.
  Separate reviewer reran 28 focused tests and accepted the bounded final source
  fixes; native screen-reader/keyboard and full WCAG acceptance remain open.
- VEL-018 checkpoints 1–3: `diagnostics-contract.md`, implementation, independent
  review and nine final helper/actual-handler tests. Native feature gate stays open.
- VEL-019 checkpoint 1: `final-local-sweep.md`; final changed-source Rust/Node,
  frontend builds, source checks and synthetic native Panel restore. Not full
  monorepo, release-OS, final blanket security review or offsite restore acceptance.

Previous milestone evidence is in `milestone-validation.md` and
`fifty-percent-validation.md`; current commands/counts and remaining gates are in
`progress-validation.md` and `final-local-sweep.md`.
Quota-interrupted workers' downloader/sync edits were integrated, tested and
independently reviewed before credit; worker assertions alone earned no checks.
No percentage grants approval to commit, merge, publish, migrate stores or release.
