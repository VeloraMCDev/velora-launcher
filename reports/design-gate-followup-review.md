# Independent bounded design follow-up review

October 15, 2026. This review owns only
`reports/design-gate-followup-review.md`. No browser/native UI interaction,
source/test edits, broad build, full-suite run or ledger edit was performed.
The lead retains the shared browser and full-suite/typecheck/build validation.
No signing, releases, installed stores, real accounts or infrastructure were used.

## Disposition and early finding

**Current disposition: accept the scoped source corrections with the evidence
limits below; the initial CompanionSettings P2 is resolved at source/test level.**
The corrected preview-local color no longer depends on global branding text, and
the final focused Node 24 run passes 30/30. The original request-changes finding
and earlier snapshots are preserved as history; the resolution addendum below
supersedes their open-P2 dispositions. This is not a complete VEL-015 review,
WCAG certification, native acceptance or a percentage award. The fixed verified
total stays 62/76.

The Companion regression and the audit helper's missing DOM-level test coverage
were reported to the lead before this report was written. A live browser fixture
and passing source tests do not resolve those findings by themselves.

### Initial P2 — global text blending can regress the fixed dark HUD preview

Historical finding against the initial global-text blend; resolved by the
preview-local correction documented in the final addendum.

Location: `launcher/src/components/CompanionSettings.svelte:60`.
The preview uses a fixed `#27312b` scene and
`rgb(23 27 32 / var(--hud-opacity))` module background, but the new 65% HUD-accent
blend takes its remaining 35% from global `--text`. `applyTheme` accepts the
branding text color; it does not constrain it to light text. Thus a light launcher
palette with dark text can darken labels on this independently dark preview.

Concrete source-derived synthetic example, reproduced with the production audit
module's contrast/compositing arithmetic under Node 24:

- Branding accent `#22d3ee`, global text `#000000`, default module opacity `.88`.
- Composed module background over the scene: approximately
  `rgb(24.92 29.64 33.32)`.
- Previous accent-only label: **9.32956:1**.
- Proposed `color-mix(in srgb, accent 65%, black)`: **4.08854:1**.
- The label is 9px normal text: this drops below the helper's 4.5:1 threshold.
- With the default light `#ecedf1` global text, the same calculation improves
  from 9.32956:1 to 10.51095:1. The default-theme improvement is real but not
  a no-regression result for administratively supplied light palettes.

This is an arithmetic/source counterexample, **not a browser-rendered finding**.
The existing default synthetic palette does not use black text. If the product
explicitly restricts all supported branding to light foregrounds, document and
enforce that contract before dismissing this case; the inspected theme consumer
does not establish such a restriction. No hostile custom CSS is needed here.

Requested correction at initial review: derive preview-label contrast against its actual fixed
dark scene/module background, or use a preview-local light text target instead
of global `--text`, then verify supported accent/opacity combinations. Include
a regression for a bright accent with dark global branding text. An arbitrary
blend alone is not proof that every very dark accent passes, so preserve bounded
claims even after correcting this regression. The lead should inspect the actual
preview and other settings in the existing synthetic browser session.

## Scoped source review

Read the applicable root instructions and architecture/validation/deployment
documents in the preceding acceptance audit; confirmed no deeper `AGENTS.md`
under launcher or reports. Inspected the requested current-checkout files,
the worker's actual reduced-motion helper/tests/GuildWallet changes, existing
action contrast helper, and relevant earlier implementation/acceptance reports.
The checkout contains concurrent unrelated changes; this is not certification
of the entire HEAD diff or any built artifact. No separate new worker
implementation report was found among the current report outputs inspected.

| Change | Bounded assessment |
| --- | --- |
| `theme.ts` danger tokens and NotificationBell badge | The badge consumes the background and black/white foreground from the same existing `actionContrast` result. Invalid/transparent/non-hex danger data selects the safe opaque fallback. This avoids mixing a computed foreground with the original unsupported background. The focused tests cover the helper and source wiring, not an independently rendered badge. |
| CommandPalette selected icon | The selected icon uses the same paired filled-action tokens. A reasonable contrast correction; source regex checks do not measure actual SVG replacement artwork, hover/disabled states or overrides. |
| Avatar fallback | Reducing generated HSL lightness to 30% preserves white initials and increases contrast. The test enumerates all 360 integer hues generated by the current username hash. It does not inspect avatar images, actual font rendering, custom CSS or rendered size/opacity. |
| Global links, NewsFeed labels/actions, Settings active badges | Semantic text-token reuse removes these direct accent uses. It is a bounded consistency/contrast improvement for the reviewed default palette, not a mathematical contrast guarantee across arbitrary branded text/surface/accent combinations. |
| Companion preview labels (initial snapshot) | Default dark-palette improvement, but the conditional branding regression above prevented unqualified acceptance of this individual change. See final correction addendum. |
| `prefersReducedMotion` and GuildWallet | The actual helper combines manual and system preferences with OR semantics, tolerates absent SSR globals/root/matchMedia and reads current values at each call. The wallet checks at admission and every scheduled frame; preference changes during an active animation settle at the target on its next frame without scheduling another. Existing hidden-document settlement and cleanup are retained. |

No new dependency, persisted identity, account behavior, native IPC or installed
path change is introduced by the scoped presentation/helper edits inspected.
This statement does not cover unrelated concurrent checkout changes.

## Rendered-audit helper: useful triage, not acceptance authority

Inspected `launcher/tests/rendered-accessibility-audit.mjs`, its new tests and
the actual callable `auditRenderedText` implementation. Its compositing/luminance
arithmetic and large-text threshold tests are useful. Complex ancestor backgrounds,
opacity, blend/filter/backdrop-filter, text shadow and differing text-fill color
are routed to `manual`, rather than automatically receiving a contrast pass.

Important blind spots and usage requirements:

1. **The new test file does not call `auditRenderedText`.** Its initial four tests cover
   `composite`, `contrastRatio`, `textThreshold` and the source-derived avatar hue
   calculation. They do not test actual traversal, computed-style/canvas parsing,
   visibility, manual classification or the browser API contract. The follow-up
   source/arithmetic tests described below do not change that limitation.
2. **Silent omissions are not a pass.** Only direct text child nodes are examined.
   Input/textarea values, placeholders, generated pseudo-element content,
   shadow DOM and iframe content are not traversed. SVG is explicitly excluded;
   icon/control boundaries and focus indicators need separate review. It is not
   a non-text contrast or keyboard/accessibility audit.
3. **`aria-hidden`/`inert` can omit visible text.** The helper skips text under
   those ancestors without recording it in `manual`; visibility to an AT tree
   is not itself a universal exemption from visual contrast requirements.
   Disabled controls are also skipped. State which exclusions are intentional;
   do not interpret zero failures as full visible-state coverage.
4. **Ancestor colors are not the full paint stack.** The model assumes an opaque
   white starting canvas, composes ancestor background colors and handles text
   alpha, but does not establish actual overlapping sibling/pseudo-element
   backgrounds, occlusion, clipping or image pixels. Ancestor image/gradient
   detection is conservative, not a substitute for resolving every manual case.
5. **Browser support and state coverage remain untested here.** The helper
   requires canvas 2D, `checkVisibility` and computed CSS behavior. Canvas color
   conversion/string comparisons are not native-webview portability proof.
   The callable does not discover unmounted routes, open dialogs, hover/focus,
   custom themes, zoom/scaling or reduced-motion states by itself.
6. **Manual and skipped cases need their own denominator.** Retain checked,
   failing and manual counts, the route/state/theme/viewport and named excluded
   surfaces. Inspect composed gradient/image/opacity cases separately. Do not
   publish a whole-app pass from an empty failures array.
7. **Use only synthetic fixtures for retained audit output.** Entries include
   the first 100 text characters; the helper is not a diagnostic redactor and
   could copy private visible text if run against a real account/store.

Independent in-memory Node probe, with a deliberately modeled document/canvas
and the **actual exported function**, confirmed these bounded behaviors:
black-on-black direct text yields one 1:1 failure; the same modeled visible text
under `aria-hidden` yields zero checked/manual entries; an input value with no
text node yields zero entries; an ancestor background image yields one manual
entry and zero checked. This is not real DOM/CSS/canvas/browser verification.
These are helper-scope limitations, not new product contrast regressions.

## Focused independent validation

Used the existing local executable:
`/Users/alyssasitler/.npm/_npx/387698761821791d/node_modules/node/bin/node`.
Its directly observed version is **v24.21.0**, not the shell's default v25.6.1.
No Node installation or dependency change was necessary.

From `launcher`, with `NODE24` set to that absolute executable:

```sh
"$NODE24" --test tests/action-contrast.test.mjs tests/rendered-accessibility-audit.test.mjs tests/reduced-motion.test.mjs
"$NODE24" --experimental-vm-modules --test tests/action-contrast.test.mjs tests/rendered-accessibility-audit.test.mjs tests/reduced-motion.test.mjs tests/accessibility.test.mjs
```

- First focused run: **16 passed**, zero failed/skipped.
- Expanded focused run: **27 passed**, zero failed/skipped. This includes existing
  modeled focus-trap cases and compiled production Settings SSR cases, not mounted
  keyboard/webview behavior. Counts overlap; they are not 43 independent tests.
- Concurrent lead edits added an editable-keybinding-opacity test between reads;
  it passed in the expanded run. This review does not claim to own that change.
- Expected Node experimental warnings concern VM modules and TypeScript stripping;
  neither run had a test failure.
- Wallet tests execute the production effect source extracted into a VM with
  mocked scheduling/Svelte globals and the actual helper. Their titles honestly
  say not rendered. They do not prove real Svelte scheduling, OS media preference
  events, actual frame timing, screen-reader announcements or unrelated animation.
- The synthetic audit-traversal probe and Companion arithmetic counterexample
  above ran separately in Node; neither is counted as a checked-in regression test.
- Full tests, typecheck, production build, browser rendering and native acceptance
  are the lead's work and are not independently claimed as passing here.

## Reviewed identities and remaining handoff

HEAD remains `4f639266a515c3cb05769b055a4772c4dcb89cb8` plus dirty source.
Key SHA-256 identities captured after the focused runs:

| Path (relative to launcher) | SHA-256 |
| --- | --- |
| `src/lib/theme.ts` | `ea2f301f0e6b134124030927588c31e958c38bbcf061a70722edc2a623fa7867` |
| `src/components/CompanionSettings.svelte` | `607ba67bd4ba9ef857e51743c60418fff8d53974f476e22dcded650bdda2e2bb` |
| `src/components/GuildWallet.svelte` | `f03785786a7d6ab92faf686e9ce15d14e8f88dd346f0530e9a2d8899d9d0baf8` |
| `src/lib/reduced-motion.ts` | `b0ac03bb2467a6ae609e37cb6a77021c4883494a5c07deb941a28c2c447302f9` |
| `tests/rendered-accessibility-audit.mjs` | `64e564a69034dad683efec9674099b0159c0ef62e6d842545bc8c586bf109c14` |
| `tests/rendered-accessibility-audit.test.mjs` (initial review) | `c8ee0c982cd8f1f922167ba02146d9737fc1198adef2b0bdaee87c2d663a1599` |
| `tests/reduced-motion.test.mjs` | `22aef76ddffa402bf129a052c9aaa7576f04eef1e71ce9cbb569be01c98e3c6e` |
| `tests/action-contrast.test.mjs` | `dd7c3ec61e6bf085cb0a78701f611512cd189736c24b4fcac4ac99dcefccdbc2` |

If these change, revalidate the affected finding/tests rather than presenting
this report as acceptance of the replacement artifact. The lead should correct
or explicitly resolve the preview branding case, keep audit manual/omitted
surfaces visible in the rendered report, and retain the complete design-gate
limitations. Earlier bounded accessibility reports already disclaim whole-gate
acceptance; this follow-up does not upgrade their source/browser/native scope.
Only this new review report is written; no checklist credit is assigned.

## Additional final-scope review

The lead requested inclusion of the editable-keybinding presentation and additional
Social/Market/shared guide/casino CSS fixes. No deeper `AGENTS.md` was found under
`shared` or `launcher`. The same report-only ownership and no-browser boundary
remain in effect. These additions do not change the Companion finding: its source
hash remains `607ba67bd4ba9ef857e51743c60418fff8d53974f476e22dcded650bdda2e2bb`.

### Additional source disposition

- **Settings editable bindings:** the `.binds` wrapper no longer receives the
  misleading disabled class/opacity while `keybinds_enabled` is false. The new
  explanation distinguishes editing a saved binding from applying it to Minecraft
  at launch. Existing capture handlers remain editable; no game-application flag
  or persistence behavior was changed by this presentation correction. The focused
  regression checks the explanation and absence of the dimming class/rule; it is
  not an end-to-end keyboard capture/save/launch test.
- **Social playing-on text:** direct accent use becomes `--state-info-text`.
  Reasonable default-palette correction with the existing arbitrary-branding,
  composed-background and branch-state limitations retained.
- **Market kind labels:** ordinary kinds use normal text; auction kinds use
  semantic info text over the existing translucent accent background. This is not
  an independently rendered ratio measurement of every auction/card/hover state.
- **Shared PlayerGuide:** active/category/code/link foregrounds use
  `--state-info-text` with a fallback composed from the host's existing accent and
  text variables. The diff is CSS only: no launcher or Panel host dependency was
  introduced. Existing imports, props, guide filtering/link validation and copying
  remain untouched. Both launcher Commands and Panel play Commands consume it;
  fallback support must therefore remain intact rather than relying only on the
  launcher defining the semantic token.
- **Shared CasinoPage:** active tabs now use normal host text on the semantic
  selected surface, with a host-palette fallback using existing accent and
  surface/background variables. This removes unconditional white-on-accent.
  The diff does not change tab selection/hidden-pane behavior, game logic, host
  adapters or imports. Launcher and Panel play both consume it. The current Panel
  stylesheet defines the fallback palette variables. Actual selected/hover/focus
  rendering in each consumer remains the lead's validation, not inferred from
  the source or compile result.

**No new blocking source defect found in these additional edits.** The original
conditional Companion contrast regression was still the outstanding targeted
correction at this additional-scope snapshot; see its resolution below. Semantic
variables and fallbacks are not guarantees against arbitrary
administrator CSS or every possible text/accent/surface palette.

### Additional independent checks

Using the same **Node v24.21.0**, compiled both shared components directly with
the installed `svelte/compiler` entirely in memory, with `generate: 'client'`
and `generate: 'server'`. All four current-source compilations succeeded with
**zero warnings**. The same four compilations of each file's HEAD source also
had zero warnings. No output artifact, bundler build or dependency installation
was created. This validates component syntax/code generation, **not** dependency
resolution, host integration or final bundle compatibility; those builds remain
the lead's responsibility.

Repeated the expanded four-file focused command above after the lead added two
more tests: **29 passed, zero failed/skipped**. This supersedes the earlier 27-test
snapshot; overlapping reruns must not be summed. The new tests check semantic-text
arithmetic for ten fixed accents against three fixed dark surfaces, and source
wiring for Social/Market/guide/casino. They do not execute the audit traversal,
use a real DOM, validate all custom palettes, or independently measure the
translucent auction and selected-surface combinations. No shared-host build,
full-suite or browser run was duplicated.

### Lead-reported 100-cell Settings matrix

The lead reports **100 cells**: ten default/custom accents across ten Settings
categories, with **3,530 checked text groups, zero failures and zero manual
entries**, under **glass=false** and **manual reduce-motion=true**. This reviewer
did not operate the shared browser, replay the matrix or inspect its retained
raw output/screenshots. These numbers are attributed observations, not independent
browser verification by this report; text groups across cells are not necessarily
unique controls or whole-app coverage.

The result supports the lead's specifically sampled Settings theme/category
configurations. It does **not** cover every conditional account/permission/data/
error/expanded/hover/focus state, other pages' complete matrices, all glass modes,
administrator custom CSS, system-only motion behavior or native webviews.
Zero manual entries under glass=false do not settle blurred/translucent/image
cases in other configurations. The audit helper's silently omitted surfaces and
non-text/focus/keyboard coverage gaps still apply. Manual reduced motion being
enabled is a fixed test condition, not proof of both motion modes and OS changes.
Neither this matrix nor these additional source fixes closes VEL-015 checkpoint 4.

Final additional source identities, SHA-256:

| Path (relative to repository root) | SHA-256 |
| --- | --- |
| `launcher/src/pages/Settings.svelte` | `7c555121fdb85b8e748d7b2110f0e968bde8333b22d2915ef5ac5b41391febb6` |
| `launcher/src/pages/Social.svelte` | `7595d4443af0031b5b9c07f7f8c739c8032255b898c7860f2fe9842f02e82f99` |
| `launcher/src/pages/Market.svelte` | `3008df57107515e7e207fe50e97f12087079117db8464787215fb5ee5437c16c` |
| `shared/experience/PlayerGuide.svelte` | `d67714b1b872faa47f77d01a84c08e0ae9f13a43aa6fb8366bccfc521f342b91` |
| `shared/casino/CasinoPage.svelte` | `33738b5bbe482a4c8082b693cc8bfbb517fdd0618201c7f7a50c49ab4acbc5af` |
| `launcher/tests/rendered-accessibility-audit.test.mjs` (final expanded run) | `3e7cb0a6ecb9a714b15bfdd548c3cb443dac84a842f2e072cc888e24d5b6867b` |

Only this review report is updated. No source, ledger or other report is modified.

## Final P2 correction and validation addendum

October 15, 2026. Independently read the patched source and new checked-in
regression. `CompanionSettings.svelte` now uses
`color-mix(in srgb,var(--hud-accent) 40%,#fff)` for the HUD labels. The fixed dark
scene and translucent module background are unchanged. The foreground no longer
depends on global `--text`; this directly fixes the initial finding rather than
requiring a new restriction on supported global branding text.

**Initial P2 resolved at source/test level. No outstanding blocker found in this
bounded review.** This resolution does not certify browser rendering, arbitrary
CSS, unrelated component states or the whole design gate.

### Independent repeat and regression assessment

Repeated the same expanded focused command with the actual existing
**Node v24.21.0** executable: **30 passed, zero failed/skipped**. The 29 previous
tests plus the new HUD regression pass; earlier runs overlap and must not be
summed. Expected VM-module/TypeScript-stripping experimental warnings remain,
not test failures. No full suite or browser/build run was duplicated.

The new test checks source wiring for the exact local-white blend and unchanged
scene/module backgrounds; calculates contrast for **12 opaque accents including
black and white**, at opacity samples **0.2 through 1.0 in 0.1 steps**; and retains
the actual cyan/dark-global-text counterexample at **0.88**. It asserts that the
old formula fails and the corrected formula passes that case. The arithmetic is
source-derived, not a DOM/canvas/paint execution of CSS `color-mix`.

An independent in-memory arithmetic probe using the production audit module's
exports reproduced **4.08854:1 old → 12.96820:1 corrected** for the cyan/0.88 case.
It also found a conservative channel-bound ratio of **4.99618:1**: any opaque
sRGB accent mixed 40% with white has every output channel at least 153; the fixed
scene/module composition is brightest at opacity 0.2, approximately
`rgb(35.8 44.6 40.8)`. This supports the correction throughout the allowed opacity
interval under that compositing model, not just at the discrete test samples.
It does not cover translucent/invalid/custom color expressions, parent opacity,
overlays, administrator overrides or native color-management differences.

### Updated lead-reported browser and build evidence

The lead reports a further **50-cell matrix**, five routes across ten accents:
**4,560 checked text groups, zero failures, 730 manual groups**. Manual counts
are reported as Home 1, Market 22 and Casino 50 **per accent**; those total
73 per accent and 730 across ten accents. The two other route names were not
specified in this update and are not inferred here. These are lead-reported
browser observations, not independently replayed by this reviewer.

**Zero automatically detected failures does not turn 730 manual groups into
passes.** Those groups need separate actual composed-background/paint review;
the helper's omitted-text/non-text surfaces remain outside its denominator.
The 50-cell result is additional sampled evidence, not whole-app/all-branch/
all-glass/custom-CSS/keyboard/system-motion/native acceptance. Do not merge its
counts with the earlier Settings matrix as if they were distinct unique controls.

The lead also reports launcher and Panel checks/builds passed, with **11 existing
Panel warnings attributed to CompanionBuilder**, plus an oversized-chunk warning.
Build logs and warning provenance were not independently inspected here; this is
attribution, not an independently verified clean-warning or preexisting-warning
classification. The lead's final full-suite repeat and Companion browser matrix
were still in progress when supplied. Neither is claimed complete by this review.

Final corrected-source SHA-256 identities:

| Path (relative to repository root) | SHA-256 |
| --- | --- |
| `launcher/src/components/CompanionSettings.svelte` | `b0124a74e675b7df6794880be54844f1aa9e7fc4a760673790b84b4a67c1999d` |
| `launcher/tests/rendered-accessibility-audit.test.mjs` | `62c91f3f00e169b5a7a75b9cdd7fe19d096574bf2ab6255ce34783491263ec24` |

These replace the earlier hashes only for the corrected files; historical hashes
and the initial finding remain evidence of the reviewed progression. Only this
report is modified. No gate/ledger credit, browser action or source edit is made.

## Final browser-evidence reconciliation

October 15, 2026. Read the lead's completed
`reports/design-gate-followup.md` and the new summary in
`reports/revitalization-status.md`, together with the supplied final update.
Their explicit bounded/non-native claims and unchanged 62/76 disposition are
consistent with this review. The following is **lead-reported browser evidence**,
not browser work independently performed or raw browser artifacts replayed by
this reviewer.

| Reported matrix | Cells | Checked text groups | Failures | Manual groups |
| --- | ---: | ---: | ---: | ---: |
| Initial Settings categories × accents | 100 | 3,530 | 0 | 0 |
| Final Settings repeat after correction | 100 | 3,620 | 0 | 0 |
| Home, Friends & Social, Player Guide, Market, Casino × accents | 50 | 4,560 | 0 | 730 |
| Fixed-dark HUD preview: global text × HUD accent × opacity | 24 | 312 | 0 | 0 |

The final report supplies the previously unspecified two route names: Friends &
Social and Player Guide. It describes ten accent variants (default purple plus
nine predefined swatches), **not ten independently branded server themes**.
The 100- and 50-cell matrices used a 960×600 CSS viewport, glass=false and manual
reduced motion=true; the actual system media query was false.

The **24 HUD cells** are two global text colors (`#ecedf1`, `#000000`) × four
HUD accents (black, white, default purple, cyan) × three module opacities
(.2, .88, 1). This is specifically relevant rendered corroboration of the resolved
P2: the actual preview's source no longer relies on global text. It does not
establish every widget branch/scale, browser engine or administrator override.

Both Settings totals are deliberately retained. The lead attributes the
**3,530 → 3,620** difference to asynchronous synthetic content settling
differently, not an identical workload or 90 newly independent scenarios. Do
not hide the first count, sum repeated groups as unique controls, or derive
performance comparisons from these runs. The 730 manual groups in the route
matrix remain unaccepted pending separate compositing review; a clean HUD or
Settings subset does not resolve them.

### Actual browser traversal probe, as reported

The lead now reports invoking the actual exported `auditRenderedText` against
synthetic DOM/CSS/canvas cases, rather than only Node-modeled structures:

- Black on black: **1 checked, 1 failure, ratio 1:1**.
- White on black: **1 checked, 0 failures**.
- Gradient background: **1 manual, 0 checked**.
- Aria-hidden text and input value: **0 checked, 0 manual** for each.

The last two omissions confirm blind spots, **not contrast passes**. The probe
was off viewport and reportedly removed in `finally`; this is browser traversal/
computed-style evidence, not visible-pixel verification. The new manual browser
probe improves the evidence for the helper's limited behavior, but the checked-in
Node tests still do not call its DOM traversal. Neither the probe nor a zero
automatic-failure count supplies keyboard, non-text, all-branch, full motion,
all-glass/custom-CSS or native acceptance.

The final implementation report now records 118 launcher tests, eight shared
experience tests and the lead's completed checks/builds, retaining the Panel
11-warning and oversized-chunk limitations. Those are report-attributed results;
this review's independent final test result remains **30 focused Node 24 tests**.
No broader command was rerun or raw build log independently audited here.

Rechecked SHA-256 identities of the corrected Companion component, its expanded
regression test and the traversal helper: all match the versions already reviewed.
No source change or additional independent test rerun was needed for this evidence
update. **Bounded source acceptance and P2 resolution stand; VEL-015 remains open.**
Only this review report is updated; there is no ledger or other-file edit.
