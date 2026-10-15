# Bounded independent final accessibility review

## Authority, scope and verdict — October 13, 2026

This is the separately authorized reviewer pass requested by the lead/user under
the attached master plan, not implementation self-review. Read `AGENTS.md`,
`MONOREPO.md`, `docs/BOUNDARIES.md`, `docs/VALIDATION.md`,
`docs/DEPLOYMENT_TARGETS.md` and `reports/revitalization-75-review.md`. Consulted
`reports/completion-checklist.md` for the fixed VEL-017 checkpoint wording.
Used this checkout directly. Existing unrelated working-tree changes belong to
other work; they are not authored or certified by this reviewer.

**VEL017 / VEL-017's bounded independent accessibility review checkpoint is
supported after the A1/A2 fixes, at the source and synthetic-fixture level.**
The final Settings navigation and system reduced-motion changes were rechecked
against current source, rather than inheriting the earlier eight-test result.
No new confirmed blocker was found within these bounded checks. This is not full
WCAG conformance, native acceptance, release approval, or acceptance of every
accessibility checkpoint. No percentage or aggregate completion count is awarded;
the checklist and prior report remain unchanged.

Only this report is authored. No production code or tests were changed, no shared
or independent browser was used, and no commit, branch, build, deployment, live
store, native launch or infrastructure action was performed. Current raster
snapshots have mismatched stale frames: **no screenshot or visual-snapshot credit**
is granted, and historical browser observations are not reused as final evidence.

## Independently executed checks

Runtime command: `"$HOME/.npm/_npx/387698761821791d/node_modules/node/bin/node" --version`
returned **v24.21.0**, exit 0.

From `launcher/`, executed exactly:

```sh
"$HOME/.npm/_npx/387698761821791d/node_modules/node/bin/node" --experimental-vm-modules --test tests/action-contrast.test.mjs tests/operation-presentation.test.mjs tests/accessibility.test.mjs tests/settings-navigation.test.mjs
```

| Suite | Passed | Failed / cancelled / skipped / todo |
| --- | ---: | --- |
| action-contrast | 2 | 0 / 0 / 0 / 0 |
| operation-presentation | 16 | 0 / 0 / 0 / 0 |
| accessibility | 8 | 0 / 0 / 0 / 0 |
| settings-navigation | 2 | 0 / 0 / 0 / 0 |
| Total | 28 | 0 / 0 / 0 / 0 |

Exit **0**; Node reported duration **831.793667 ms**. The VM Modules experimental
warning was emitted; it was not a test failure. No nonexistent test filename was
included. No broad `npm test`, typecheck, lint, build or native suite is credited.

Also executed a read-only Node module evaluation of
`actionContrast('#6d6af5')`: background `#6d6af5`, foreground `#000000`, computed
ratio **5.030370098630986:1**, exit 0. This is numerical helper output, not a
rendered/computed-style measurement.

## Final source findings and evidence limits

### A1 — resolved for the bounded Home filled action contract

`Home.svelte` composes `OperationStatus`. `action-contrast.ts` accepts opaque
three/six-digit hex, expands shorthand, falls back to `#6d6af5` for unsupported
input, linearizes sRGB and chooses the higher-contrast black/white foreground.
`OperationStatus` derives the pair from allowed custom or manifest accent data,
binds both local action CSS variables and uses a solid background in normal and
hover states. Its scoped `.primary` rule overrides the global gradient rule by
source specificity. The default mathematical contrast now exceeds 4.5:1.

The two passing tests exercise the default and enumerated custom/server-compatible
hex colors, all 256 grayscale values and malformed/transparent/non-hex fallback
inputs. This does not certify all rendered theme states, unrelated primary
buttons, borders/icons, focus-ring contrast or compositing over artwork.

### A2 — resolved at guard/helper and source-wiring level

The Home operation window listener checks editable ancestors (including
contenteditable), an active `role=dialog`/`aria-modal=true` surface and an inert
operation ancestor. `launchShortcutAllowed` additionally rejects repeating,
consumed and composing events; Home view, account and `view.canStart` remain
required before launch. The passing shortcut test covers each rejection flag and
Ctrl/Meta admission. CommandPalette's Enter handler prevents default, and its
input is editable; DialogSurface supplies the modal marker. Thus the earlier
background shortcut bypass is addressed by actual source, not merely inert CSS.

The helper fixture does not dispatch through mounted Home/palette listeners or
assert native IPC call counts. Such end-to-end acceptance remains open.

### Settings — keyboard semantics and propagation rechecked

The named vertical tablist has `tabindex=-1`, avoiding an extra sequential stop.
Only the selected tab has `tabindex=0`; other tabs have `-1`. Tabs expose stable
IDs, selection and panel association, and the scrollable `tabpanel` has a matching
accessible label and `tabindex=0` for keyboard reachability. The latter is
intentional, not a defect solely because a compiler warning classifies it.

`navigateTabs` ignores composition/non-tab targets, calculates against current
tab controls, prevents default and **stops propagation before focus/click** for
handled Arrow/Home/End keys. The two helper tests pass for both axes, wrapping,
Home/End, one item, unrelated keys and invalid/removed indices. Production Settings
SSR fixtures pass for tab/panel markup, switch labels/help and both reduce-motion
values, key-binding names, account-dependent categories and loading status text.

SSR does not run the browser handler. Actual propagation suppression, roving
focus/selection updates, compact layout orientation and removal of a selected
account-dependent tab still need mounted/native keyboard acceptance.

### Focus, Modal, CommandPalette and reduced motion

`focus.ts` dynamically excludes hidden/inert/disabled/disconnected/negative-tabindex
controls, initializes focus, captures Tab/Shift+Tab and Escape for only the top
dialog, contains focus escape, restores prior inert state and cleans up listeners.
Five passing synthetic focus tests exercise cycling, opener restoration, dynamic
controls, latest close callback, nested overlays, out-of-order teardown and
destroy-before-microtask. These run production helper logic against a custom
ElementFixture, not a browser accessibility tree or native focus engine.

Modal and CommandPalette use the labelled modal DialogSurface/focusTrap. Palette
source exposes a named combobox, controlled listbox, active descendant, selected
options excluded from sequential Tab order, result status and keyboard help;
composition is ignored. Source constrains overlay height and scrollable bodies,
with compact-width/height rules. These are inspected semantics/CSS, not independently
mounted palette behavior or raster evidence.

Both the manual `.reduce-motion` rules and system
`prefers-reduced-motion: reduce` rules cover elements and pseudo-elements, near-zero
animation/transition duration, one animation iteration, zero delays and automatic
scroll behavior. The prior system casino exception is absent. The eighth
accessibility test checks these rules and bounded overlay/Settings compact CSS.
This does not prove OS preference integration, every animation (including JS,
canvas, media or gameplay), zoom/reflow, or physical keyboard usability.

## Remaining native / WCAG acceptance gates

- Test the exact packaged native webviews on supported OSes with physical keyboard
  and screen readers: names/roles/values, live status/progress announcements,
  combobox/listbox active-descendant reading, focus order and restored/removed
  openers, nested/no-focusable dialogs, and Escape/Tab without keyboard traps.
- Dispatch Ctrl/Meta+Enter from editable content, palette and nested modals and
  verify zero background launch IPC; exercise Settings Arrow/Home/End propagation,
  category changes/removal and focus continuity in the mounted application.
- Measure actual rendered normal/hover/focus/selected/error/custom-theme contrast,
  including translucent/image-backed surfaces, focus indicators and non-text
  controls. The action helper is not whole-product contrast certification.
- Verify supported minimum-window layouts, text scaling/zoom, reflow and focus
  visibility/not-obscured behavior; inspect clipped content and target sizes.
  Source media rules and stale frames do not establish these outcomes.
- Exercise OS and in-app reduced-motion preferences on the final native artifact,
  including programmatic scrolling, background media and other animation systems;
  inspect forced-colors/high-contrast behavior and assistive technology interaction.
- Establish final source-to-built-artifact correspondence and obtain fresh,
  synthetic-data raster evidence before any visual-snapshot credit. Full WCAG
  acceptance and native release gates remain open; no blanket accessibility pass.

## Inspected source identity

HEAD: `4f639266a515c3cb05769b055a4772c4dcb89cb8` plus existing uncommitted changes.
SHA-256 identities captured for this review; changed source requires revalidation.

| Path | SHA-256 |
| --- | --- |
| launcher/src/pages/Home.svelte | `63564b3adc4e3fe52b034a77ee9842e948b1837bc2a058cc67556d43a4bfa278` |
| launcher/src/components/OperationStatus.svelte | `ec532c8dc696cadae6ccce3bb5a3993e1ba7a1f35d079fea37a959be9c6c7ec1` |
| launcher/src/lib/action-contrast.ts | `9c538a4699f527161d5b1a2e4f15e458c31960f15f55cb5526764f691556b7ce` |
| launcher/src/lib/operation-presentation.ts | `88144b85c4e3b8ae517891b1cac302570ff8d360dc17b86f7863ec91ea38bcd5` |
| launcher/src/pages/Settings.svelte | `7894e4a64286104a71e4698e877d0ac1816f4b5603363a255e2ca1b4ccc2d283` |
| launcher/src/lib/settings-navigation.ts | `343b1b901a6ffa98f91da4c94e5a1086e06835920d7aa90a165b31253e92e8e1` |
| launcher/src/lib/focus.ts | `33e74e20fe36081f8a3f1146baad7b8322232f73e40c574ddc956f8c36236f5a` |
| launcher/src/components/ui/DialogSurface.svelte | `57e20c1d37e9dbc9584ddaf4112a4c06604fac620a8ac3af9254e05a39859536` |
| launcher/src/components/Modal.svelte | `faa8480a8a231a7dd2894d3873084043c75e7e1f2858c2003684dcd786ac3811` |
| launcher/src/components/CommandPalette.svelte | `6ca4febd911fa5559b2b2bb76ca8cef55a7fb9d08a4f57b13f3ed38a945fab42` |
| launcher/src/app.css | `8a567a21bebf309c4a1e3e8f1e82e827fe29655ef0c15d396298aae521e22c5f` |
| launcher/tests/action-contrast.test.mjs | `d8919e007fc556c5aa3b2937eb323fa05fbd62a78654503f86a27ef0d966e7ed` |
| launcher/tests/operation-presentation.test.mjs | `620c5c14f78a2cccb1c27c01b22a496a04b2f9a29041801bf1017be45a69e701` |
| launcher/tests/accessibility.test.mjs | `036abe05d22d45af0665044000eea480af91607d94303ae481bd191d07424f93` |
| launcher/tests/settings-navigation.test.mjs | `a4194760e55033c8e8eaccb8637b83eb6b205bf283d4b5add2824c962f367be4` |

## Changed paths

- `reports/accessibility-final-review.md` only.
