Current implementation work and authorization supersede the Stage 1-only snapshot below. Read [CURRENT_PROGRESS.md](CURRENT_PROGRESS.md) before using this historical tracker. Unfinished acceptance gates remain unfinished.

# Velora migration, rebrand, open-source and self-hosting task tracker

**Branch:** `codex/velora-stage-1-foundation` (monorepo),
`codex/stage-1-foundation` (eight target repositories).
**Scope:** stage 1 foundations and initial public platform contract extraction.
Running application/gameplay code has not moved or changed. Repository visibility
and production deployments have not changed.
**Implementation authorization:** user requested step 1 on 2026-10-04. This permits
the foundation work recorded in [STAGE_1.md](STAGE_1.md), not completion of unresolved
architecture or publication gates.

This is the execution checklist for Codex, Claude and human maintainers. Read
[MIGRATION_PLAN.md](MIGRATION_PLAN.md) for the source inventory, module ownership,
eight target trees, current dependencies and preservation requirements. This
tracker supersedes its earlier assumption that public products could require
private experience packages. Source paths below are historical evidence, not new
Velora names already applied to code.

## 1. Required outcome

**Velora owns the platform. Each instance owns the experience.**

| Repository | Visibility | Must build and test without experiences access? | Responsibility |
| --- | --- | --- | --- |
| `launcher` | Public/open source | Yes | Tauri desktop shell, installation/launch engine, local accounts/settings and generic experience host |
| `panel` | Public/open source | Yes | Admin/player/landing shells, control plane, instance distribution, platform/social services and extension gateway |
| `minecraft-integrations` | Public/open source | Yes | Paper/Fabric/Forge adapters, shared transport, map producer, companion and extension SPI |
| `auth` | Public/open source | Yes | Identity/credentials/groups, account lifecycle and Yggdrasil-compatible authentication |
| `sdk` | Public/open source | Yes | Public schemas, clients, host extension interfaces, generic shared UI/helpers and platform brand assets |
| `experiences` | **Private, including `shared`** | Not applicable | Shared gameplay implementation, SMP/Frontiers composition, gameplay UI/catalogs/rules/assets and authorized distributions |
| `infras` | Public/open source | Yes | Packaging, Compose/native/panel-hosting recipes, CI orchestration, compatibility releases and operational tests |
| `docs` | Public/open source | Yes | Architecture/ADRs, public API/extension/developer/operator/user documentation |

The public product must be useful by itself: a host can deploy it, create accounts
and instances, import/distribute files, configure supported integrations and let
players install/authenticate/launch. It must not demand the owner's GitHub account,
private registry token, hosted SCOPENET endpoint, private CI runner or experience
license just to boot. SMTP, Discord, CurseForge and private gameplay are optional
features with explicit setup and failure states.

Existing gameplay is not deleted to achieve that public build. Move it to private
experience modules and keep an authorized full-feature composition tested against
the original behavior. A clean public reference extension uses new synthetic
fixtures/data, not copied SMP/Frontiers rules, art, defaults or implementations.
Approve the exact public contract surface separately from private implementation.

Multiple containers are allowed. Recommended target: public panel and auth behind
one public origin; a compatible private experience service or private composed
artifacts are added explicitly. SDK/docs/infras are not containers. A decision is
still required on private backend/UI/Java composition before choosing a plugin
implementation; do not assume a stable Rust dynamic ABI or execute arbitrary
remote UI code from manifests.

### Hosting scope

"Any game server hoster" means vendor-neutral installation for supported servers
with documented capabilities, not a promise that every restricted hosting plan
permits every Java agent, mod loader, port or service. Support these deployment
routes and publish the exact requirements/limitations:

1. VPS/dedicated host: Docker Compose, panel/auth containers and configurable HTTPS.
2. Linux host without Docker: documented binary/static-web deployments and service
   units, with the same API/backup/security/configuration guarantees.
3. Hosting control panel such as Pterodactyl/Pelican: unprivileged service templates,
   supplied port allocations, persistent paths and reachable internal/external URLs.
4. Minecraft-only/shared provider: install the supported plugin/mod on the game
   host and connect it to an independently hosted Velora stack over HTTPS. Do not
   require Docker access or a shared filesystem on that game host.

Define architecture/OS/Java/loader support and measured resources. Document hosts
which disallow required JVM/authlib-injector arguments; do not claim custom
Yggdrasil game authentication works on those plans or invent Microsoft account
support that the current project does not implement.

## 2. How an implementation agent uses this tracker

1. Read both plans and the applicable repository instructions. Verify the actual
   baseline; the initial audit used `44e28ee`, and main now includes Docker fix
   `57daa92`. Record the full current revision before extraction.
2. Until `GOV-01` is complete, work only on planning/review corrections. A checkbox
   is not authorization to publish, deploy, migrate production data or delete code.
3. Pick the first uncompleted task whose dependencies and phase gate are complete.
   Never start a task blocked on a naming/license/service decision by inventing it.
4. Claim one task by adding an entry to the work log with owner/agent, start time,
   status, branch and intended scope. Use `todo`, `in_progress`, `blocked`, `review`,
   `done` or `deferred`. Keep the task checkbox unchecked until acceptance passes.
5. Make a small reviewable change. Migration source moves, rebrand renames, schema
   changes and deployment changes should have separate commits/checkpoints where
   possible. Preserve provenance and run the relevant existing/boundary tests.
6. Record the exact commit/PR, changed paths, validation commands/results and
   compatibility/data effects. Redact credentials and real player/server data.
7. Mark `[x]` only after the task's stated outcome is verified. Failure or unavailable
   tooling means `blocked`/`review`, not "done". Record unresolved questions and the
   next safe action; do not silently drop acceptance requirements.
8. At each phase boundary, update the dependency/ownership/compatibility manifests
   and get the specified review. Resume agents use that log, not conversational
   assumptions. User instructions and reviewed ADRs take precedence over proposals.

**Task format:** ID, checkbox, owning repository, prerequisite IDs, bounded work
with a concrete completion condition. Owners may create implementation subtasks
under an ID (for example `EXP-04.a`); keep the original ID and scope traceable.
`—` means no task dependency, not bypassing the global implementation gate.
Dependencies are minimum prerequisites; the phase gate also applies.

**Implementation status:** stage 1 has started. GOV-01 is complete only for the
explicitly authorized foundation scope. Other checkboxes remain unchecked unless
their full acceptance criteria pass; partial subtasks are tracked below.
**Next action:** review remaining GOV decisions and mixed-file disposition, then
expand the independently tested SDK before extracting running applications.

### Work log template

```text
Task ID / status:
Owner or agent / UTC timestamp:
Repository / branch / commit / PR:
Reviewed decisions and prerequisites:
Changes and source-to-destination inventory entries:
Validation commands, results and evidence:
Public/private disclosure and compatibility impact:
Data/rollback impact, if applicable:
Blocker or next action:
```

Maintain progress here or in a linked tracker with the same IDs. Do not store
private implementation details in a public issue/work log; keep private task
evidence in the private repository and publish only the permitted status summary.

## 3. Release gates and order

| Gate | Completion condition |
| --- | --- |
| G0: plan approved | GOV tasks resolve licenses/names/privacy/extension architecture and the user reviews the execution plan. |
| G1: baseline locked | Baseline inventory, preservation fixtures, history/artifact classification and rollback boundaries are recorded. |
| G2: contracts available | SDK builds in isolation; old/new protocol fixtures and public extension reference pass. |
| G3: independent public builds | All seven public repositories build/test with private network access/credentials unavailable. Private composition retains gameplay. |
| G4: compatible Velora identity | Rebrand matrix is implemented/tested; existing installations, IDs, permissions, channels, keys and downloads remain usable under the approved compatibility policy. |
| G5: host-ready stack | Fresh install, persistent-volume upgrade, TLS/proxy, auth, connectivity, backup/restore and supported hosting profiles pass. |
| G6: publication ready | Public history/artifacts/licenses/docs are audited; no private experience implementation or credentials leak; package/release provenance and compatibility manifest are ready. |
| G7: release accepted | Reviewed public release and private authorized composition pass end-to-end acceptance; operational rollback/support instructions are verified. |

Work order: governance/baseline -> public boundary and SDK -> auth/experience ports
and cohesive extraction -> applications/integrations -> compatible rebrand ->
packaging/self-hosting -> documentation/publication -> acceptance/cutover. Rebrand
inventory/artwork preparation and docs planning can happen early; breaking names,
database cutovers and repository visibility changes happen only at their gates.

## 4. Governance and baseline tasks

| ID | Done | Owner | Depends on | Work and acceptance |
| --- | --- | --- | --- | --- |
| GOV-01 | [x] | docs/user | — | User authorized step 1 on 2026-10-04: repository foundations, baseline/provenance and initial generic SDK contracts. Later unresolved architecture/publication gates remain in force. |
| GOV-02 | [ ] | docs/user | GOV-01 | Record the organization, eight repository URLs, public package scopes/coordinates and owned Velora domains; avoid invented reverse-DNS IDs or example URLs used as production defaults. |
| GOV-03 | [ ] | docs/user | GOV-01 | Choose open-source code license(s), copyright attribution and contributor policy for seven repositories; separately define private experience and artwork/trademark distribution terms. No implied license selection. |
| GOV-04 | [ ] | docs/user | GOV-01 | Approve the public contract/implementation boundary, including existing Java common logic, gameplay DTOs, docs/examples/test fixtures and historical binaries; shared gameplay remains private. |
| GOV-05 | [ ] | sdk/experiences | GOV-04 | Approve backend, UI and Java extension composition: public base plus independently installable private service/authorized builds; public CI/manifests resolve no private dependency. Document version/ABI/trust constraints. |
| GOV-06 | [ ] | auth/panel | GOV-01 | Resolve identity/groups/session/presence/mail/deletion authority and auth service topology using MIGRATION_PLAN D1/D3/D7/D8/D10; specify revocation/failure behavior. |
| GOV-07 | [ ] | sdk | GOV-01 | Resolve canonical schema/codegen, API compatibility and optional Bukkit binding using D4/D6; retain public Java packages/provider/class identity during transition. |
| GOV-08 | [ ] | infras | GOV-02, GOV-03 | Resolve registries, provider support, version/release policy, signing/provenance, history strategy and exact supported platform/loader matrix. |
| GOV-09 | [ ] | launcher/auth/MC | GOV-02 | Approve Velora identifier compatibility policy: local app/keyring IDs, package names, mod IDs, permission aliases, channels and public URLs; name the retirement conditions for legacy aliases. |
| GOV-10 | [ ] | docs/infras | GOV-05, GOV-06 | Review Compose/native/shared-host/control-panel support and measured-resource test plan; approve public default behavior and optional private profile. |
| BASE-01 | [ ] | infras | GOV-01 | Freeze exact source revision, including merged Docker temp fix and outstanding approved changes; record toolchains, lockfiles and known failures without changing source to hide them. |
| BASE-02 | [ ] | docs | BASE-01, GOV-04 | Produce a per-tracked-file disposition manifest: current path, final repo/path, public/private class, license, tests, source commit and extraction step; account for every file with no unexplained deletion. |
| BASE-03 | [ ] | sdk/panel | BASE-01 | Capture all registered HTTP methods/paths, auth scopes, JSON/error shapes, headers, cookies if any, public URLs, IPC commands and companion messages; compare future versions against this manifest. |
| BASE-04 | [ ] | auth/experiences | BASE-01 | Create sanitized legacy/current database and asset fixtures with users/groups, balances/XP, rewards, guilds/claims, maps, content, key/session IDs and migration markers; record expected outcomes. |
| BASE-05 | [ ] | launcher/MC | BASE-01 | Preserve launcher settings/accounts/keyring paths/game saves and server utilities/retry ledgers/companion preferences as upgrade fixtures; never commit live secrets or personal data. |
| BASE-06 | [ ] | infras | BASE-01, GOV-04 | Hash/classify every historical installer/JAR/checksum/release note and download location; decide which archives stay private and which binaries may be distributed without exposing private experiences. |
| BASE-07 | [ ] | infras | BASE-03, BASE-04, BASE-05 | Run/document original relevant tests and runtime flows per supported target; separate known failures from new regressions and capture packaging/upgrade evidence. |
| BASE-08 | [ ] | infras/docs | BASE-04, BASE-06 | Define backups, read/write cutover checkpoints and restore/rollback procedure per data/schema version; test the baseline restore before extracting persistence. |

## 5. Open-source/public-private boundary

Do not make the current monorepo public wholesale: its source/history/artifacts
contain the implementation that will belong to private experiences. Publicly
visible source cannot be made secret again by deleting it in a later commit.
Preserve the original full history privately and publish only reviewed public
extractions/history. This requirement also covers build outputs and CI logs.

| ID | Done | Owner | Depends on | Work and acceptance |
| --- | --- | --- | --- | --- |
| PUB-01 | [ ] | infras | GOV-02, GOV-03, GOV-04, BASE-02 | Prepare seven public and one private repository configurations after review; initially keep unpublished extractions private until history/contents gates pass. |
| PUB-02 | [ ] | infras/docs | PUB-01, BASE-06 | Implement reviewed history filtering or clean initial imports with provenance; inspect branches/tags/LFS/releases, not just working trees. Original archive stays private. |
| PUB-03 | [ ] | all public repos | PUB-02, GOV-03 | Add approved LICENSE/NOTICE/copyright and third-party notices; verify dependencies, generated Iconify/font/art assets and bundled authlib/JAR redistribution obligations. |
| PUB-04 | [ ] | all public repos | PUB-01 | Add contributing/build/test guidance, issue/PR templates, security contact/disclosure policy and maintainer ownership; each repo's instructions work independently. |
| PUB-05 | [ ] | infras | PUB-02 | Scan public source/history/config examples/logs/releases for credentials, private URLs/player data and private gameplay; remediate findings and record evidence before visibility changes. |
| PUB-06 | [ ] | sdk/panel/launcher/MC | GOV-05, SDK-10 | Remove mandatory private Cargo/npm/Maven/path imports from public manifests and default bundles; clean unauthenticated builds make no request to private sources/registries. |
| PUB-07 | [ ] | sdk | GOV-04, SDK-01 | Expose reviewed schemas/extension ports only; ensure generated clients/examples/maps/type declarations do not incorporate private algorithms, preset catalogs or branded experience assets. |
| PUB-08 | [ ] | infras/experiences | GOV-05, PUB-06 | Separate public CI/images/packages from private composition builds; no public workflow requires secrets granting private repository access, including fork PR checks. |
| PUB-09 | [ ] | infras/experiences | PUB-08 | Audit private artifact distribution, source maps, compiled JS/JAR/native bundles, debug symbols, SBOMs and caches; default to authorized private distribution until terms/visibility are approved. |
| PUB-10 | [ ] | docs | SDK-10, GOV-03 | Publish a synthetic reference extension and third-party developer guide under the approved public license; demonstrate public buildability without copying private experience content. |
| PUB-11 | [ ] | infras | PUB-03, PUB-05, PUB-08, PUB-09 | Verify repository/package/workflow access policies and least-required publication permissions; private experience logs/artifacts/status details stay private. |
| PUB-12 | [ ] | infras/user | PUB-04, PUB-07, PUB-10, PUB-11, TEST-01 | Review G6 evidence and publish only the seven approved repositories/packages; do not publish experiences or change original archive visibility. |

## 6. SDK and contract tasks

| ID | Done | Owner | Depends on | Work and acceptance |
| --- | --- | --- | --- | --- |
| SDK-01 | [ ] | sdk | GOV-07, BASE-03 | Inventory Rust shared types, route-local structs/JSON, both frontend types, shared board/casino/map types and Java wire/API objects; every external shape has an owner/schema. |
| SDK-02 | [ ] | sdk | SDK-01 | Establish canonical schemas, generated packages, compatibility exports and neutral errors; schema generation and cross-language fixture checks pass. |
| SDK-03 | [ ] | sdk | SDK-02 | Extract platform manifests/files/loaders/branding/updates/social/presence contracts from `crates/shared` and frontend duplicates; preserve field/default/null/UUID/version semantics. |
| SDK-04 | [ ] | sdk | SDK-02 | Extract public auth/Yggdrasil/session/texture and verified-principal contracts; no UserRow, signing secrets, password hashes, SQL pools or app state in public wire models. |
| SDK-05 | [ ] | sdk | SDK-02, GOV-04 | Package instance context/capabilities/module/widget and reviewed gameplay DTOs without private rules/default content; new public defaults are generic, while legacy stored SMP fields/config remain representable and are not silently reset. |
| SDK-06 | [ ] | sdk | SDK-02 | Define server admission/sync/receipt/action/reward/claim/map/companion/content contracts and operation IDs; old clients' messages remain interoperable. |
| SDK-07 | [ ] | sdk | SDK-03, SDK-04, SDK-05, SDK-06 | Publish Rust/TS/Java client mechanisms with explicit scoped headers, auth, timeout/error handling and version negotiation; clients work against captured API fixtures. |
| SDK-08 | [ ] | sdk | SDK-03 | Extract panel-used metadata, safe path, hash and neutral net helpers from `crates/core`; panel no longer imports install/launch implementation. |
| SDK-09 | [ ] | sdk | GOV-05, SDK-04, SDK-05, SDK-06 | Define injectable backend/report/identity/storage/delivery/game-host/page/widget ports; no interface requires a panel AppState or private implementation class. |
| SDK-10 | [ ] | sdk | SDK-09 | Build the synthetic public reference extension and empty/generic host provider; prove real host composition, error handling and missing-extension behavior without private packages. |
| SDK-11 | [ ] | sdk | SDK-03, SDK-05, SDK-09 | Extract generic map renderer/widget host and shared modal/avatar/skin/form primitives; Svelte packages have exports/types and no direct Frontiers/gameplay imports. |
| SDK-12 | [ ] | sdk/MC | GOV-07, SDK-06 | Extract neutral Java client/SPI/API and approved optional Bukkit declarations; preserve package/provider/classloader expectations and compileOnly consumer examples. |
| SDK-13 | [ ] | sdk | GOV-08, SDK-07, SDK-11, SDK-12 | Release pinned public Rust/npm/Maven packages, locks and provenance; install them in clean isolated consumers without sibling checkouts or private tokens. |
| SDK-14 | [ ] | sdk | SDK-13 | Add contract change/version policy and old/new client compatibility fixtures, including Yggdrasil errors and float money representations; incompatible changes require reviewed major/version migration. |
| SDK-15 | [ ] | sdk/infras | SDK-14, PUB-07 | Verify generated/public package contents, dependency directions, licenses and reproducibility; close G2 with recorded evidence. |

## 7. Auth extraction and storage boundary

| ID | Done | Owner | Depends on | Work and acceptance |
| --- | --- | --- | --- | --- |
| AUTH-01 | [ ] | auth | GOV-06, SDK-04, BASE-04 | Extract `auth.rs` credentials/JWT/login guard/identity/groups with owner-local models/tests; panel guards become SDK adapters. |
| AUTH-02 | [ ] | auth | AUTH-01, SDK-06 | Extract `yggdrasil/*` routes/keys/certificates and sessions; captured authenticate/refresh/join/hasJoined/profile/signature behavior passes. |
| AUTH-03 | [ ] | auth | AUTH-01, SDK-09 | Split login/register/account/admin identity/username/skin/cape/saved profile logic from public/account/admin routes; online checks and audit use explicit ports. |
| AUTH-04 | [ ] | auth/panel | AUTH-01, GOV-06 | Extract account linking/reset workflows from `connections.rs`; separate generic SMTP/provider/bulk email logic, with retries and no startup dependency cycle. |
| AUTH-05 | [ ] | auth | AUTH-02, AUTH-03, AUTH-04 | Define auth-owned migrations/config/key and texture storage plus bootstrap; preserve IDs/hashes/reserved names/groups/auth versions and key bytes. |
| AUTH-06 | [ ] | auth/panel | AUTH-05, PANEL-03 | Remove direct foreign auth SQL/ATTACH use in platform/experience paths; verify current disabled-account/group revocation checks and auth-unavailable denial behavior. |
| AUTH-07 | [ ] | auth/infras | AUTH-06, BASE-08 | Implement non-destructive import/checkpoint tool; copy/validate historical auth records and signing assets without replaying gameplay triggers or running two credential writers. |
| AUTH-08 | [ ] | auth | AUTH-07 | Add independent service entrypoint, health/readiness/config and shutdown tests; missing/invalid config produces actionable errors, no duplicate first-admin creation. |
| AUTH-09 | [ ] | auth/panel | AUTH-08, PANEL-04 | Keep compatible auth/Yggdrasil routes, callbacks, signed texture URLs and API-location indication through gateway; original credentials/sessions work under approved policy. |
| AUTH-10 | [ ] | auth/infras | AUTH-09, OPS-10 | Verify protected persistence permissions, backup/restore, key continuity and auth service upgrades/outages; no auth or production secrets committed. |

## 8. Private experiences extraction and preservation

This section is **private implementation work**. The full module-to-file mapping
is in MIGRATION_PLAN sections 2.3/2.5/6. Publish public extension contracts, not these
implementations. Current Frontiers work is a preset/widget/module snapshot, not a
complete city simulation; new simulation is outside migration scope.

| ID | Done | Owner | Depends on | Work and acceptance |
| --- | --- | --- | --- | --- |
| EXP-01 | [ ] | experiences | GOV-04, GOV-05, SDK-09, BASE-02 | Establish private backend/UI/Java packages and module registration; SDK is the only required public source contract and no panel/loader internals are imported. |
| EXP-02 | [ ] | experiences | EXP-01, BASE-04 | Preserve old migrations/triggers/import marker and original private archive; define instance migration namespaces and a private legacy gameplay upgrade tool. Public code contains no private trigger/policy implementation. |
| EXP-03 | [ ] | experiences | EXP-02 | Extract economy/ledger/market/auctions/guild-bank/orders/contracts/mailbox together; balances, escrow, operation receipts and money effects stay atomic and replay-safe. |
| EXP-04 | [ ] | experiences | EXP-03 | Extract casino backend/game variants/admin settings and shared casino UI/animation/preferences/audio; payouts/round persistence/limits match baseline. |
| EXP-05 | [ ] | experiences | EXP-02 | Extract progression/levels/stats/ranks/glyphs/bonuses/rewards/queue/delivery/LuckPerms links; preserve earned history and acknowledgements without double grants. |
| EXP-06 | [ ] | experiences | EXP-05 | Extract quests/chains/assignments/objectives/claims/achievements and private seed catalogs; all baseline trigger/completion rules and defaults survive. |
| EXP-07 | [ ] | experiences | EXP-02, EXP-03 | Extract guild membership/roles/requests/invites/relations/primary guilds/posts/claims/admin land/flags; enforce scoped ownership and preserve guild-bank linkage. |
| EXP-08 | [ ] | experiences | EXP-05 | Extract collections/cosmetics/templates/unlocks/equip rules and in-game model policies; SDK/loader ports handle public identity/world execution. |
| EXP-09 | [ ] | experiences | EXP-02 | Extract content/custom-item/resource-pack imports/safety/Blockbench/model previews/glyph assets; keep installation `packs.rs` in panel and retain framework import limitations. |
| EXP-10 | [ ] | experiences | EXP-07, SDK-11 | Extract scoped map backend/zoom/live overlays/actions/assets; signed/public pack/map URLs, dimensions and original tiles keep working. |
| EXP-11 | [ ] | experiences | EXP-01, SDK-12 | Extract Java gameplay command/utility/shop/guild/reward/chat rules and catalogs from common; loaders retain threads/world/inventory/cache/poller adapters. |
| EXP-12 | [ ] | experiences | EXP-05, EXP-07, EXP-10, EXP-11 | Extract community events, companion layouts/actions, domain tasks/notifications and gameplay Discord content; social/provider/account workflow stays public platform. |
| EXP-13 | [ ] | experiences | EXP-04, EXP-06, EXP-07, EXP-08, EXP-09, EXP-12 | Extract all gameplay admin/player/desktop pages and widgets/fixtures into host-independent packages; existing controls remain accessible in authorized compositions. |
| EXP-14 | [ ] | experiences | EXP-13, BRAND-18 | Compose SMP and existing Frontiers preset/widget/snapshot with separate branding/nav/config; do not rename operator-created identities or build a new city simulation under migration. |
| EXP-15 | [ ] | experiences/infras | EXP-14, PUB-08, PUB-09 | Release authorized private service/composed artifacts and run full baseline gameplay/isolation/import/restart tests; public base still builds/boots with none installed. |

## 9. Panel, launcher and Minecraft repository migrations

### Panel, player site and mobile

| ID | Done | Owner | Depends on | Work and acceptance |
| --- | --- | --- | --- | --- |
| PANEL-01 | [ ] | panel | SDK-13, BASE-02 | Extract server/web/icons/mobile into independent manifests/locks/tests; remove root workspace/frontend source aliases and use public package versions. |
| PANEL-02 | [ ] | panel | PANEL-01, SDK-08 | Extract instance registry/installation metadata/files/import/search/admin settings/branding/landing/publishing services; retain Modrinth/CurseForge/plain zip paths and file safety. |
| PANEL-03 | [ ] | panel | PANEL-01, SDK-09 | Split AppState/RequestState/store/db/experience into platform host and scoped extension ports; replace foreign identity SQL, module imports and mixed KV authority; per-domain storage is explicit. |
| PANEL-04 | [ ] | panel | PANEL-02, PANEL-03, AUTH-08 | Provide compatible gateway/public origin for auth, instance context, server credentials and public downloads; legacy routes/errors/header fallback remain deliberate. |
| PANEL-05 | [ ] | panel | PANEL-03, SDK-07 | Extract public social/profiles/presence/audit and report composition; no duplicate credential store or gameplay SQL joins. |
| PANEL-06 | [ ] | panel/auth | PANEL-05, AUTH-06 | Implement retryable account deletion/anonymization and scoped notifications/reporting; failed store cleanup is visible/retryable and third-party history is retained correctly. |
| PANEL-07 | [ ] | panel | PANEL-03, GOV-06 | Split generic task host and platform mail/Discord/provider administration from experience task/content contributions; absent providers/extensions do not break boot. |
| PANEL-08 | [ ] | panel | PANEL-04, SDK-11, PUB-06 | Extract platform admin/player/landing/PWA shells and instance-first navigation; generic public views work with no private pages and missing extensions have useful state. |
| PANEL-09 | [ ] | panel | PANEL-08, GOV-05 | Add reviewed page/editor/widget extension seams and private composition integration; public bundles/manifest contain no private imports. |
| PANEL-10 | [ ] | panel | PANEL-08, BRAND-13 | Extract Capacitor shell/assets/local config with operator-supplied URL and approved ID migration; preserve Android/iOS/PWA deep links and native-hosted player experience. |
| PANEL-11 | [ ] | panel | PANEL-02, PANEL-08 | Move icon-pack builder/server APIs, local demo runner and platform tests; synthetic public fixtures replace private gameplay demo content and generators need no sibling source. |
| PANEL-12 | [ ] | panel | PANEL-06, PANEL-07, PANEL-09, PANEL-10, PANEL-11 | Verify independent public build and full-feature private composition against every old panel/admin/player route/editor/action; archive moved tests only after equivalents pass. |

### Launcher and engine

| ID | Done | Owner | Depends on | Work and acceptance |
| --- | --- | --- | --- | --- |
| LAUNCH-01 | [ ] | launcher | SDK-08, SDK-13, BASE-02 | Extract Tauri shell and remaining `crates/core` engine with local Cargo/npm locks/config; engine build/test does not depend on panel source. |
| LAUNCH-02 | [ ] | launcher | LAUNCH-01 | Preserve Java/assets/libs/natives/loaders/Maven/rules/install/sync/path/options/servers_dat/system/progress modules and online tests; all actual supported loaders retain baseline behavior. |
| LAUNCH-03 | [ ] | launcher | LAUNCH-01, AUTH-09 | Adapt accounts/game auth/authlib-injector to SDK endpoints; offline/account/profile flows and online game sessions still work. |
| LAUNCH-04 | [ ] | launcher | LAUNCH-01, SDK-07 | Split large IPC registry into platform/local and generic extension adapters; preserve command signatures, selected-instance propagation and errors. |
| LAUNCH-05 | [ ] | launcher | LAUNCH-01, SDK-11 | Keep local setup/login/settings/play/log/crash/console/repair UI and replace sibling shared aliases with public packages; public generic experience views work. |
| LAUNCH-06 | [ ] | launcher | LAUNCH-04, GOV-05 | Implement reviewed private UI composition seam plus platform social/profile/notifications; no private compile/runtime prerequisite in public release. |
| LAUNCH-07 | [ ] | launcher | LAUNCH-03, BRAND-12 | Migrate local app directories/keyring/encrypted account state and companion preferences using approved ID policy; upgrades retain accounts/game files and secrets decrypt correctly. |
| LAUNCH-08 | [ ] | launcher | LAUNCH-04, BRAND-06 | Make server URL onboarding/unlocked defaults/update repository configurable without SCOPENET endpoints; imported existing host/account URLs are retained. |
| LAUNCH-09 | [ ] | launcher | LAUNCH-02, SDK-06 | Preserve telemetry/lifecycle/quick-play/companion preflight/channel/preference behavior and selected instance scopes. |
| LAUNCH-10 | [ ] | launcher | LAUNCH-05, BRAND-11 | Generate desktop icons/bundle metadata and native packages under Velora while following legacy upgrade/signing policy. |
| LAUNCH-11 | [ ] | launcher | LAUNCH-08, BRAND-16, REL-05 | Verify hosted/GitHub/Gitea update/download/version paths, immutable hashes, current URL checks and upgrade continuity; no forced central Velora service. |
| LAUNCH-12 | [ ] | launcher | LAUNCH-06, LAUNCH-07, LAUNCH-09, LAUNCH-10, LAUNCH-11 | Test independent public Windows/Linux/macOS builds and authorized experience edition, with native storage/launch/repair/update regression checks. |

### Minecraft adapters and companion

| ID | Done | Owner | Depends on | Work and acceptance |
| --- | --- | --- | --- | --- |
| MC-01 | [ ] | minecraft-integrations | SDK-12, SDK-13, BASE-02 | Extract Gradle/wrapper/descriptors/common/Paper/Fabric/Forge/mixins/client/examples; use published public SDK with separate legacy/modern toolchains. |
| MC-02 | [ ] | minecraft-integrations | MC-01, SDK-07 | Split PanelClient/Wire/Panel adapters from private claim/gameplay rules; admission/heartbeat/sync/error handling uses public protocol. |
| MC-03 | [ ] | minecraft-integrations | MC-02, SDK-09 | Add public game-host SPI/extension registration, retaining main-thread/world/inventory/entity execution and scheduling/caches in adapters; public plugin/mod starts without experiences. |
| MC-04 | [ ] | minecraft-integrations | MC-03, GOV-07 | Preserve developer API provider/event emission/class identity and optional Vault/LuckPerms/PlaceholderAPI/WorldGuard/CoreProtect/Spark detection through optional contributions. |
| MC-05 | [ ] | minecraft-integrations | MC-03 | Keep NBT/Anvil/block palettes/region sampling/map rendering/upload and versioned mixins; validate produced tiles and no private overlay/protection rules embedded. |
| MC-06 | [ ] | minecraft-integrations | MC-03, SDK-06 | Preserve retry ledgers, action/reward/resource-pack pollers and acknowledgements; authorization and operation replays do not double-deliver inventories or rewards. |
| MC-07 | [ ] | minecraft-integrations | MC-01, SDK-06 | Preserve both 1.20.1 and 26.3 client source sets, rendering/HUD/maps/dialogue/preferences/link; generic public UI and optional experience capabilities are explicit. |
| MC-08 | [ ] | minecraft-integrations | MC-02, AUTH-09, OPS-08 | Document/test admission, authlib-injector/JVM flags, launcher requirements, proxy/IP trust and external HTTPS stack on shared game hosts; state incompatible host restrictions. |
| MC-09 | [ ] | minecraft-integrations | MC-04, MC-07, BRAND-14, BRAND-15 | Apply reviewed Velora Java/mod/plugin/command/permission/channel/resource namespace changes with transitional compatibility; old server config and clients still interoperate where promised. |
| MC-10 | [ ] | minecraft-integrations | MC-05, MC-06, MC-09 | Verify Paper baseline, Fabric/Forge 1.20.1/1.21.1/26.x and both companion variants using approved actual matrix; remapping/shading/descriptors and optional API behavior are correct. |
| MC-11 | [ ] | minecraft-integrations | MC-10, EXP-11 | Validate private gameplay contribution artifact plus public-only plugin/mod, including shop/utility/guild/chat/cosmetic execution and classloader ownership. |
| MC-12 | [ ] | minecraft-integrations | MC-08, MC-10 | Ship neutral config/sample plugin/mod installation docs and diagnostics; no operator needs owner-specific endpoints or credentials to connect a supported server. |

## 10. Complete rebranding and compatibility matrix

### Supplied visual sources

- **`Neon Galaxy V Emblem.png`**: angular galaxy V with orbital planet/star details;
  use as the source emblem for launcher/site/mod/mobile icons and compact marks.
- **`Velora Galaxy Graffiti Logo.png`**: horizontal graffiti Velora wordmark with
  orbital/planet details; use for headers, welcome/about/splash and docs artwork.
- Visual direction supplied: saturated violet/magenta, white highlights and dark
  outlines. Exact UI palette/accessibility tokens are to be derived and tested,
  not guessed from the preview. The lettering artwork is not evidence of a licensed
  UI font; choose the UI typography separately.
- Images are visible in this conversation; the original PNG bytes have not been
  imported into this cloud workspace. Verify resolution, alpha/background, color
  profile and file checksums when obtaining the source files. Preserve originals;
  do not assume black preview pixels are transparent or replace them with generated art.
- Full-detail artwork may be illegible at 16–32 px. Prepare approved simplified,
  monochrome/high-contrast and mask-safe derivatives while preserving the original
  identity. This plan does not edit or generate images.

### Naming rule

New user-facing product/platform defaults use Velora. Historical audit/archive text
keeps provenance. Existing operator instance/server/world/guild/account names and
their artwork are user data: never replace them merely because they contain
"SCOPENET". SMP/Frontiers experience identities are changed only via their private
reviewed defaults/configuration, with old installations preserved.

Do not apply a repository-wide search/replace. Treat presentation text separately
from compatibility identifiers. Retain scoped aliases or a proven conversion for
old headers/env vars/commands/permissions/paths/channel/resource IDs. Record each
permitted legacy occurrence and its retirement policy in a machine-checkable
allowlist, including archival text; "zero search hits" is not the success criterion.

| ID | Done | Owner | Depends on | Work and acceptance |
| --- | --- | --- | --- | --- |
| BRAND-01 | [ ] | sdk/docs | GOV-03 | Obtain/catalogue both supplied PNG originals, checksum/alpha/resolution/license information and intended uses; canonical assets are preserved verbatim with no personal filesystem paths published. |
| BRAND-02 | [ ] | docs | GOV-02, GOV-09, BASE-02 | Inventory every SCOPENET/scopelauncher/scopedd/operator-origin name and identifier, generated output and binary metadata; assign replacement, legacy alias or archive status and test owner. |
| BRAND-03 | [ ] | sdk/docs | BRAND-01 | Define approved Velora palette, typography, contrast/focus/motion rules, logo spacing/background treatment and host-overridable defaults; use supplied visual direction and accessible UI contrasts. |
| BRAND-04 | [ ] | sdk | BRAND-03 | Prepare source/derived emblem/wordmark, small-size/maskable/monochrome/dark/light exports and brand manifest; inspect legibility and clipping without destructive overwriting of originals. |
| BRAND-05 | [ ] | all apps | BRAND-02, BRAND-03 | Replace product text/log titles/default branding/about/email/Discord/landing/companion copy with Velora; preserve operator-chosen settings and experience-specific identity. |
| BRAND-06 | [ ] | launcher/panel/mobile | GOV-02, BRAND-02 | Remove hard-coded `scopenetmcpanel.scopedd.lol` and owner repo fallbacks for new public installs; require/offer host URL onboarding and configurable update/service URLs. Existing configured accounts keep their URLs. |
| BRAND-07 | [ ] | all Rust repos | BRAND-02, SDK-13 | Rename `scopenet-{shared,core,panel,launcher}` packages/binaries/library imports to approved Velora coordinates; update Cargo manifests/locks/tests/build scripts and transitional binary wrappers if required. |
| BRAND-08 | [ ] | sdk/panel/launcher | BRAND-02, SDK-13 | Rename npm scopes/Vite aliases/exports, package metadata, runes scripts and generated declarations; no stale source alias or private package enters the public graph. |
| BRAND-09 | [ ] | infras/auth/panel | BRAND-02, GOV-09 | Introduce approved `VELORA_*` environment/config names with legacy `SCOPENET_*` aliases, documented precedence/conflict warnings and defaults; preserve saved configuration paths/values. |
| BRAND-10 | [ ] | sdk/panel/MC | BRAND-02, SDK-05 | Introduce `X-Velora-Instance` with approved legacy header/query compatibility; reject conflicting contexts and preserve server-token ownership/HTTP API v1 paths unless a separate migration is approved. |
| BRAND-11 | [ ] | launcher | BRAND-04, GOV-09 | Rebrand Tauri product/window/publisher/bundle/installer strings and icon PNG/ICO/ICNS/SVG assets; preserve signing/update identity or perform the approved installer transition. |
| BRAND-12 | [ ] | launcher | GOV-09, BASE-05 | Handle `net.scopenet.launcher` app-data/keyring service identifiers and encrypted secret lookup: migrate/copy only after verified read, retain backups, avoid duplicate installs/account resets. |
| BRAND-13 | [ ] | panel/mobile | BRAND-04, GOV-09 | Rebrand favicons/webmanifest/PWA icons and Capacitor name/app ID/splash/app icons; verify mobile signing/package/deep-link upgrade constraints before changing `net.scopenet.player`. |
| BRAND-14 | [ ] | minecraft-integrations/sdk | BRAND-02, GOV-09 | Rebrand Java group/package/artifact names, entry classes, plugin/mod descriptors, build-info keys, resource namespaces, locale strings and mixin references; keep approved API/config/mod-ID shims without duplicate content registration. |
| BRAND-15 | [ ] | minecraft-integrations/experiences | BRAND-14 | Add `/velora` and reviewed legacy `/scopenet` aliases; migrate/alias permission nodes and `scopenet:c2s`/`scopenet:s2c` companion channels plus any other inventoried IDs; persisted item/model/namespace references survive upgrades. |
| BRAND-16 | [ ] | infras/launcher/panel | BRAND-02, GOV-08 | Rebrand GHCR/image/container/service/volume descriptions, installer/JAR/mobile filenames, checksums/validator expectations, repository links and download/update metadata; preserve old aliases/redirects and never rename a live data volume without migration. |
| BRAND-17 | [ ] | sdk/infras/apps | BRAND-04 | Replace cross-checkout `scripts/icons/build.mjs` with versioned common asset generation and owner-local outputs; launcher/mobile/web/companion all use verified Velora derivatives. |
| BRAND-18 | [ ] | experiences | BRAND-05, BRAND-04, EXP-01 | Apply approved private SMP/Frontiers preset/news/widget/artwork/command catalog labels; preserve operator branding and original private asset provenance. |
| BRAND-19 | [ ] | docs | BRAND-05, BRAND-16 | Update guides/examples/screenshots/canvas/release notes/project links and regenerated command references; mark historic SCOPENET names intentionally rather than falsifying archived material. |
| BRAND-20 | [ ] | infras/all apps | BRAND-07, BRAND-08, BRAND-09, BRAND-10, BRAND-11, BRAND-12, BRAND-13, BRAND-15, BRAND-16, BRAND-17, BRAND-19 | Run residual-name allowlist and fresh/legacy visual/config/data/API/IPC/channel/installer tests; close G4 only when all replacement/alias decisions have evidence. |

## 11. Host-ready operations and deployment tasks

| ID | Done | Owner | Depends on | Work and acceptance |
| --- | --- | --- | --- | --- |
| OPS-01 | [ ] | infras | GOV-10, PUB-06, PANEL-04, AUTH-08 | Define public Compose topology/internal URLs/volumes/network and optional private profile; only gateway/public ports exposed by default and public profile starts with no experience credentials. |
| OPS-02 | [ ] | infras | OPS-01 | Replace root-checkout Docker inputs with versioned public binary/web/icon artifacts; preserve source-build route or documented replacement and provenance. |
| OPS-03 | [ ] | infras | OPS-02 | Package non-root panel/auth images with writable `/tmp`, persistent UID/path ownership, minimal runtime dependencies and healthcheck entrypoints; SQLite spill/startup/upgrade tests pass. |
| OPS-04 | [ ] | infras/apps | OPS-01, BRAND-09 | Supply neutral `.env.example`, configuration reference and secret-file/generated-secret handling; validate bind/public/internal URLs, trusted proxies, uploads/memory and missing/invalid secrets with actionable errors. |
| OPS-05 | [ ] | panel/auth | OPS-04 | Implement safe first-admin/bootstrap/init flow and optional integrations disabled by default; no fixed password/JWT/key or duplicate administrator on restart/migration. |
| OPS-06 | [ ] | infras | OPS-03, OPS-04 | Provide DNS/TLS/reverse-proxy examples, websocket/stream/upload/body-size/timeouts and forwarded-header trust; public auth/API-location/callback/asset URLs work behind HTTPS. |
| OPS-07 | [ ] | infras/panel | OPS-01, GOV-05, SDK-10 | Implement experience registration/config validation and explicit missing/disabled/unsupported module behavior; authorize private profile separately and document installation/update/removal without deleting data. |
| OPS-08 | [ ] | infras/MC | OPS-06, PANEL-04 | Verify game server -> panel/auth connectivity, credentials/scope binding, IP/proxy/session admission, firewall/DNS/TLS egress and external-host topology; no shared filesystem assumption. |
| OPS-09 | [ ] | infras | OPS-03, AUTH-07, BASE-08 | Produce audited migration/init tooling with dry-run/preflight/backups/record counts and resumable markers; upgrade original `/data` without deleting or replaying gameplay. |
| OPS-10 | [ ] | infras | OPS-09 | Implement consistent backup/restore for panel/auth/private stores/assets/signing keys with secure retention; perform an actual clean-host restoration and document what survives. |
| OPS-11 | [ ] | infras | OPS-10 | Implement pinned-version upgrade/rollback playbook per schema compatibility, interrupted upgrade and original archive checkpoint; never suggest `compose down -v` for normal updates. |
| OPS-12 | [ ] | infras/apps | OPS-03, AUTH-09 | Define readiness/liveness/startup/restart order and graceful shutdown; test auth outages, slow startup, temporary network failure and reconnect without privilege bypass. |
| OPS-13 | [ ] | infras/panel/auth | OPS-04 | Provide actionable redacted logs/diagnostics/version/config checks and support bundle policy; operator can identify endpoint/volume/temp/permission errors without leaking secrets/player content. |
| OPS-14 | [ ] | infras | OPS-03, BASE-07 | Measure idle/active CPU/RAM/storage/bandwidth, map/uploads/concurrency and large migrations; publish minimum/recommended resources and tested limits rather than relying on old idle-memory claims. |
| OPS-15 | [ ] | infras | GOV-08, OPS-02 | Build/test approved Linux amd64/arm64 image/binary variants and desktop target coverage; clearly state unsupported architectures instead of claiming multi-arch from cross-compile code alone. |
| OPS-16 | [ ] | infras/auth/panel | OPS-04, OPS-10 | Provide native Linux binary/static-assets install and systemd units with working dirs, users, temp/storage/env and upgrade commands; fresh install and restore require no Docker. |
| OPS-17 | [ ] | infras | GOV-10, OPS-04, OPS-08 | Provide reviewed Pterodactyl/Pelican service templates with host allocations, writable volume paths, restart commands and externally hosted auth/HTTPS options; no privileged Docker/socket access inside game containers. |
| OPS-18 | [ ] | docs/infras/MC | OPS-08, MC-08 | Provide game-only/shared-host procedure for supported plugin/mod/JVM/authlib setup and separate Velora stack; capability checklist explains restricted-host incompatibility and alternatives. |
| OPS-19 | [ ] | launcher/mobile/panel | BRAND-06, OPS-06 | Make host endpoint discovery/onboarding and host-branded distribution configurable; stock public clients use the operator endpoint and no owner-owned central service is mandatory. |
| OPS-20 | [ ] | infras/panel/auth | OPS-06, OPS-12 | Document single-network tenancy vs multiple deployments, service credentials/rotation/trust, CORS/CSP and authorization boundaries; an instance scope never becomes cross-host authorization. |
| OPS-21 | [ ] | infras | OPS-03, OPS-13, OPS-14 | Publish operational monitoring/retention/disk-full/upload/temp-storage checks and optional log/metric integrations; base stack doesn't require paid monitoring or provider accounts. |
| OPS-22 | [ ] | infras/experiences | EXP-15, OPS-07, OPS-10 | Supply private profile/composition artifact setup, registry authorization, module version checks and separate backup/upgrade handling; public defaults never pull a private image. |
| OPS-23 | [ ] | infras/docs | OPS-06, OPS-10, OPS-11, OPS-16, OPS-17, OPS-18, OPS-19 | Run fresh-operator walkthroughs for all hosting routes from published docs; a hoster can install/connect/update/restore using their own URLs and credentials. |
| OPS-24 | [ ] | infras | OPS-12, OPS-15, OPS-20, OPS-21, OPS-23 | Close G5 with packaged-image/native/control-panel/shared-host evidence, known limitations and exact tested configurations. |

## 12. CI/CD, releases, documentation and acceptance

### Independent builds and artifacts

| ID | Done | Owner | Depends on | Work and acceptance |
| --- | --- | --- | --- | --- |
| REL-01 | [ ] | infras/all repos | GOV-08, PUB-01 | Split owner-local checks/locks/triggers from reusable workflows; pin refs and retain approved GitHub/Gitea runner parity with public runner alternatives. |
| REL-02 | [ ] | infras | REL-01, SDK-13 | Implement coordinated immutable compatibility manifest across independent SDK/app/extension versions; no single monorepo version stamp or floating private dependency. |
| REL-03 | [ ] | infras | REL-01, OPS-02 | Publish artifact contracts for panel/auth binaries/web/icons, desktop/mobile packages and per-loader/client JARs; restore executable modes and validate descriptors/filenames/targets/hashes. |
| REL-04 | [ ] | infras | REL-03, PUB-08, PUB-09 | Separate public base and private edition release channels, registries, caches, retention and provenance; publication cannot accidentally include private gameplay bundles. |
| REL-05 | [ ] | infras/panel | REL-02, REL-03, GOV-09 | Establish Velora release/update URLs and legacy aliases, package/image provenance/signature/checksum policy and trusted delivery; third-party host can publish its own compatible distribution. |
| REL-06 | [ ] | infras/launcher | REL-03, BRAND-11 | Validate NSIS/Windows, Linux AppImage/deb and macOS universal/DMG workflows, signing options and approved required/optional gates. |
| REL-07 | [ ] | infras/panel | REL-03, BRAND-13 | Validate Android/iOS build/signing/sideloading paths and PWA fallback; document operator signing/app ID requirements rather than assuming access to owner signing accounts. |
| REL-08 | [ ] | infras/MC | REL-03, MC-10 | Validate every approved server/client JAR against actual loader/version/Java/remapping/shading matrix; keep legacy clients and distinguish install-engine NeoForge/Quilt support from nonexistent integration artifacts. |
| REL-09 | [ ] | infras | REL-04, OPS-03 | Gate image/release publication on packaged runtime/temp/isolation/persistent restart smoke tests and metadata validator tests; native unit tests alone cannot publish `latest`. |
| REL-10 | [ ] | infras | REL-02, REL-09 | Produce release notes/compatibility/support matrix with reproducible evidence and hashes; pin deployment examples to tested versions/digests with explicit update commands. |
| REL-11 | [ ] | infras | REL-04, BASE-06 | Preserve authorized legacy downloads/checksums/redirects and archive index privately/publicly according to GOV-04; old updater/host URLs do not become dangling links. |
| REL-12 | [ ] | infras/user | PUB-12, TEST-12, REL-06, REL-07, REL-08, REL-10, REL-11 | Review and publish the first accepted Velora release/stack manifest; do not change public defaults or tag production releases before approval/evidence. |

### Documentation and developer handoff

| ID | Done | Owner | Depends on | Work and acceptance |
| --- | --- | --- | --- | --- |
| DOC-01 | [ ] | docs | GOV-01, BASE-02 | Extract all existing guides/images/screenshots/implementation summary/companion review/canvas with provenance; replace private implementation material with approved public interface docs. |
| DOC-02 | [ ] | docs | GOV-05, GOV-06, GOV-07 | Publish ownership/service/storage/extension ADRs and diagrams; public/private boundaries and temporary facade retirement criteria are explicit. |
| DOC-03 | [ ] | docs | SDK-14, PUB-10 | Generate API/protocol/SDK reference and extension walkthroughs from released schemas/synthetic catalog; no `../shared` or private-source checkout required. |
| DOC-04 | [ ] | docs/all repos | SDK-15, REL-01 | Publish per-repo clone/build/test/local-dev and dependency update instructions; a new contributor needs no monorepo workspace/private credentials. |
| DOC-05 | [ ] | docs | OPS-23 | Publish tested Compose/native/control-panel/game-only hosting quickstarts, capability requirements/TLS/network/storage/config and resources; include optional-provider/experience setup. |
| DOC-06 | [ ] | docs | OPS-11, AUTH-10, BRAND-20 | Publish SCOPENET -> Velora data/config/identity/installer/plugin/channel upgrade and restore guide, with alias/conflict/retirement details and no unsafe volume deletion. |
| DOC-07 | [ ] | docs | PANEL-12, LAUNCH-12, MC-12 | Update public launcher/admin/player/mobile/integration guides and separate private gameplay guides; regenerate screenshots/art references and preserve historic context. |
| DOC-08 | [ ] | docs | PUB-04, OPS-13 | Publish contribution/security/support/troubleshooting, redacted diagnostics, optional integration setup and documented unsupported hosting restrictions. |
| DOC-09 | [ ] | docs | DOC-01, DOC-03, DOC-05, BRAND-19 | Verify links/snippets/generated command/reference versions and accessibility of docs assets; public docs CI does not fetch private sources/data. |
| DOC-10 | [ ] | docs | DOC-02, DOC-04, DOC-06, DOC-07, DOC-08, DOC-09 | Update this tracker/source manifest and handoff index with repository links, remaining work and acceptance evidence; no missing domain/test/artifact quietly marked migrated. |

### Final acceptance and deliberate cutover

| ID | Done | Owner | Depends on | Work and acceptance |
| --- | --- | --- | --- | --- |
| TEST-01 | [ ] | infras | PUB-06, SDK-15, PANEL-12, LAUNCH-12, MC-12, AUTH-10 | Build/test all seven public repos with private access unavailable and fresh caches; public stack supports a real vanilla/generic install/auth/connect/launch loop. |
| TEST-02 | [ ] | infras/experiences | EXP-15, MC-11, PANEL-12, LAUNCH-12 | Run every baseline gameplay/admin/player/companion operation in the authorized private edition; compare values/history/UI/control availability, not just compilation. |
| TEST-03 | [ ] | infras | TEST-01, TEST-02, AUTH-09 | Verify old/new launcher/JAR/companion/API combinations, scopes/errors/admission/updates, hidden/disabled instances and missing extensions; publish supported version ranges. |
| TEST-04 | [ ] | infras | OPS-09, OPS-11, TEST-02 | Upgrade original sanitized volumes/keys/assets/local installs; interrupt/resume and restore rollback checkpoints; no duplicate balance/XP/reward/delivery or lost identity. |
| TEST-05 | [ ] | infras | AUTH-10, PANEL-06, OPS-20 | Test live account/group revocation, cross-instance/server-token rejection, disabled modules, deletion/anonymization retries, proxy trust, auth outages and asset authorization. |
| TEST-06 | [ ] | infras | OPS-24, TEST-01 | Reproduce complete public setup on each promised host profile/architecture, including external game host and fresh admin; record exact commands/version/resource limitations. |
| TEST-07 | [ ] | infras | BRAND-20, TEST-04 | Verify emblem/wordmark/export usage, UI readability/contrast, legacy config/paths/keyring/items/permissions/channels and branded installer/mobile migration on actual targets. |
| TEST-08 | [ ] | infras | REL-09, TEST-01, TEST-02 | Verify packaged release runtime, SQLite temp spill, first boot, persistent restarts, readiness, workload limits and source/artifact matching. |
| TEST-09 | [ ] | infras | PUB-11, REL-04, DOC-09 | Final audit of public git refs/releases/packages/source maps/docs/logs/cache/fixtures for private experience implementation and secrets; check public licenses/notices/provenance. |
| TEST-10 | [ ] | docs/infras | BASE-02, TEST-02, DOC-10 | Reconcile every original file/module/feature/test/artifact with its new home or approved archival status; remaining compatibility adapters have owner/removal criteria. |
| TEST-11 | [ ] | infras/user | TEST-03, TEST-04, TEST-05, TEST-06, TEST-07, TEST-08, TEST-09, TEST-10 | Review G3–G7 evidence, publication scope, rollback/support and unresolved risks; record acceptance before release/cutover. |
| TEST-12 | [ ] | infras/docs | TEST-11 | Produce signed-off release readiness/handoff summary and approved cutover checklist; no production changes or repository deletions occur merely to complete this task. |

## 13. Source migration coverage ledger

Use BASE-02 for a machine-readable per-file ledger when implementation starts.
The following mapping ensures no current directory family disappears between plans:

| Source family | Extraction tasks | Rebrand/operations work |
| --- | --- | --- |
| `crates/shared` and route/frontend/Java duplicated contracts | SDK-01–SDK-07, SDK-12 | BRAND-07–BRAND-10, contract tests |
| `crates/core` | SDK-08, LAUNCH-01–LAUNCH-03 | BRAND-07, loader/launch compatibility |
| `panel/server` platform/mixed handlers | PANEL-01–PANEL-07, AUTH-01–AUTH-09, EXP-01–EXP-12 | BRAND-05–BRAND-10, OPS-03–OPS-13 |
| `panel/web`, `panel/icons`, `mobile` | PANEL-08–PANEL-12, EXP-13, SDK-11 | BRAND-04/05/06/08/13/17, REL-07 |
| `launcher` | LAUNCH-01–LAUNCH-12, EXP-13 | BRAND-05/06/07/08/11/12/17, REL-06 |
| `integrations/api/common` | SDK-12, EXP-11, MC-01–MC-06 | BRAND-14/15, REL-08 |
| `integrations/paper/fabric/forge/minecraft/fabric-client` and examples | MC-01–MC-12 | BRAND-14/15/17, OPS-08/18 |
| `shared/{casino,board,commands}` | EXP-04/11/13, SDK-01/05 | Private gameplay/catalog rebrand BRAND-18; synthetic public docs DOC-03 |
| `shared/{experience,map}` | SDK-05/11, EXP-10/13/14 | Generic public host vs private Frontiers registry split; BRAND-18 |
| `branding`, generated product icons | BRAND-01–BRAND-04, BRAND-17 | SDK platform assets and product-owned exports |
| `docs`, README, implementation summary/review artifacts | DOC-01–DOC-10 | BRAND-19, privacy/license/link review |
| Docker/Compose/env/config and CI provider directories | OPS-01–OPS-24, REL-01–REL-11 | BRAND-09/16, non-root/temp/health/upgrade gates |
| `scripts/ci`, demo/generators and one-off `fix-*.cjs` | REL-01, PANEL-11, BRAND-17, DOC-03 | Reusable infras tooling; private gameplay demo payloads stay private; retain rewrite tools privately if they expose private implementation |
| Historical `release-artifacts`, root manifests/lockfiles/gitignore/rustfmt | BASE-06, PUB-02/03, REL-11, owner-local extraction tasks | Approved archive visibility; local build configurations/locks preserved |

No one-off script, sample, historical binary or copied test containing private
gameplay is automatically public just because its proposed owner is infras/docs.
Private originals remain in the private archive/experiences; public replacements
contain only approved generic material and provenance summaries.

## 14. Blocking decisions and completion definition

Already specified by the user: the Velora name and supplied emblem/wordmark, eight
repositories, seven public/open source and experiences private, multiple containers
acceptable, preserve functionality, and **plan first**.

Confirmed: organization `VeloraMCDev`, URLs below, and MIT for seven platform
repositories. Still unresolved: domains/package coordinates; artwork/private
distribution terms; extension deployment/UI trust strategy; public contract
disclosure scope; legacy identifier/installer/mobile compatibility policy;
registry/provider/matrix/resource support; auth/storage/communication ownership
details. GOV tasks settle these before dependent work, without stopping this
planning delivery or pretending decisions were approved.

This project's implementation is complete only when every task is either verified
done or explicitly reviewed as deferred/out of scope, the eight-repository ledger
accounts for all existing functionality, public builds need no private access,
authorized experiences retain their behavior, Velora compatibility upgrades work,
and hosters can install/connect/operate/restore supported deployments from the docs.
Publishing a renamed UI or creating eight empty repositories does not meet that
definition.

## 15. Stage 1 work log — 2026-10-04

Owner: Codex. Source baseline: `57daa92cb6cb4027982d06bfa1c692ab9edb88ff`.
Actual destinations: `https://github.com/VeloraMCDev/{launcher,panel,
minecraft-integrations,authentication,sdk,experiences,infra,docs}`.
Logical `auth`/`infras` labels in this tracker mean `authentication`/`infra`.

| Task/subtask | Status | Evidence / remaining scope |
| --- | --- | --- |
| GOV-01 | done (stage 1 scope) | Explicit user instruction to work on step 1. No production migration/publication authorized by this checkbox. |
| GOV-02.a | done | All eight supplied destination URLs resolved and were empty when inspected; package scopes/domains remain pending. |
| GOV-03.a | done | User selected MIT; LICENSE in seven platform foundations. Artwork/trademark/private terms remain pending. |
| BASE-01.a | done | Exact main revision including Docker /tmp fix frozen; contract source checksums recorded. Full platform baselines/known failures remain pending. |
| BASE-02.a | review | All 796 original tracked files hashed in infra/migration/source-inventory.json. Mixed ownership, final paths, licenses and tests remain incomplete. |
| SDK-01.a | review | Initial Rust-only platform DTO subset extracted with provenance; no gameplay policy/preset. Broader disclosure and canonical schema decisions remain pending. |
| SDK-03.a | done | Independent Cargo workspace/lock, public dependencies, six compatibility/version tests, formatter and clippy validation. Apps have not adopted it; full SDK acceptance remains pending. |
| BRAND-04.a | done | Velora name used in repository foundations and new SDK default display branding; explicit operator branding retained. Full application rebrand pending. |
| BRAND-01 | blocked | Original attached PNG files unavailable in cloud filesystem; no artwork substitution or license assumptions. |
| Publication / PRs | blocked | Monorepo Git writes available; all eight Velora pushes authentication-rejected (credential helper HTTP 401), API 403. Target branches remain local; recoverable patches are on the monorepo branch. Empty targets lack a PR base. |

Detailed changes, validation and resume order: [STAGE_1.md](STAGE_1.md).
No monorepo functionality was removed, no consumers switched, and no data/volume
migration occurred. Keep full task checkboxes open for partial acceptance above.
