# Initial Agent D — frontend architecture / UX audit

Audit started 2026-10-11; report completed 2026-10-12 (America/New_York). Base HEAD: `4f639266a515c3cb05769b055a4772c4dcb89cb8`.

## Scope and evidence discipline

- Read root `AGENTS.md`, `MONOREPO.md`, `docs/BOUNDARIES.md`, `docs/VALIDATION.md`, and `docs/DEPLOYMENT_TARGETS.md`; also read launcher, Panel web, and reusable Panel UI READMEs. No deeper AGENTS.md was found within the audited frontend paths. The Master Plan designation comes from the task; a separate matching plan document was not discovered in the Markdown search.
- Read-only source audit. The only agent-authored file is this report. No dependency installation, package/lockfile mutation, frontend build/check/runes duplication, native runtime, backend startup, commit, branch, PR, or deployment.
- Primary scope: `launcher/src`, `panel/web/src` (admin, landing, player), and their shared composition dependencies. Panel mobile/operations are separate hosts, not included in the size totals or runtime acceptance claims.
- Evidence labels: **source-observed** means inspected source; **Main-reported** means supplied by the coordinating agent; **image-observed** means inspection of the supplied synthetic screenshot. Recommendations are not implemented behavior. Compiler success is not keyboard, assistive-technology, or native acceptance.
- During the audit, concurrent changes appeared in `crates/core/src/http.rs`, `crates/core/src/sync.rs`, `launcher/src-tauri/src/commands.rs`, and `launcher/src-tauri/src/game.rs`. They were not edited or reverted. Line references describe this working-tree observation, not an immutable release artifact.

## Executive assessment

The frontend already has useful seams: launcher IPC behind a browser-mock adapter, a shared capability/navigation layer, neutral widget rendering with host-supplied components, reusable login UI, and lazy player-panel pages. Preserve these. The highest-value work is small correctness/accessibility changes, then isolated page-controller and presentation extractions—not a visual rewrite or a global store/router replacement.

Priority order: restore Panel browser zoom; fix Home action accessible names and dialog focus behavior; prevent stale gameplay responses and distinguish failed loads from empty data; align launcher navigation entry points; harden hash parsing; then extract one narrow component/controller at a time. Measure production bundles before introducing additional lazy boundaries or shared style packaging.

## Inventory: source size, not transfer size

Method: recursively enumerate `.svelte`, `.ts`, and `.css` under each path, excluding `node_modules`; count physical lines (no extra line for a trailing newline) and UTF-8 bytes. Includes declarations and mocks; excludes JSON catalogues, JS/MJS, media, native code, generated output, and dependency trees. Shared rows are separate inventories, not additional bytes proven to ship in both applications.

| Root | Files | Svelte / TS / CSS | Lines | Source bytes |
| --- | ---: | --- | ---: | ---: |
| `launcher/src` | 66 | 49 / 16 / 1 | 11,716 | 606,692 |
| `panel/web/src` | 149 | 123 / 24 / 2 | 24,099 | 1,479,765 |
| `shared` | 56 | 32 / 24 / 0 | 3,963 | 314,081 |
| `packages/platform-ui` | 3 | 1 / 2 / 0 | 85 | 4,303 |
| `packages/panel-ui` | 1 | 1 / 0 / 0 | 118 | 6,980 |
| `ui` | 5 | 2 / 3 / 0 | 112 | 6,402 |

Launcher subdivision: pages 12 files / 5,459 lines; components 36 / 3,754; lib 15 / 2,123. Panel subdivision: admin/landing pages 43 / 11,976; components 57 / 4,893; lib 20 / 1,475; player subtree 26 / 5,064. Entry files/root styles account for the remaining files/lines.

| Hotspot | Physical lines / bytes | Exact evidence and mixed responsibilities |
| --- | --- | --- |
| `launcher/src/pages/Guilds.svelte` | 1,242 / 56,970 | Imports host store, IPC, wallet/map and shared board at :10; data orchestration at :64; lifecycle at :254 and :315; script ends :327; styles start :781. |
| `launcher/src/pages/Social.svelte` | 1,190 / 42,008 | Host state/IPC :12; friends/invites :54; search :75; conversation request/mutation :143; tabs :243; styles :674. |
| `launcher/src/components/PlayerProfileModal.svelte` | 988 / 38,751 | Friend mutations :117; script ends :165; large presentation before styles :538. |
| `launcher/src/pages/Settings.svelte` | 873 / 53,373 | Host/IPC imports :14; OAuth flow :30; updater invocation :311; storage actions :712; styles :772. |
| `launcher/src/pages/Quests.svelte` | 786 / 26,320 | Large page, inventoried from :1; below the faction/social correctness priority. |
| `launcher/src/pages/Stats.svelte` | 724 / 35,632 | Large page, inventoried from :1; defer until a specific independent section is selected. |
| `panel/web/src/pages/PublicLanding.svelte` | 1,061 / 79,937 | Configuration/social/gameplay arrays :18; preview message :36; media/link helpers :44; parallel public resource loading :122; styles :807. |
| `panel/web/src/play/pages/Guilds.svelte` | 871 / 64,382 | Permissions :53; request generation :62; API orchestration :67; script ends :262; styles :752. |
| `panel/web/src/pages/LuckPermsAdmin.svelte` | 655 / 41,345 | Large admin surface, inventoried from :1; domain-sensitive, not a first generic UI extraction. |
| `panel/web/src/pages/EconomyAdmin.svelte` | 635 / 35,712 | Large admin surface, inventoried from :1; preserve monetary contracts. |
| `panel/web/src/pages/InstanceEditor.svelte` | 599 / 33,525 | Large admin surface, inventoried from :1; preserve stored configuration IDs. |
| `panel/web/src/pages/Progression.svelte` | 589 / 37,231 | Large admin surface, inventoried from :1; do not rename capability/storage identities. |

File length is a maintenance signal, not proof of slow rendering. For example, Social contains 517 lines from its style opening through EOF; moving CSS alone would lower page length without improving request safety or UX.

## Architecture and coupling map

1. **Application hosts intentionally compose gameplay.** Both Vite configurations resolve `@velora/experience`, map, casino, commands and board directly into repository `shared` sources (`launcher/vite.config.ts:9`, `panel/web/vite.config.ts:11`). These are source aliases, not independently published package dependencies. This is not evidence that reusable platform libraries import complete hosts; the complete Panel composition is explicitly permitted by `docs/BOUNDARIES.md:12`.
2. **Launcher state is a host facade, not a neutral platform API.** `launcher/src/lib/store.svelte.ts:22` combines bootstrap, accounts, navigation and launch state; :86 bootstraps IPC, :129 refreshes manifests, :154 persists settings, :185 switches accounts, :198 launches. `launcher/src/lib/tauri.ts:20` selects native IPC versus dynamically imported in-memory mock. Feature controllers may take small ports, but should remain in the launcher/gameplay host unless genuinely neutral.
3. **Panel transport already has a reusable seam.** `panel/web/src/lib/api.ts:5` uses `shared/http/platform.mjs`; :8 supplies session token, route instance and unauthorized callback. Do not replace the transport or change its compatibility timeout merely to split pages. `panel/web/src/lib/session.svelte.ts:1` preserves `scopenet.panel.token`; `panel/web/src/components/Sidebar.svelte:16` preserves `scopenet.admin.nav.open`.
4. **Widget direction is sound.** `packages/platform-ui/ExperienceWidgets.svelte:5` accepts a component map and :14 restricts lookup to its own keys. `shared/experience/ExperienceWidgets.svelte:3` supplies a composition registry; `ui/experience/components.ts:3` composes Frontiers locally. Keep gameplay components out of `packages/platform-ui` and reusable `packages/panel-ui`.
5. **Capability vocabulary is duplicated.** `shared/experience/index.ts:5`, `ui/experience/index.ts:5`, and `packages/platform-ui/experience.ts:1` need compatibility review before consolidation. Shared navigation also owns gameplay presets (`shared/experience/index.ts:60`). A future neutral-contract extraction must not silently turn those presets into platform-library dependencies or rename `guilds`, `commands`, instance/widget IDs, or stored module keys.
6. **Code-loading policies differ.** Launcher pages are statically imported (`launcher/src/App.svelte:14`), as are many Panel admin/landing pages (`panel/web/src/App.svelte:9`). Player pages already have dynamic loaders (`panel/web/src/play/nav.ts:4`, :8) and loading/error UI (`panel/web/src/play/PlayApp.svelte:124`). Reuse that pattern only after Main's production analysis identifies a valuable split. Both builds disable CSS splitting (`launcher/vite.config.ts:15`, `panel/web/vite.config.ts:23`); Panel raises its chunk warning threshold at :24, which is not an optimization.

## Navigation and interaction findings

- **Launcher destinations have multiple owners.** Rail builds configurable navigation at `launcher/src/components/Rail.svelte:12`; the palette hardcodes another page list at `launcher/src/components/CommandPalette.svelte:21`; store union lives at `launcher/src/lib/store.svelte.ts:31`; rendering is an if-chain at `launcher/src/App.svelte:73`. Palette lacks Collection and Experience while Rail offers them conditionally, and palette labels/order do not derive from configured navigation. This is a source-observed discoverability inconsistency, not a failed route test.
- **Existing capability behavior must survive.** `shared/experience/index.ts:76` filters unknown/disabled configured pages, retains unconfigured defaults, and normalizes presentation labels without changing IDs. Player Guide remains reachable when admin commands are disabled. Launcher App enforces the exception at :35; player Panel applies it at `panel/web/src/play/PlayApp.svelte:25`. Frontiers and legacy SMP require distinct regression fixtures.
- **Panel admin has a useful registry already.** `panel/web/src/lib/adminNav.ts:1`, `panel/web/src/components/Sidebar.svelte:19`, and `panel/web/src/App.svelte:72` connect navigation and palette. Preserve grouping, active parent behavior and persisted expanded sections rather than creating another registry.
- **Hash parsing can throw.** `panel/web/src/lib/router.svelte.ts:4` runs `decodeURIComponent` directly on user-controlled hash segments at module initialization and on hashchange (:12). A malformed percent escape can throw; `go` (:20) and `hashFor` (:31) also duplicate prefix rules. Feature gating and role routing are UI affordances (`panel/web/src/App.svelte:91`, :139), not a substitute for backend authorization.
- **Navigation remount is intentional today.** Launcher keys page content by selected instance plus view (`launcher/src/App.svelte:72`); player Panel keys by instance plus page (`panel/web/src/play/PlayApp.svelte:125`). Do not accidentally retain stale form/chat state when extracting components or adding lazy loaders. Hashchange scrolls the window (`panel/web/src/lib/router.svelte.ts:17`); actual content scroll/focus behavior remains a browser acceptance gate.

## Tokens and accessibility

**Useful existing foundations:** shared semantic colors/radius/font variables in `launcher/src/app.css:2` and `panel/web/src/app.css:1`; launcher user scale, glass and reduce-motion settings in `launcher/src/lib/theme.ts:13`; focus-visible rules (`launcher/src/app.css:105`, `panel/web/src/app.css:160`); labelled switch inputs and focus rings (`launcher/src/components/Toggle.svelte:5`, :21; `panel/web/src/components/Toggle.svelte:5`, :25); live toast regions (both hosts' `src/components/Toasts.svelte:7`); `aria-current` navigation (`launcher/src/components/Rail.svelte:53`, `panel/web/src/play/PlayApp.svelte:80`).

**Observed risks and limits:**

- **Panel zoom is explicitly restricted.** `panel/web/index.html:5` sets maximum scale and disables user scaling. `panel/web/src/main.ts:8` installs `nozoom`; `panel/web/src/lib/nozoom.ts:8` cancels gestures, :11 cancels multi-touch motion, :15 synthesizes clicks on quick second taps, and :25 blocks desktop zoom events when installed. Installation is conditional on coarse pointer (:32), so it is incorrect to claim all desktop browsers lose keyboard zoom. Mobile/coarse-pointer access is the concrete concern.
- **Home tools need explicit accessible names.** `launcher/src/pages/Home.svelte:63`, :64, :65 use icon-only content plus `title`, without `aria-label` or visible text. Main reports three unnamed buttons in the synthetic browser snapshot. Titles can be fallback names in some accessibility mappings; the observed unnamed result makes explicit naming a testable fix rather than assuming all title-bearing buttons are always unnamed. Console also lacks an exposed pressed state at :64.
- **Dialog semantics exist, focus lifecycle does not.** `launcher/src/components/Modal.svelte:12`, :16; `panel/web/src/components/Modal.svelte:13`, :22; `panel/web/src/play/ui/Sheet.svelte:10`, :14 provide Escape, labelled dialog and modal semantics, but their implementations do not move/trap/restore focus or inert the background. Global Escape listeners make stacked dialogs a further test case. Palettes focus search (`launcher/src/components/CommandPalette.svelte:63`, `panel/web/src/components/Palette.svelte:53`) but that is not a complete modal focus policy.
- **Custom button-like cards are only partly keyboard-complete.** `launcher/src/pages/Social.svelte:439` and :443 handle Enter, not Space; the card nests more interactive content. Prefer one native profile button/link with separate sibling actions, not more suppression comments. Source scan found three launcher and two Panel lines with a11y suppressions; these counts are not counts of distinct violations. Caption suppressions for decorative previews should be evaluated separately from interactive terrain/card semantics.
- **Tokens already drift between surfaces.** Launcher maps success/danger (:9 of `launcher/src/app.css` and :14 of `launcher/src/lib/theme.ts`); Panel uses good/bad (`panel/web/src/app.css:15`). Launcher uses 15px root/rem scaling; Panel body uses 14.5px (:37). Player branding maps only selected colors (`panel/web/src/play/PlayApp.svelte:53`). Modal widths use rem in launcher (`launcher/src/components/Modal.svelte:5`) versus a numeric rem/pixel heuristic in Panel (`panel/web/src/components/Modal.svelte:10`). Preserve surface differences; document semantic aliases before sharing CSS.
- **Focus and contrast are gated, not proven failures.** Later field rules remove outlines in favor of `--ring` (`launcher/src/app.css:193`, `panel/web/src/app.css:353`), and Panel button focus is overridden by shadows (:423). Branding/custom CSS can change the result (`launcher/src/lib/theme.ts:37`). Need computed-style and keyboard review across branding fixtures; static tokens alone cannot establish contrast or focus compliance.
- **Motion controls exist with an explicit casino exception.** OS reductions exclude `.casino-motion` (`launcher/src/app.css:204`, `panel/web/src/app.css:379`); launcher's user switch is applied at `launcher/src/lib/theme.ts:29`. Autoplay background video is selected without checking reduce-motion (`launcher/src/components/Background.svelte:8`, :18). Validate reduced-motion video behavior and casino-local controls before claiming motion coverage.

## Prioritized narrow refactor backlog

Priority is an audit recommendation, not authorization to edit source. P1 means the first follow-up batch; P2 is structural/performance work after baseline gates. Each row is intended as a separate small change, not a single combined rewrite.

| Priority / slice | Scope and guardrails | Testable acceptance criteria |
| --- | --- | --- |
| P1 — Restore zoom | Panel viewport/nozoom only; retain safe-area support and normal scrolling. | Coarse-pointer fixture allows pinch zoom; Ctrl/Cmd +/-/0 and Ctrl-wheel are not cancelled; no synthetic duplicate button mutation on double tap; 200% zoom keeps sign-in, navigation and primary actions usable. |
| P1 — Name Home tools | Three buttons at Home :63; add explicit names and console state, keep icons/layout/actions unchanged. | Accessibility snapshot resolves exactly “Instance settings”, “Game console”, “Player stats & leaderboards”; console exposes its toggled state; keyboard activation calls the same actions once. Recheck progression-disabled fixture hides stats. |
| P1 — Dialog focus lifecycle | Fix each host's Modal/Sheet locally first, with a small reusable neutral helper only if warranted; no width/style unification. | Initial focus lands inside; Tab/Shift+Tab stay inside; only the top dialog responds to Escape; backdrop behavior unchanged; close restores connected trigger focus; background is not reachable; verify nested profile/account/settings dialogs. |
| P1 — Request ownership | Launcher Guilds :64 and Social :143; snapshot IDs/target, reject stale results; retain IPC method/argument identities. Panel faction sequence at :62 is an existing local example. | Deferred A/B responses resolving B then A leave B visible; switch conversation during send does not append to the new thread; unmount prevents stale writes; one logical faction bootstrap rather than effect/mount overlap at Guilds :315/:321; loading resets only for current request. |
| P1 — Load failure versus empty | First faction/social initial loads; do not generalize every transport in one patch. Current inner catches at Guilds :69 and Social :58 erase failure distinctions. | Failed fetch shows recoverable error/retry; a successful empty array shows the existing empty state; optional-resource failure does not erase successful main data; quiet refresh retains last-good data and reports staleness. |
| P1 — Malformed hash handling | Extract/test pure parse/format rules from router; preserve every valid hash/instance prefix and existing session keys. | `#/instance/%ZZ/control` and truncated escapes do not crash module load or hashchange; safe fallback is deterministic; round-trip encoded IDs (spaces, Unicode, encoded slash), platform routes and play routes; `go`/`hashFor` prefix outputs stay compatible. |
| P2 — Launcher navigation descriptor | Launcher-local metadata consumed by Rail and palette, with explicit social/settings extras; retain existing IDs and view lifecycle. | Configured labels/order match both entry points; Collection/Experience are discoverable when enabled; Frontiers/legacy SMP/current SMP preserve capability filtering and Guide exception; unknown IDs cannot add executable destinations. |
| P2 — One page seam at a time | After request fixes, extract one faction directory/presentation section; then Social conversation presentation. Keep mutations/controller in host or gameplay, not reusable platform UI. | Parent loses the selected section, child accepts explicit data/callbacks rather than importing global store/transport, same empty/loading/error UI and action payloads, no CSS/route drift; test adversarial response ordering before/after extraction. No arbitrary LOC target. |
| P2 — Landing resource controller | PublicLanding :122 resource orchestration only; leave block rendering and editor preview contract intact (:36). | Same endpoint/query shapes and preview origin check; distinguish config failure from optional leaderboard/server failure; stale sort response cannot replace latest sort; no authenticated fixture required for tests. |
| P2 — Semantic token contract | Document existing host aliases and width units before introducing a minimal neutral utility. No redesign, font replacement or mass spacing edits. | Default/custom/instance branding keeps computed colors/radii/scale; success/danger aliases agree semantically; 200% scaling and long synthetic names remain usable; keyboard focus visible across branded/custom-CSS fixtures. |
| P2 — Measured lazy split | Candidate: launcher casino or Panel admin heavy route, only after production baseline confirms benefit. Preserve player lazy loading and error handling. | Production entry/import graph demonstrates chosen route leaves initial JS; first-use loading/error/retry tested; repeat navigation and capability fallback work; compare cold-load transfer/parse and first-use latency, not dev timing. No dependency updates. |

Do not merge desktop and mobile faction pages wholesale: they have distinct transport, presentation and permission behavior. Do not convert all global state to context, replace hash routing with a new library, extract a full design system, rename protocol/persisted IDs, or alter SQL/runtime paths as part of these slices. Preserve independent lockfiles, licenses/provenance and build/release boundaries.

## Validation and visual/performance gates

### Executed by Agent D

`node --test shared/experience/index.test.mjs shared/experience/presentation.test.mjs ui/experience/index.test.mjs` — **12 passed, 0 failed**, Node **v25.6.1**. These existing tests cover capability/nav compatibility, Guide behavior and logo configuration; they do not cover the proposed router, async-controller or dialog changes. The mandated Node 24 baseline remains a gate. File inventory and source searches are read-only; no frontend commands or dependency changes were made.

### Main-reported baseline, not independently rerun

- Launcher `npm ci`, `npm run check`, `npm run check:runes`, and `npm run build` pass on Node 25.6.1; Svelte reports **0 errors / 0 warnings**. Main will rerun under Node 24.
- Build **2.22s**; main index JS **809.49 kB**, gzip **223.09 kB**; mocks separately chunked **29.39 kB** and **15.45 kB**. These are Main's build observations, not network/parse/interaction measurements or a Panel build baseline.
- Both rune scripts inspect only local `src` (`launcher/check-runes.mjs:13`, `panel/web/check-runes.mjs:13`), despite Vite's out-of-tree aliases. Their success must not be presented as exhaustive shared-source rune coverage. Whether to extend coverage is a separate validation-owner decision.

### Synthetic screenshot: inspected, not captured by Agent D

Main explicitly confirms the supplied browser contains only synthetic `Alex_Miner`, at `http://localhost:1420/?mock=main`, **1280×800**. Agent D inspected the provided file without launching or interacting with a browser/native app:

`/Users/alyssasitler/.t3/userdata/browser-artifacts/browser-screenshot-localhost-mv4q22fk-8d5f1146.png`

The artifact stays outside the repository; this report references it but does not copy it. Image-observed: launcher rail, selected Overview, primary Play, secondary icon actions, news column and synthetic account fallback are visible. No obvious overlap appears in that one viewport. The image cannot establish names, contrast ratios, Tab order, focus behavior, other resolutions or runtime speed. Main's unnamed-button snapshot and avatar DNS failure are separately Main-reported observations.

Source supports the synthetic seam: `launcher/src/lib/tauri.ts:6` detects native IPC and :32 loads the mock otherwise; `launcher/src/lib/mock.ts:9` selects fixture, :17 uses an example Panel origin and :102 defines synthetic account data. However, mock mode is **not** a universal network sandbox: avatars derive example-origin URLs (`launcher/src/components/Avatar.svelte:17`), icon catalogues use external artwork (`shared/experience/icons.svelte.ts:25`), and links can open externally (`launcher/src/lib/tauri.ts:57`). The reported avatar DNS failure is expected fixture isolation fallout, not proof of a production avatar outage. Before additional screenshots, use a fresh isolated browser context, synthetic fixture only, no native marker/session, controlled or blocked external requests, and no real token/store/backend. For deterministic visual tests, intercept avatar/media requests with synthetic local responses; do not point the fixture at a live Panel.

Panel preview remains **not demonstrated safe**: default Vite proxy can reach a configured backend (`panel/web/vite.config.ts:6`, :17), session reads persisted token (`panel/web/src/lib/session.svelte.ts:3`), and landing preview still accompanies public API loading (`panel/web/src/pages/PublicLanding.svelte:122`). No Panel screenshot/runtime was attempted. Require isolated storage plus complete synthetic API/media interception before visual acceptance.

### Remaining gates / handoff to Main

1. Node 24 launcher baseline and corresponding contract tests; completed by Main in the integration note below. Panel check/runes/build results remain unsupplied and unclaimed.
2. Main's production browser performance measurements are pending. The dev screenshot and build duration are not Core Web Vitals, startup or interaction proof. Record production artifact/fixture, cache state, network/CPU settings, initial transfer/parse, first navigation and repeated navigation before setting a budget.
3. Synthetic keyboard/assistive-tech checks: names, native button Space/Enter, dialog focus/stacking, zoom/reflow, focus visibility, branded contrast and reduced-motion/video/casino cases. No compliance certification from this audit.
4. Deterministic request-ordering and router tests for the proposed slices; existing 12 contract tests remain compatibility gates, not substitutes.

## Lead integration note — October 12, 2026

Main repeated launcher check/runes/build with Node 24.21.0: all pass, Svelte zero errors/warnings. The exact three-file capability/presentation/navigation test command above also passes all 12 under Node 24. Production preview navigation/paint observations and their limitations are in `performance-baseline.md`; no native-performance or Panel-runtime acceptance is implied.
5. Native desktop/game acceptance, packaging/signatures, upgrade/restore continuity and releases remain manual and outside this read-only audit. No actual player data, live store, paid resource or automatic deployment is involved.

**Outcome:** initial architecture/UX audit complete; actionable narrow backlog and explicit remaining gates recorded. No frontend behavior has been changed.
