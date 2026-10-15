# Rendered accessibility follow-up

October 14, 2026. Production frontend source served through Vite on local port
1420, with `/tests/domain-browser.html?mock=main` and synthetic IPC/data only.
No native launcher, real account, game, player store or live Panel was used.
This follow-up fixes concrete rendered findings; it does not close the complete
VEL-015 acceptance matrix or change the **62/76 (81.58%)** ledger.

## Findings and fixes

1. Selecting the built-in pale accent `#e5e7eb` rendered its selected check mark
   white on `rgb(229, 231, 235)`. Selection marks now use the existing tested
   black/white contrast helper. Actual computed color after the fix is black.
2. Shared filled primary buttons used unconditional white foregrounds and a
   lightened accent gradient; Home's isolated action fix did not cover these.
   Theme application now derives safe filled-action background/foreground and
   highlight tokens. The gradient moves away from the chosen text color rather
   than always toward white, retaining contrast through normal/hover endpoints.
   The actual Add account dialog's enabled Sign in action under the pale accent
   renders black text on the pale gradient. Synthetic input was not submitted.
3. At the supported 130% interface scale and 960-by-600 viewport, the last
   Settings category extended below the viewport (About bottom 628.46 px).
   Categories now have their own vertical scroll region and do not shrink their
   keyboard targets. After the fix, End selects/focuses About, scrolls the tablist
   by 51.5 px and exposes the entire target (bottom 576.96 px). Home selects/
   focuses Accounts. Document width remains 960 px, without horizontal overflow.

## Executed rendered observations

- End/Home on the actual Settings tablist select and focus the appropriate tabs;
  precisely one tab has tabindex 0. The focused tab has a computed 2-px outline.
- All nine predefined accent swatches were activated through their real click
  handlers. Each exposes `aria-pressed=true` when selected and receives its
  contrast-safe foreground; theme filled-action tokens update with the selection.
- Actual Add account dialog initializes focus on its labelled close button and
  isolates the background with inert. Escape removes the dialog/inert isolation
  and returns focus to Add account. These observations are bounded, not a full
  forward/reverse nested-dialog traversal or native screen-reader review.
- The real Reduce motion switch applies the root class. Sampled background and
  control animation/transition durations become `1e-06s`, scroll behavior auto.
  Browser system preference was false: no new rendered system-preference credit.
- Synthetic avatar URLs intentionally do not resolve; their network failures are
  disclosed, not mistaken for a real Panel outage. No screenshot/raster acceptance
  is claimed from the browser snapshot's differing CSS/raster dimensions.

## Regression coverage and scope

`action-contrast.test.mjs` now checks gradient samples between every supported
accent and its 12% highlight, plus shared theme/CSS/swatch wiring. The existing
Settings SSR fixture imports the actual contrast helper rather than mocking its
result. A new source contract retains scrollable, nonshrinking Settings tabs.
Final full frontend tests pass **101/101**. Typecheck has **zero errors and zero
warnings**; runes check and production build pass. The first broad run exposed
the SSR fixture's missing contrast-module link; the fixture now imports the real
helper, and the final run passes. Logs stay in `/tmp/velora-revitalization-20261014/` under
`accessibility-followup-*`.

Separate `authority-accessibility-review.md` accepts the bounded source fixes
after independent focused tests. Its frontend repeat used Node 25, not the
prescribed Node 24 used by the lead. It did not independently repeat browser
observations or award the full accessibility checkpoint.

Arbitrary administrator custom CSS, every gameplay/social page, every modal/error
state, OS-level reduced motion, native keyboard/screen readers and full contrast/
focus/WCAG conformance remain unaccepted. Other custom-colored gameplay controls
are not automatically certified by fixing shared primary actions. No dependencies,
lockfiles, IPC names or installed identities change in this slice.
