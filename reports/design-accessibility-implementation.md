# VEL-015 / VEL-017 bounded implementation — October 12, 2026

## Final lead follow-up — October 13, 2026

The handoff below is historical to the worker slice. Final source now has roving
Settings tabs/labelled tabpanel, Home/End/arrows, tabindex -1 on the tablist and
handled-event propagation isolation. System reduced-motion no longer exempts
casino motion. Final 98-test/check/runes/build run passes with zero Svelte warnings.
Home action contrast and modal-safe launch shortcut findings were also fixed by
the lead. `visual-state-validation.md` records verified DOM/keyboard checks,
rejected stale PNGs and six final accepted DOM-matched, inspected state captures.
These support bounded visual snapshot credit, not complete rendered acceptance. Final independent
re-review is now recorded in `accessibility-final-review.md`: 28 focused tests,
bounded source/fixture acceptance, no native/WCAG/raster certification.
Older counts/warnings/source descriptions below do
not certify this later candidate.

## Scope and provenance

Worked directly in the current checkout. No branch, worktree, commit, dependency/lockfile change, build, deployment, native application launch or live-store access. Existing unrelated changes, including Settings persistence changes, remain intact. App, Home, CrashDialog, store, IPC, identifiers and domain modules are not edited by this slice.

Read the supplied root AGENTS instructions, MONOREPO.md, docs/BOUNDARIES.md, docs/VALIDATION.md, docs/DEPLOYMENT_TARGETS.md, reports/completion-checklist.md and the existing frontend audit. No deeper AGENTS files were found under launcher or reports. A separately attached master plan was not present in this task context or discoverable in the checkout; the checked-in VEL-015 / VEL-017 checkpoint definitions and explicit user scope govern this bounded work. Do not represent the separate master plan as read.

The real global stylesheet is **launcher/src/app.css**, imported by launcher/src/main.ts. This discrepancy was reported to the lead, who explicitly assigned app.css to this slice. No style.css was created.

## Changed paths and behavior

| Path | Change |
| --- | --- |
| launcher/src/app.css | Theme-derived semantic success/danger/warning/info/busy text, neutral/selected surfaces, focus ring and disabled-opacity tokens. Visible keyboard rings also cover links/tabindex targets. Both manual and system reduced-motion rules remove delays and smooth scrolling while retaining the existing system-only casino exception. |
| launcher/src/lib/focus.ts | Production Svelte focus-trap action: initial focus, current visible/enabled focus targets, forward/reverse Tab, Escape, programmatic containment, inert background isolation, nested top-layer ownership, teardown and connected-opener restoration. Existing inert/tabindex state is preserved. |
| launcher/src/components/ui/DialogSurface.svelte | Shared named modal dialog primitive using the production focus action; used by Modal and CommandPalette. |
| launcher/src/components/ui/SettingSwitch.svelte | Shared native-checkbox switch with explicit label/help association, disabled behavior, generous hit area and visible focus. Used by Settings without changing save callbacks or setting values. |
| launcher/src/components/ui/StateMessage.svelte | Shared semantic status/alert primitive using state tokens; used by palette result announcements and Settings skin/storage loading and skin errors. Text communicates state rather than color alone. |
| launcher/src/components/Modal.svelte | Named close button, shared focus behavior, bounded dynamic viewport height, scrolling body with persistent header/footer and compact wrapping footer. Public bindable open/title/width/children/footer/onclose contract is preserved. |
| launcher/src/components/CommandPalette.svelte | Shared focus behavior, named close button, combobox/listbox options with active-descendant/selection semantics, polite result counts and zero-result state. IME Enter is ignored; ArrowDown cannot produce a negative cursor. Compact viewport bounds, visible input focus and wrapping help footer. Existing commands/navigation filtering remains unchanged. |
| launcher/src/pages/Settings.svelte | Named category navigation/current category, labelled focusable scroll region, shared switches/statuses, pressed state for choice buttons, descriptive keybinding/conflict/capture labels, profile/color/panel/folder action names and compact wrapped categories/content. Existing settings/domain/IPC behavior is retained. |
| launcher/tests/accessibility.test.mjs | Eight fixtures exercising the production focus helper and compiled production Settings/shared-switch/status rendering; reduced-motion/compact CSS contract fixtures are explicitly source fixtures. |
| reports/design-accessibility-implementation.md | This implementation/evidence/gates handoff. |

The helper searches light-DOM HTML focus targets. Arbitrary shadow-root widgets, iframe contents, native file-picker focus and all radio-group browser subtleties are not claimed by its modeled fixtures. If an opener disappears during navigation, it is not focused after destruction; remaining nested dialogs retain focus ownership. A general route-focus manager is outside this slice.

## Exact local validation

Commands run inside launcher unless noted. The prescribed Node 24 binary already existed in the local npm cache; no toolchain or project installation was performed.

```sh
NODE24=/Users/alyssasitler/.npm/_npx/387698761821791d/node_modules/node/bin/node
"$NODE24" --version
"$NODE24" --experimental-vm-modules --test tests/accessibility.test.mjs
"$NODE24" --experimental-vm-modules --test tests/*.test.mjs
"$NODE24" node_modules/svelte-check/bin/svelte-check --tsconfig ./tsconfig.json
"$NODE24" check-runes.mjs
```

- Node **24.21.0**: targeted **8 passed / 0 failed**; current full Node suite **93 passed / 0 failed**. Suite counts reflect concurrently added lead tests in this checkout.
- Svelte check: **0 errors / 1 warning**. The warning is `a11y_no_noninteractive_tabindex` on the named Settings `role="region" tabindex="0"` scroll container. The extra Tab stop deliberately permits keyboard scrolling across webviews; no misleading interactive role or warning-suppression comment was added. Independent review should decide whether this tradeoff is acceptable.
- Rune check: exit **0**. Its pre-existing local-src scope remains unchanged.
- Earlier default Node **25.6.1** runs: targeted **8/8**, then-current suite **90/90**, Svelte **0 errors / 1 warning**, rune check exit **0**. Node 24 results above supersede these as the final source-validation evidence.
- Root scoped `git diff --check` passes for the four existing owned source files. No full frontend build was run, avoiding concurrent lead bundle work.
- Focus fixtures use a synthetic DOM model and invoke the actual exported production helper. Settings fixtures compile the real Svelte component and real owned switch/status children with synthetic account/settings data and stubbed transport/icons/non-owned children. They are not native-webview/E2E tests.
- Reduced-motion CSS fixtures inspect shipped rules. They do not simulate an OS accessibility setting or establish computed CSS behavior by themselves.

Logs: `/tmp/velora-accessibility-node24-suite.log` and `/tmp/velora-accessibility-node24-check.log`. Initial fixture failures involved SSR attribute serialization, missing synthetic `running` state and Svelte snippet markers; fixtures were corrected, then the final commands passed.

## Synthetic browser DOM/style/geometry snapshots

Separate collaborative preview **tab_4**, `http://localhost:1420/?mock=main`. The lead benchmark tab was not navigated. All interactions used synthetic **Alex_Miner** data; no raster artifact or real-player data was stored.

`preview_snapshot` failed on this tab exactly twice: first text-only, then `save=true/includeImage=false`. Both returned `PreviewAutomationExecutionError`. No further screenshot retries were made. The following are actual DOM/computed-style/geometry observations, **not raster screenshots and not substitutes for raster review**. They were captured during implementation; the lead owns final-source replay and independent review.

| State | Observed snapshot |
| --- | --- |
| Palette, 1280×800 | Opening Quick search focused `input[role=combobox]`; named dialog height 510px; 40 options; five background siblings inert. Shift+Tab wrapped to the named close button. Escape removed the dialog, returned focus to Quick search and removed all applied inert attributes. |
| Appearance, 1280×800 | Settings x=187.5/y=41.25/w=1092.5/h=758.75; category width 202.5; content w=890, clientWidth=scrollWidth=890. Three switches had associated labels/help and 41.25×41.25px input targets at default 15px/rem. |
| Appearance, 640×480 | Settings x=187.5/y=41.25/w=452.5/h=438.75. Wrapped categories h=135 with internal scrollHeight=143. Content x=187.5/y=176.25/w=452.5/h=303.75, clientWidth=scrollWidth=444 and scrollHeight=481. Pane clientWidth=scrollWidth=421; active category was Appearance. No observed horizontal overflow in these owned containers. |
| Manual reduced motion, 640×480 | Semantic click on Reduce motion checked the native switch and applied root `.reduce-motion`. Computed thumb transition and pane animation duration both `1e-06s`; delays `0s`; content scroll behavior `auto`. OS media query was **false**, so this observes the manual preference only. |
| Empty palette, 640×480, manual reduced motion | Synthetic search `synthetic-no-command-match`; dialog x=20/y=152.5859375/w=600/h=174.828125; clientWidth=scrollWidth=598. Input focused, expanded=true, active-descendant absent, controls=`c1-results`; disabled empty option and live `0 results available.`. Computed dialog animation `1e-06s`, delay `0s`; five background siblings inert. |

Observed theme/token values at the default synthetic theme:

```text
--accent = #6d6af5
--success = #3fb97c
--danger = #e5484d
--warn = #e2a336
--state-success-text = color-mix(in srgb, #3fb97c 70%, #ecedf1)
--state-danger-text = color-mix(in srgb, #e5484d 70%, #ecedf1)
--state-warning-text = color-mix(in srgb, #e2a336 70%, #ecedf1)
--state-info-text = color-mix(in srgb, #6d6af5 65%, #ecedf1)
--state-selected-surface = color-mix(in srgb, #6d6af5 20%, #16181e)
--state-focus-ring = color-mix(in srgb, #6d6af5 55%, #ecedf1)
computed selected-category background = color(srgb 0.15451 0.158431 0.286274)
computed focused palette input outline = color(srgb 0.651569 0.646863 0.953726)
```

No contrast-ratio pass is inferred from token names or these samples. Arbitrary server/custom CSS may override colors and remains a separate acceptance fixture.

## Remaining gates / checkpoint ownership

- **Raster gate remains open.** Reviewer decides whether VEL-015 checkpoint 3 accepts these synthetic DOM/style/state snapshots or still requires captured raster visual/state fixtures. This report does not award checkpoints or modify the completion ledger.
- Lead must replay final rendered palette arrows/Enter/empty results, forward/reverse Tab, Escape, pointer close, repeated openings and Modal nested/close/focus cases. Existing manual palette observations are bounded, not comprehensive E2E acceptance.
- Final Settings keyboard interactions, conflict/capture behavior, long synthetic strings, small/short windows, 200% text/zoom, every category and shared primitive consumer require lead review. Native switch persistence remains covered by the pre-existing domain suite, not by SSR rendering alone.
- System `prefers-reduced-motion`, forced colors/high contrast, contrast ratios across default/custom/server themes, screen-reader announcements/reading order, shadow DOM/native file pickers, macOS/Windows/Linux webview focus/inert behavior and OS minimize/restore require independent/runtime acceptance.
- System reduced-motion retains the pre-existing casino exception; manual Reduce motion suppresses it. Other non-owned screens/components and their accessibility behavior remain outside this bounded implementation.
- No native packaging, updater/signing, installer upgrade, credential-store, game launch, release, deployment or operating-system acceptance is claimed. Preserve the existing manual release gates.
