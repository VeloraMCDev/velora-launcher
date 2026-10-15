# Bounded visual and state validation

October 13, 2026. Synthetic `?mock=main` data only; no native application,
live Panel account, player store or game process is opened.

## Verified DOM/keyboard behavior

Final production preview on local port 1422, CSS viewport 960 by 600:

- Appearance is the sole selected Settings tab with tabindex 0; other tabs are -1.
- End moves selection **and focus** to About; Home moves both to Accounts.
- Document scroll width equals viewport width (960), a bounded no-horizontal-
  overflow observation, not a guarantee for every category or custom theme.
- Opening Quick search focuses its named combobox. Populated results expose an
  active-descendant and count; a synthetic unmatched query exercises empty state.
- Escape destroys the palette, removes inert isolation and restores Quick search
  focus. The production focus helper also has nested-owner modeled fixtures.

Settings navigation/source fixtures and reduced-motion fixtures pass in the final
98-test run. Source CSS now suppresses manual and system motion without a casino
exception. This is not OS preference/native screen-reader acceptance. Home action
contrast tests validate opaque hex colors or a safe fallback and black/white text
at least 4.5:1; arbitrary custom CSS and all other buttons are not certified.

## Final accepted raster captures

On the final continuation, the visible shared preview produced captures that
matched the inspected DOM. Each PNG below was opened and visually inspected,
rather than accepted solely from a successful save status. All use synthetic data
on the unchanged final production bundle. CSS viewport is 960 by 600; raster
dimensions are 1280 by 800. This bounded set supports VEL-015 checkpoint 3, not
native/E2E, every-theme, every-error-state or full responsive acceptance.

| State | Artifact filename | SHA-256 |
| --- | --- | --- |
| Settings Appearance | `browser-screenshot-localhost-mv6fyh0v-eca6594f.png` | `7304315b17b74f162ee57a1ad07ddd308cc593792bbb3a453af70f2767ce77fd` |
| Palette populated | `browser-screenshot-localhost-mv6g0ysz-666d6afa.png` | `4fa5ebfd2e3d3d1a4ae50da8f335bdb65e6eaa3e905b5c8e67d5ccf4902c1fd9` |
| Palette empty | `browser-screenshot-localhost-mv6g1yqj-9dd3803a.png` | `780e035a172bdbbaf1c32af5252241870b151fa9acd6b60eb016d2a87edd1f10` |
| Home ready | `browser-screenshot-localhost-mv6g310s-cdf662a1.png` | `fbd995f77670659a07a8608c0c1a79ee0ac8c0d35d6cd660ba49d0f6ec449ab0` |
| Repair preparation | `browser-screenshot-localhost-mv6g5dpo-5abd9937.png` | `6dfcb8b14d64705befeb5d8f56c7458dcb2784413c853e93d2cba68cfb0797c5` |
| Ended after cancel request | `browser-screenshot-localhost-mv6g6bh6-47ad788e.png` | `c147938e9968f21cb438b5369bc7439f5cc4ddeacf6fec8852e03413f4837840` |

Files remain under `/Users/alyssasitler/.t3/userdata/browser-artifacts`, not in
the repo. Appearance shows the theme selector, switches and interface-size control
without horizontal clipping. Palette captures show a visible input focus ring,
selected Home option, bounded scroll region and empty-result message. Home ready
shows Play/repair actions. Repair capture shows disabled start actions, Cancel,
10% current-phase-only progress and file count. Ended capture shows the qualified
cancellation message and typed repair retry. The main column scrolls vertically;
the status card/action is visible, but lower server status may be below the fold.

Preparation was exercised only through `?mock=progress`, followed by the mock
Cancel action. No actual download/game launch or native cancellation is asserted.
The separate source/fixture reviewer did not review these rasters. Full arbitrary
theme contrast, screen-reader behavior and the complete error/layout matrix remain
open; VEL-015 checkpoint 4 is not awarded from these pictures.

## Historical raster failures

The shared preview reports successful PNG saves, but visual inspection found
stale frames inconsistent with the concurrently inspected DOM:

| Local artifact filename | Intended state | Inspected raster |
| --- | --- | --- |
| `browser-screenshot-localhost-mv6ahbcp-dcf3aab4.png` | Settings Appearance | Home |
| `browser-screenshot-localhost-mv6ainpc-53b42677.png` | Populated palette | Older Settings frame |

Files live in `/Users/alyssasitler/.t3/userdata/browser-artifacts`, not the repo.
The returned images are 1280 by 800 despite CSS viewport 960 by 600; do not label
their pixel dimensions as the CSS viewport. A fresh visible tab was opened; its
resize timed out and subsequent snapshot failed. No alternative browser system
was used. A final reopening of the existing visible tab followed by a double-
animation-frame paint check timed out after 15 seconds, indicating no dependable
live paint acknowledgment. Further identical retries are not evidence.

Earlier Home rasters alone did not establish this batch's final multi-state visual
acceptance. They were not credited. A slow first preparation capture also showed
already-completed idle state, not the intended in-progress frame; it was excluded.
The accepted table above supersedes the earlier blocked raster gate without
retroactively validating mismatched files. The preview issue's root cause is not
diagnosed or claimed fixed. Successful tool status alone never earns credit.

No WCAG conformance, complete keyboard traversal, native/webview focus, installed
upgrade, assistive-technology or all-theme contrast claim is made.
