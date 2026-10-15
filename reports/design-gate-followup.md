# Rendered design-gate follow-up

October 15, 2026. This is implemented frontend work and bounded rendered
verification, **not a newly completed master-plan checkpoint**. The fixed ledger
remains **62/76 (81.58%)**. At least 85% needs three whole checkpoints (65/76).
See `85-percent-acceptance-audit.md` for the independent feasibility assessment;
supported native hosts, installed upgrade/restore and successful disposable
game workloads are not supplied by a browser fixture.

## Implemented fixes

The current source was exercised through the production app in
`launcher/tests/domain-browser.html?mock=main`, using synthetic accounts and IPC.
The initial retained tab had outlived its dev server: navigation first exposed
the existing lazy-load failure fallback, then the missing server. A fresh owned
Node 24 Vite server and full navigation restored the fixture. That fallback is
not evidence of a production package-loading regression.

- Notification counts were white on default danger red: measured contrast 3.91.
  Theme application now computes an opaque danger background/foreground pair
  with the existing contrast helper, including a safe malformed-color fallback.
- News tags/links were default accent on the panel surface: measured 4.25.
  They and global links now consume the semantic information-text token.
- Active account labels, social playing status and guide categories/commands
  used unmodified accent text. Observed ratios included 3.77, 4.25 and 3.69.
  They now use semantic text; shared Player Guide retains a CSS palette fallback
  rather than depending on launcher code or requiring a host token.
- Default avatar fallback initials included white on a username-derived green:
  measured 3.71. Fallback hue/saturation remain unchanged, with lightness reduced
  from 40% to 30%; a mathematical regression covers all 360 generated hues.
- Marketplace Buy now badges were muted on a raised surface: measured 4.38.
  Regular labels now use normal text, and auction labels use semantic text.
- Casino selected tabs were white on default accent: measured 4.17. They now use
  selected-surface/normal-text tokens with a fallback for other hosts. Palette
  selected icons also use the existing contrast-safe filled-action pair.
- Companion preview headings now blend the supplied HUD accent toward local
  white, not global branding text. Independent review caught a light-theme
  counterexample in the initial global-text blend; the correction and regression
  cover the fixed dark scene, accent extremes and supported opacity settings.
  This changes launcher preview styling, not the actual Minecraft HUD or IDs.
- Keybinding controls were editable while the whole group was dimmed to 55%
  opacity. They remain editable at full opacity, with explicit text explaining
  that the switch controls application to Minecraft rather than editing.
- Guild Wallet balance animation now honors both the current root preference
  and system reduced motion before scheduling and on each subsequent frame.
  A live preference change settles to the exact target without another frame;
  hidden-document behavior and effect cleanup remain intact.

## Repeatable audit and rendered results

`launcher/tests/rendered-accessibility-audit.mjs` exports the actual sRGB contrast,
alpha compositing and large-text threshold calculations, plus a browser-only
text audit. Browser usage on the synthetic fixture:

```js
const { auditRenderedText } = await import('/tests/rendered-accessibility-audit.mjs');
auditRenderedText(document.querySelector('main'));
```

It examines direct nonempty DOM text groups, excluding SVG, hidden/inert and
disabled content. It resolves CSS colors through canvas and composites ancestor
background colors. Gradients/images, ancestor opacity, filters/backdrop filters,
blending, text shadows and nonstandard text fill are sent to `manual`, **not
counted as passes**. It is not an accessibility tree, screenshot/pixel analyzer,
axe audit, focus/border/placeholder checker or general WCAG certification.
Sibling overlap, pseudo-elements and arbitrary custom CSS require manual review.

An actual browser DOM/CSS/canvas probe of the exported traversal found one 1:1
failure for black-on-black text, one pass for white-on-black and one manual case
for a gradient. Aria-hidden text and an input's value produced zero entries,
confirming documented blindspots rather than passes. The temporary synthetic
probe was outside the viewport and removed afterward; it checks traversal logic,
not visible screenshot pixels. It is separate from the Node arithmetic tests.

Final T3 browser runs used a measured CSS viewport of **960×600**, synthetic
default branding, glass **off** and manual reduced motion **on** during the
contrast matrix. The real OS media query was **false**. Ten accent values were
applied through the real `applyTheme`: default `#6d6af5` plus all nine Settings
swatches. These are ten accent variants, not ten independent server themes.
Settings and requested routes used the production store/components. After each
cell/route, rendering settled before inspection; original fixture theme settings
were restored afterward.

| Final scope | Cells | Checked text groups | Failures | Manual groups |
| --- | ---: | ---: | ---: | ---: |
| All ten Settings categories × ten accents, after the review correction | 100 | 3620 | 0 | 0 |
| Home, Friends & Social, Player Guide, Market, Casino × ten accents | 50 | 4560 | 0 | 730 |
| HUD preview: two global text colors × four accent extremes/examples × three opacity values | 24 | 312 | 0 | 0 |

The second matrix confirmed actual store views and headings, not just requested
navigation. At the default accent, manual groups were Home 1, Market 22, Casino
50; their repeated ten-accent counts explain 730. No pass is inferred for these
groups or for all page branches. Earlier exploratory requested-route cells are
not counted because some requests can redirect or resolve different content.
The last matrix directly exercised global black/light text, black/white/default
purple/cyan HUD accents and opacity .2/.88/1 against the actual fixed scene.
The first Settings run checked 3530 groups; asynchronous synthetic content
settled differently in the final repeat (3620). Counts are retained honestly,
not treated as identical workload timing or independent scenarios. All matrices
count repeated rendered text groups, not thousands of independent scenarios.

Actual semantic keyboard actions also verified: Settings End focuses/selects
About, Home returns to Accounts, and ArrowDown selects Skin & cape with one
tabstop. Focus outline was a solid 2px semantic ring. At 960×600, main, category
tabs and content had matching client/scroll widths. The command palette focused
its named combobox, exposed a valid active descendant and 40 options; Shift+Tab
wrapped to Close, Tab returned to search, and an unmatched query removed the
active descendant, exposed a disabled empty option and announced zero results.
Escape removed the dialog/inert markers and restored Quick search focus. These
are browser observations, not native keyboard/screen-reader acceptance.

## Validation and remaining gates

Node **24.21.0**: launcher **118 tests passed**, Svelte **0 errors/0 warnings**,
rune check and production build passed. Shared experience tests **8 passed**.
Panel check/build passed for the shared consumers; the check reports **11
warnings in one existing file**, and the build retains its oversized-chunk
warning. Neither warning category is claimed fixed. Boundary/installed-identity
check and whitespace checks passed. Raw command logs are under
`/tmp/velora-revitalization-20261014/design-gate-*`.

Tracked-source secret scan: 1535 files, six already-reviewed findings and zero
unreviewed findings. A separate scan of all 198 nonignored untracked files found
zero findings; the tracked scanner alone would omit this batch's new modules
and reports. Markdown validation checked 212 files/101 local targets with no
failures; external URLs and heading anchors are outside that check. The supplied
attachment and the audit's Downloads copy of the master plan compare byte-equal.

The motion suite tests the actual preference helper and executes extracted
Guild Wallet effect source; it is not a mounted/native component test. CSS
wiring assertions and color mathematics supplement rather than replace browser
observations. `design-gate-followup-review.md` records independent source review.

VEL-015 remains open: finish the manual compositing cases, all defined key-flow
branches/errors/modals, rendered system-motion/JavaScript animation checks and
the remaining scaling/focus/contrast review. Arbitrary server CSS, full gameplay,
all supported native webviews and installed/release acceptance are not certified.
No new dependency, lockfile, persisted identity, credential, deployment, release,
commit or branch is introduced by this follow-up.
