# Guarded synthetic native acceptance harness

October 14, 2026. Preparation for real current-host Tauri/webview/IPC observations,
not automatic checkpoint credit. The fixed ledger remains **62/76 (81.58%)**.

## Safety and scope

The opt-in `native-acceptance` feature requires debug assertions; normal release
profiles with that feature are rejected. Never package a custom profile that
deliberately enables it. Its host requires an environment-selected
fixture before building Tauri. It never falls back to the installed data path.
The root must be a private canonical UUID-named direct child of the canonical OS
temporary directory, with a versioned matching synthetic ownership marker.
Private regular singly-linked files contain the existing account/settings/
manifest schemas and an already valid 32-byte fallback key. Initial settings
must reference uncredentialed loopback HTTP, and remote baked Panel/repository
configuration is rejected. Static symlinks/hardlinks are not accepted for these
guarded inputs. A bounded whole-root scan also rejects descendant links/special
files before startup texture processing. It allows at most 10,000 entries and
16 levels; this is not a no-follow defense against concurrent path replacement.
The feature uses a retained exclusive file lock rather than the
normal identity's single-instance plugin, avoiding focus of an installed launcher.

Feature-only non-test credential loading now has **no OS-keyring code path**:
it requires the existing regular fallback key and authenticates any encrypted
store with it, rejecting corruption or a wrong key. Every automatic fixture
window is configured incognito before Tauri build, with fixture-local data
directory where supported and a distinct synthetic title. macOS uses WebKit's
nonpersistent store, not the normal persistent webview store.

The default build still uses the original app-data path, keyring/service,
single-instance plugin, encryption/store formats, IPC and updater identities.
The feature does not change or impersonate those persistent identities. It is
not a hostile-filesystem sandbox, no-follow descriptor traversal or a restriction
on every command's explicit file/network arguments. Only controlled synthetic
UI actions are in scope. No real keychain/installed/player store is opened.

`launcher/scripts/native-fixture.mjs` creates an exclusive private root, synthetic
offline identity, random local fallback key and a minimal Vanilla manifest.
It serves loopback manifest data and controlled immediate/delayed instance errors;
it never returns install artifacts. Stdin controls online/error timing. Logs contain
only timestamps and allowlisted routes, capped at 2,048 entries, never headers,
bodies, URLs with queries or bearer values. Shutdown cancels timers/connections
and retains its own fixture; it does not discover or delete user stores.

## Preparation checks

- Seven focused guard tests pass: valid-root exclusion/reopen; invalid/missing/
  remote inputs with no replacement key/lock; static links; corrupt/wrong-key
  encryption; nested startup/other links; nonpersistent window configuration;
  and the actual Node-produced JSON parsed by Rust guard plus `AppState` startup.
- Feature-enabled debug native binary builds with locked dependencies and reduced
  debug information, using existing local artifacts. No package/signature created.
- Worker syntax/selftest/stdin/signal checks used Node 25 and are supplementary.
  Lead repeats syntax and selftest with prescribed **Node 24.21.0**, passing.
- Separate isolation review in `native-acceptance-isolation-review.md` accepted
  the corrected harness for bounded current-host execution. The subsequent
  actual native observations are recorded in `native-current-host-observations.md`;
  they are not complete application/loader/clipboard/OS-matrix acceptance.
- Default host suite: **138** tests; feature-enabled host: **145** (overlapping,
  not 283 independent outcomes). Default non-test check and final native debug
  build pass. The first follow-up compile exposed duplicate embedded Info.plist
  symbols in a test; it now parses the actual config rather than expanding a
  second context macro. A temporary unused-import fix was corrected; final native
  build has no warnings. No known intermediate failure is reported as passing.

Initial independent review placed launch on HOLD for encrypted-file/keyring
fallback, descendant filesystem links and persistent WebKit storage. All three
have source fixes and new regressions; the follow-up reviewer accepted the
bounded run, including specific temporary developer-container conditions. Initial HOLD remains
historical evidence, not hidden or erased.

The October 15 follow-up found activity-report warnings despite the fixture's
`send_stats: false`. This is a real privacy-preference defect, not evidence of
external delivery: the fixture logs only reviewed loopback requests and collapses
other paths to `/unmatched`. The subsequent production fix and focused tests
are documented in `telemetry-consent-implementation.md`. The original binary's
native observations do not constitute native verification of that later fix.

Source/guard/build logs are under `/tmp/velora-revitalization-20261014/` with
`native-` prefixes. Disk remains about 5.2 GiB free after the bounded build.
The harness adds no dependency, lockfile version, paid runner, automatic
deployment, signing, commit or release. Actual billed model USD is unknown.
