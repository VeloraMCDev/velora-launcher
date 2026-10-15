# Rendered frontend-domain and reconnection acceptance

October 13, 2026. Current production `App`, store, sidebar handlers and deferred
pages run in the shared T3 browser, tab `tab_c`, at 1280×800. New test-only
`launcher/tests/domain-browser.html` installs a synthetic in-memory Tauri-shaped
IPC facade before importing the real main entry. The existing mock supplies only
synthetic accounts/instances. No native launcher, actual Panel account, credential
store, JVM, game or installed data is opened. Public illustration URLs can still
load as ordinary browser assets; this is not a network-isolation certification.

## Observed checks

| Flow | Observed result |
| --- | --- |
| Fresh fixture boot | `ready=true`, `offline=false`, `view=home`, selected `survival-smp`; synthetic Alex_Miner/Steve accounts; production Home rendered |
| Rejected `refresh_manifest` | Actual store catch path sets `offline=true`; accounts, manifest instance IDs and selected instance are unchanged; Home remains rendered |
| Settings navigation while disconnected | Actual sidebar DOM handler sets `view=settings`; deferred production Settings mounts ten named tabs and account content; offline marker rendered |
| Successful reconnect in Settings | Actual refresh changes `offline=true → false`; Settings view, selected instance and accounts remain unchanged; tablist remains mounted |
| Stats navigation | Actual handler sets `view=stats`; production deferred page renders Stats & Leaderboards and its synthetic empty-state text; expected stats/economy IPC calls recorded |
| Market navigation | Actual handler sets `view=market`; deferred page renders Market & Auctions and five synthetic listings; expected listing/activity IPC calls recorded |
| Failure/reconnect while Market remains mounted | Offline becomes true then false, same view/selection/accounts persist; Market heading stays mounted; event listener count stays five before/after |

The fixture routes commands through the real frontend wrapper/production store,
records command names only and handles event subscription IDs. Failure injection
affects only `refresh_manifest`; it does not pretend to exercise backend auth or
native I/O. Identity/context/stale-event behavior also remains covered by the
existing production-store fixtures in the passing 98-test launcher suite.

### Reproduce

Run Node 24 `npm run dev -- --host 127.0.0.1 --port 1422 --strictPort` in
`launcher`, then open `/tests/domain-browser.html?mock=main` on that local server.
The fixture exposes only this test page's `window.veloraDomainFixture`:

```js
const before = window.veloraDomainFixture.state();
window.veloraDomainFixture.setDisconnected(true);
await window.veloraDomainFixture.refresh();
const disconnected = window.veloraDomainFixture.state();
window.veloraDomainFixture.setDisconnected(false);
await window.veloraDomainFixture.refresh();
const recovered = window.veloraDomainFixture.state();
```

Use actual sidebar DOM buttons for Settings, Stats and Market between these
checks, then inspect production headings and state. No direct store-field mutation
is used to manufacture the offline transitions or selected routes.

## Automation limitations and credit

T3 snapshot capture failed at the preview client. A semantic click returned
without changing observed state; inspection confirmed the intended button was
enabled, then its actual DOM `click()` handler was invoked through the shared
tab's evaluate tool. An initial requestAnimationFrame-based check timed out;
navigation reset it and the later checks await store promises without that
unreliable frame wait. These failed attempts are not passing interaction or
raster evidence. Results above come from exact state plus rendered DOM reads.
No new screenshots, visual polish, hardware keyboard, layout/WCAG, native bridge,
real server reconnection or native launch acceptance is claimed.

This closes **VEL-008 checkpoint 4**, specifically frontend render/navigation
and synthetic reconnection acceptance. It does not close VEL-013 native startup,
VEL-015 full design-system review, VEL-016 native UX or VEL-018 native feature
gates. The test page is not a production Vite build entry, and its synthetic IPC
facade must not be included in shipped bundles. No production source is changed
by this fixture.
