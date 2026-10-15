# Native acceptance isolation review

Date: 2026-10-14 (current host, macOS). Independent security/native-isolation
review; only this report is owned by this reviewer.

## Decision: HOLD — do not launch the native application

The opt-in guard and fixture are useful, but the current source does not yet
guarantee synthetic-only stores and zero OS credential-store access. Three
source-confirmed launch blockers follow. Passing guard tests and a debug binary
build do not discharge these findings. No native application was launched.

### B1 — An accepted encrypted file can enter the real credential store

`launcher/src-tauri/src/native_acceptance.rs:89` accepts an existing
`secrets.bin` on regular/private/single-link/size checks alone. It does not
verify that the fallback key decrypts that file. In
`launcher/src-tauri/src/secrets.rs:63`, an existing fallback whose encrypted
store cannot decrypt falls through, on a non-test macOS or Windows build, to
`keyring::Entry::new(SERVICE, "secrets-key")` and `get_password` at line 77.
The service remains the installed-client identity `net.scopenet.launcher`.

A valid marked root with a valid 32-byte fallback key and a private, regular
`secrets.bin` containing corrupt bytes therefore passes preflight and can read
the real OS keychain during `AppState::try_new`. This requires no concurrent
path replacement. The three guard tests cannot detect this: unit-test builds
compile out this OS-keychain branch.

Required before acceptance: make credential loading under `native-acceptance`
unconditionally fallback-only and fail closed on missing/invalid keys or
incompatible encrypted data; never call the OS credential backend in this
feature. Alternatively an equally strong preflight invariant must cover all
subsequent credential initialization paths. Preserve the default build's
credential identity and recovery behavior. Add regressions for corrupt and
wrong-key encrypted files and the feature-specific no-keychain path.

### B2 — Static descendant links can escape the synthetic data root

The guard validates selected root files, but does not inspect `minecraft` or
its descendants. `launcher/src-tauri/src/state.rs:81` uses
`Layout::new(data_dir.join("minecraft"))`; startup immediately calls
`game::refresh_textures` (`launcher/src-tauri/src/lib.rs:74`).
`crates/core/src/textures.rs:33` reads the versions directory and follows jar
metadata, and `ensure` can extract textures into the corresponding textures
directory. A pre-existing `minecraft` symlink to an installed store is accepted
and can cause installed-store reads, and potentially texture writes, at startup.
An ordinary `minecraft` directory with linked descendants presents the same
containment problem. This is a static-path issue, not a hostile-race allegation.

Required before acceptance: reject unsupported pre-existing children or validate
all accepted runtime subtrees for containment and regular/private/single-link
files before startup. Cover root-level and nested directory/file links, including
the startup texture path. Review profile and other IPC-created stores under the
same policy; do not assume a UUID root makes descendants safe. Fail before
starting worker threads or writing outside the fixture.

### B3 — The macOS webview is not fixture-isolated

`launcher/src-tauri/tauri.conf.json:13` retains the normal window configuration;
neither it nor the feature builder sets incognito/nonpersistent storage or an
isolated webview data-store identity. Redirecting `AppState` does not redirect
WebKit storage. The locked local Wry 0.57.0 source
(`src/wkwebview/mod.rs:232` in the Cargo registry) selects
`WKWebsiteDataStore::defaultDataStore` without those overrides; incognito selects
`nonPersistentDataStore`. Tauri's local configuration defaults confirm
incognito is false and the data-store identifier is absent.

Consequently this feature retains the ordinary persistent browser-store path,
with no source-level guarantee against installed browser-store access. No claim
is made that installed data was actually opened: the app has not been launched.
Required before acceptance: configure the feature's webview as nonpersistent or
otherwise independently isolated *before* automatic window creation. Preserve
the default product/bundle/updater identities. Verify current-host behavior;
an `AppState` temporary path alone is insufficient evidence.

## Positive source findings

- Cargo declares `native-acceptance = []` without making it a default feature.
- The feature module contains `compile_error!` when `debug_assertions` is off.
  This is a normal-release-profile guard, not a prohibition against deliberately
  enabling debug assertions in a custom release profile.
- `from_environment` runs before Tauri builder execution and before `AppState`.
  Missing `VELORA_NATIVE_ACCEPTANCE_ROOT` fails; the feature has no app-data
  fallback. Compiled Panel/repository values, including legacy aliases resolved
  by `state::build`, are rejected when nonempty.
- Root validation requires an absolute canonical direct child of canonical
  `temp_dir`, a `velora-native-` UUID-parsable basename, private Unix directory
  permissions, and a matching versioned purpose marker. Required JSON files
  are bounded and deserialized before initialization; the key must decode to
  exactly 32 bytes. Checked files reject symlinks and Unix multiple hard links.
- Initial Panel settings must describe uncredentialed loopback HTTP with an
  explicit nondefault port and no query/fragment. This is startup validation,
  not a runtime egress sandbox.
- The production single-instance plugin remains present when the feature is
  absent. Only this feature replaces it with a retained per-root file
  `try_lock`; `app.manage(fixture)` keeps the lock alive for application state.
- The ordinary app-data selection, bundle identity, and credential service
  identity remain unchanged by the feature integration.

## Fixture script review

`launcher/scripts/native-fixture.mjs` creates a fresh canonical temporary UUID
root with mode 0700, exclusive mode-0600 seed files, a cryptographically random
32-byte fallback key, a synthetic offline account, and a synthetic manifest.
It binds an ephemeral port on `127.0.0.1`. Settings explicitly disable updater
checks and stats. The instance endpoint only returns failure statuses; the
manifest endpoint allows 200 or failure statuses. The script does not download
artifacts, accept real credentials, invoke the launcher, or use OS keychain APIs.

Unknown routes are 404. Request logs contain bounded timestamp/route entries,
not bodies, query strings, tokens, or arbitrary unmatched paths. Connection
counts and delays are bounded; cancellation/shutdown clears timers, closes
connections/server/log descriptor, and intentionally retains the fixture.
Successful selftest retains synthetic evidence only.

These are properties of this cooperative fixture, not guarantees enforced by
the native guard against arbitrary accepted fixtures. Account/manifest parsing
uses permissive production DTOs. `Settings` accepts omitted values and defaults
`check_updates`/`send_stats` to true. Remote manifest assets or Panel accounts
are not semantically rejected by the guard. The exposed `set_panel_url` IPC can
contact a nonloopback address before saving settings. If acceptance requires
loopback-only for the entire session, enforce that at runtime or explicitly
restrict the reviewed procedure; do not call this feature an egress sandbox.
Also consider requiring the exact canonical UUID spelling: `Uuid::parse_str`
alone is broader than the fixture script's lowercase hyphenated basename.

## Independent validation and limits

Read `AGENTS.md`, `MONOREPO.md`, `docs/BOUNDARIES.md`, `docs/VALIDATION.md`, and
`docs/DEPLOYMENT_TARGETS.md`. Reviewed feature source/integration, fixture source,
credential initialization, recovery, settings, startup textures, Panel IPC,
configuration, and locked local Tauri/Wry source. No internet/vendor activity,
deployment, installed-store inspection, real keychain calls, or ledger edits.

After the lead confirmed Cargo idle, independently ran:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher --lib --features native-acceptance native_acceptance::tests
node --check launcher/scripts/native-fixture.mjs
node launcher/scripts/native-fixture.mjs --selftest
```

Result: three guard tests passed, zero failed, 138 filtered out. Script syntax
and selftest passed. Reviewer Node was **25.6.1**, not Node 24; the lead separately
reports Node **24.21.0** syntax/selftest passes. Inspected the lead's
`/tmp/velora-revitalization-20261014/native-guard-tests.log` and
`native-fixture-build.log`; they report three passing tests and a completed
debug build. Build success is not native startup/IPC acceptance.

The guard tests seed an empty/default manifest rather than reading the generated
script fixture. **Actual Rust guard/DTO parsing of that generated fixture remains
unverified independently.** `FixtureRoot::open` is private and its current tests
create their own root; invoking the existing native binary to bridge this gap
would violate the launch hold. An authorized owner should add a library-level
integration test accepting a generated root, without launching Tauri and without
altering the reviewer-owned scope. Also test missing environment, branded build
rejection, normal release rejection, and the three blockers before lifting hold.

Metadata-check/open races and advisory locking are explicitly not a defense
against hostile concurrent local mutation. Unix mode/link checks do not certify
Windows ACLs, supported-OS acceptance, installed upgrades, production recovery,
signing, packaging, or game launching. Only current-host bounded synthetic
GUI/real IPC could be considered after fixes and explicit accepted guard review.

Ledger remains **62/76** as supplied; no automatic native acceptance or gate
advancement. Actual billed USD is **unknown**.

## Reviewed source fingerprint

SHA-256 at review time (re-review changes before accepting):

```text
890c65e50f242df24791fdd46f250656de4cda2273dfa1f46656402078cb726d  launcher/src-tauri/src/native_acceptance.rs
e6f595ff0481b257552170db40439c13c53053f3db8bfe6d67b98df6f2e9dac9  launcher/src-tauri/src/lib.rs
04c346f990d11e4d3f6c0e58f8d41402aefef089071ee66a1fae4ee34f370be4  launcher/src-tauri/Cargo.toml
3fd813c2ba858ad8f7e347325250cfbccdc31ca5bcdefe1475952b2fc76c8f38  launcher/scripts/native-fixture.mjs
```

## Independent follow-up: B1–B3 fixes, source review

Date: 2026-10-14. This addendum preserves the historical HOLD and findings above.
The owner reports no native launch and requests source review while the lead
runs guard/default/feature/build validation. **No tests or native binary were
run by this reviewer in this follow-up.** Explicit Cargo-idle confirmation is
required before independent test repetition.

### Source disposition

**B1 addressed in source.**
`launcher/src-tauri/src/secrets.rs:59` now selects a separate
`load_or_create_key` implementation for `feature = "native-acceptance"` and
`not(test)`, which calls only `native_acceptance::fallback_key`. The ordinary
implementation is excluded in that configuration; the credential helper is
also excluded. The only `keyring::Entry` call in launcher source resides in
the excluded ordinary implementation. Default and unit-test key behavior
remain on the original implementation, preserving installed-client identity.
`launcher/src-tauri/src/native_acceptance.rs:41` validates the fallback and
decrypts any existing encrypted store through the actual `Secrets` decrypt
routine exposed as `validate_fixture_store`. Preflight calls it before lock
creation and `AppState`. This closes the static corrupt/wrong-key store path
to the OS backend; feature tests still do not execute the non-test branch, so
its no-keychain property rests on reviewed compile-time routing plus the
pending non-test feature build, not an observed keychain test.

**B2 addressed in source.**
`launcher/src-tauri/src/native_acceptance.rs:53` walks all existing descendants
using `symlink_metadata`, allows only actual directories and regular files,
and rejects Unix multiply-linked files. Limits are 10,000 entries and depth 16.
`FixtureRoot::open` calls the walk before fallback/DTO validation, lock creation,
state initialization, and startup workers. This covers unrelated entries and
both root-level and nested Minecraft paths, not just the previously selected
seed files. The new link regression checks unchanged synthetic targets.
These checks address static containment, not hostile metadata/open races or
arbitrary future IPC behavior. Descendant files are not all checked for private
mode by this walk; root mode 0700 and explicitly private sensitive seed files
remain the current-host cooperative-fixture privacy boundary.

**B3 addressed in source for the current automatic windows.**
`launcher/src-tauri/src/lib.rs:31` creates the context, then under the feature
calls `fixture.isolate_windows` before `.build(context)` at line 210.
`launcher/src-tauri/src/native_acceptance.rs:82` sets every configured automatic
window to `incognito = true`, supplies the fixture's `webview` data directory,
and changes only the window title to `Velora Synthetic Acceptance`. The locked
Wry macOS selection reviewed above maps incognito to a nonpersistent store.
No dynamic window-builder calls were found in launcher Rust source. The default
context is not mutated. The fixture path supplements incognito; it must not be
mistaken for the mechanism that isolates WKWebView storage on macOS. Actual
current-host window/storage behavior remains a bounded native validation step,
not something these source checks claim to observe.

### New regression coverage reviewed, not yet repeated

The frozen source now contains seven guard tests: the original three plus
corrupt/wrong-key encrypted-store rejection, nested/unrelated link rejection,
automatic-window configuration, and Node-generated fixture integration.
The integration runs the actual fixture script selftest with a 20-second
deadline, reads its generated root, calls `FixtureRoot::open`, and initializes
`AppState::try_new`; it does not call Tauri build/run or contact a live Panel.
Its `AppState` initialization uses unit-test credential routing, as expected.
Once independently passed, it can close the generated-fixture Rust parsing gap
recorded in the historical review. At this stage none of these four added
tests is claimed independently passed.

### Follow-up decision and remaining limits

**Source review finds B1–B3 resolved; overall launch HOLD remains pending the
explicit-idle independent seven-test run and reconciliation of the lead's
default/feature/non-test-build evidence.** No new source blocker was identified
for the cooperative, current-host fixture and narrowly bounded GUI/real-IPC
procedure. This is not permission to launch now.

The previous runtime-egress, permissive DTO/canonical-UUID spelling, race,
custom-release-profile, and cross-platform limitations remain. A later bounded
acceptance must use the reviewed script's fresh synthetic root, disable real
sign-in/update/install/external-Panel actions, and not mutate filesystem paths
during execution. Do not promote this to hostile isolation, arbitrary-fixture
acceptance, a release matrix, or an installed-store upgrade test. No source,
test, ledger, identity, deployment, or credential data was edited by this
reviewer. Ledger stays 62/76; actual billed USD remains unknown.

Follow-up SHA-256 source snapshot:

```text
538baf3d21a9ffae7696ecf0694a68da8df6d94754e7c0e3e785ede4ab0b1906  launcher/src-tauri/src/native_acceptance.rs
59ce199eea5276f0f443071da823fb0b8f785777f9559cc844bcb635ec29c6b2  launcher/src-tauri/src/lib.rs
8b1cc319e04a119d9ad4877dc9eeee8be7b6b1fb2d63f386cadedfde84d0198c  launcher/src-tauri/src/secrets.rs
3fd813c2ba858ad8f7e347325250cfbccdc31ca5bcdefe1475952b2fc76c8f38  launcher/scripts/native-fixture.mjs
```

### Compile-only adjustment reviewed

The lead reported duplicate `_EMBED_INFO_PLIST` from a second
`generate_context!` in the window unit test. Source inspection confirms the
adjustment: `isolate_windows` now accepts a mutable `WindowConfig` slice;
production passes `context.config_mut().app.windows` at the same pre-build
point. The test deserializes the actual `tauri.conf.json` through `include_str!`
and verifies mutation of its automatic windows. Only one `generate_context!`
remains in launcher Rust source. This does not change the B3 disposition;
the test is a configuration assertion, not a native-window execution test.
Independent tests still await explicit idle, and launch HOLD remains in force.

Updated source SHA-256 (other follow-up fingerprints unchanged):

```text
c82e7cb4908a325d0cef89608cf18ec77b6c3e0396e3c4c36cdb02bd631b655c  launcher/src-tauri/src/native_acceptance.rs
e758eeadbedf67daa4918b7185c69bec4804c2c39d90cc826c5e3c91b8cd80eb  launcher/src-tauri/src/lib.rs
```

## Final independent disposition: bounded current-host guard accepted

Date: 2026-10-14. This final addendum supersedes the pending-test launch HOLD
only for the bounded procedure below; it preserves all historical findings.
**B1–B3 are resolved for that procedure. The verified feature build may now be
used for synthetic current-host GUI/real-IPC validation under these conditions.
No native application has been launched by this reviewer, and native acceptance
itself is not yet complete.**

### Independently repeated evidence

After explicit Cargo-idle confirmation, ran all seven guard tests successfully
twice: first with the reviewer's default Node 25.6.1, then with **Node 24.21.0**
explicitly prepended to PATH from its already installed local binary. Both
runs reported **7 passed, 0 failed, 138 filtered out**. Also repeated script
syntax validation on Node 24.21.0 successfully. No installation/download was
needed, and no native executable was run.

The Node-generated-fixture test passed independently on Node 24.21.0. It runs
the actual script selftest, parses the generated root with the actual Rust
`FixtureRoot::open`, and initializes actual `AppState::try_new` without Tauri
startup. This closes the previously unverified script-to-Rust DTO/preflight
integration gap. Its selftest server is stopped before Rust initialization,
and its synthetic root is removed afterward; this test does not provide a
running server/root for a later GUI session.

Inspected the lead's final logs under
`/tmp/velora-revitalization-20261014/`:

- `native-guard-fixed-tests.log`: seven guard tests pass.
- `native-default-host.log`: 138 default host tests pass.
- `native-feature-host.log`: 145 feature host tests pass.
- `native-default-check.log`: default dev check finishes successfully.
- `native-fixture-fixed-build.log`: final feature debug build finishes successfully.

No compiler warning/error lines were found in these five logs. The full suites,
default check, and non-test debug build are **lead-run, log-inspected evidence**,
not independent repetitions by this reviewer. Their scope is not a full
workspace/release/supported-OS matrix.

The final import-only change gates `base64::Engine` with
`any(not(feature = "native-acceptance"), test)` while keeping `rand::RngCore`
available for encryption nonces. Rechecked compile-time routing: feature
non-test key loading remains fallback-only; the production OS-keyring helper
and ordinary implementation are excluded. This adjustment does not reopen B1.
The bounded static tree walk and pre-build incognito mutation remain unchanged,
so B2/B3 source dispositions stand. Tests assert configuration and static
rejections; they do not prove observed WebKit storage behavior at runtime.

### Conditions of permission to run

1. Use only the verified **debug `native-acceptance`** build of the frozen source
   below; not a default binary, installed launcher, installer, or release build.
   Re-review substantive source changes before reuse.
2. Start the reviewed `native-fixture.mjs` with Node 24.21.0 in its normal server
   mode to obtain a **fresh** canonical UUID direct temporary child and live
   loopback URL. Supply that exact root as `VELORA_NATIVE_ACCEPTANCE_ROOT`.
   Never substitute an installed/root-retargeted store or reuse the integration
   test's already removed root. Native startup must still pass actual preflight;
   failure is a stop, never a reason to fall back to app data.
3. Keep seed accounts/data synthetic, stats and update checks off, fixture server
   bound to `127.0.0.1`, and all controlled requests on the fixture. Do not use
   real sign-in, OS-keychain experiments, external Panel changes, external links,
   updater/installer execution, successful game installs/launches, or arbitrary
   imported files. The script's failure-only instance endpoint supports bounded
   failure/cancellation IPC, not an actual Minecraft artifact installation.
4. Limit work to the planned synthetic GUI/real-IPC observations. Do not mutate
   fixture paths during execution; static guards/advisory locking are not a
   hostile-local-process sandbox. Keep generated screenshots synthetic. Stop
   on any evidence of installed-store access, persistent browser-store leakage,
   real credential access, unexpected network traffic, or guard failure.
5. Observe current-host nonpersistent window behavior during the bounded run
   and record actual results separately. Do not infer completed native
   acceptance from permission to run, passing unit tests, or a build log.

Only this report was edited by the reviewer. No source/tests/ledger/credentials
were changed, no native app was launched, and no paid infrastructure or
deployment was used. **Ledger remains 62/76; actual billed USD remains unknown.**
The historical non-hostile-race, non-egress-sandbox, permissive DTO/UUID,
custom-release-profile, and supported-OS limitations remain explicit.

Final reviewed SHA-256 (stable across final test repetitions):

```text
c82e7cb4908a325d0cef89608cf18ec77b6c3e0396e3c4c36cdb02bd631b655c  launcher/src-tauri/src/native_acceptance.rs
e758eeadbedf67daa4918b7185c69bec4804c2c39d90cc826c5e3c91b8cd80eb  launcher/src-tauri/src/lib.rs
4979185198192062e554641165dd5b6e29a5126d2f5991db1754a84072fd1eb6  launcher/src-tauri/src/secrets.rs
04c346f990d11e4d3f6c0e58f8d41402aefef089071ee66a1fae4ee34f370be4  launcher/src-tauri/Cargo.toml
3fd813c2ba858ad8f7e347325250cfbccdc31ca5bcdefe1475952b2fc76c8f38  launcher/scripts/native-fixture.mjs
```

## Temporary developer .app container: conditional isolation opinion

Date: 2026-10-14. The lead reports an initial bounded native process passed
preflight but Vite was absent, the fallback-show warning occurred, and no fixture
HTTP requests were observed. The lead reports stopping the owned process and
restarting Node 24 Vite; CUA cannot yet bind the unbundled process. These are
**lead-reported observations, not independently observed GUI evidence**. They
do not establish successful GUI/IPC acceptance.

**Conditionally accepted as a local development-container experiment**, not
packaging/release acceptance: a private temporary `.app` under the reviewed
fixture root, containing a byte-identical copy of the already verified feature
debug executable and minimal synthetic bundle metadata, does not itself require
changing default product compatibility identities. No container was created or
launched by this reviewer. CUA discovery/binding success is not guaranteed by
this isolation opinion; stop and report a resolver limitation if the supported
path-based binding still fails.

Source fingerprints remain identical to the final reviewed snapshot above.
Read-only `otool -L` inspection of `target/debug/velora-launcher` showed only
absolute `/System/Library/...` and `/usr/lib/...` dependencies, with no listed
relative executable-path dylib dependency. This supports relocation of these
linked dependencies; it does not prove all runtime resources will resolve.
The source executable's SHA-256 at this review is:

```text
9b5e5fa9601e8c8e76a0fa083a1b6e629538bd861d093d218ac51e4a5125a2ab  target/debug/velora-launcher
```

### Required container and launch conditions

1. Stop the previous owned native process and establish that the fixture lock
   is released before restarting; do not kill unrelated launchers or remove a
   lock to bypass a live holder. Confirm the reviewed loopback fixture server
   and expected Vite endpoint are serving before launch. Keep the session's
   synthetic settings/accounts unchanged.
2. Create the container entirely beneath the canonical private fixture root,
   before rerunning native preflight. Use actual private directories and regular
   single-link files: no symlink to the workspace executable, no hard-link copy,
   no linked framework/resource tree. The copied executable must be runnable
   but not group/world accessible. Re-run the actual native guard at startup
   against the resulting tree; adding the container is not an exemption from
   its entry/depth/link restrictions.
3. Use a minimal `Contents/Info.plist` with an exact `CFBundleExecutable` pointing
   to the copied file in `Contents/MacOS`, package type `APPL`, and an unmistakably
   synthetic name. The container-only `CFBundleIdentifier` must be exactly
   `net.scopenet.launcher.native-acceptance`, never the installed default
   `net.scopenet.launcher`. Validate plist syntax and record its relevant fields.
   Do not add document/URL handlers, services, login items, background agents,
   entitlements, privacy-permission requests, or updater/install integration.
4. Record SHA-256 of source and copied executable and require exact equality to
   the fingerprint above immediately before launch. Do not patch/re-sign the
   binary or change its embedded metadata, Tauri config/context identifier,
   keyring service, updater identities, or repository source. The synthetic
   outer bundle ID is an identity for this temporary developer container only,
   not an installed-client identity migration or evidence for release signing.
5. Launch the exact copied executable with `VELORA_NATIVE_ACCEPTANCE_ROOT`
   explicitly supplied in that process's environment. Do not assume Finder,
   Launch Services, or a CUA-triggered launch inherits terminal environment.
   Prefer binding CUA to that already-running exact container/path/process;
   verify the resolved path/process and synthetic window title. Never bind by
   the ordinary launcher name/identifier or silently attach an installed app.
   A resolver-initiated relaunch must preserve the same explicit environment
   and pass preflight, otherwise stop. Do not use global `launchctl setenv` or
   an environment-setting wrapper in place of the byte-identical executable.
6. The existing feature protections remain mandatory: fallback-only/no-keyring,
   incognito windows before build, no global single-instance plugin, retained
   fixture lock, compiled Panel/repository rejection, and no app-data fallback.
   Retain all bounded GUI/IPC restrictions from the final disposition above.
7. No Cargo bundling, signing, notarization, distribution, release, deployment,
   `/Applications` installation, explicit registration/handler changes,
   `defaults` writes, privacy/Accessibility permission changes, quarantine-policy
   bypasses, or global Launch Services resets. If OS security or CUA requires
   any such change, stop and request separate review rather than work around it.
   Stop only owned processes before any fixture/container cleanup; never clean
   installed stores or global caches as part of this experiment.

### Important interpretation of OS side effects

Apple's *Core Foundation Keys* reference documents that `CFBundleIdentifier`
identifies a bundle and participates in preferences and Launch Services lookup;
`CFBundleExecutable` identifies its executable. Its *Launch Services Concepts*,
Application Registration section, describes automatic application registration,
including Finder discovery. Those primary references support using a distinct
outer identity and avoiding handler claims, but **not** a guarantee that simply
discovering/launching a temporary `.app` leaves no OS registration, diagnostic,
cache, or preference artifacts outside the fixture. Do not claim complete
OS-state containment or guaranteed deregistration on deletion.

This approval interprets “no OS preferences changes” as no explicit changes to
OS/user settings, permissions, handlers, or global environment. If the requirement
instead means absolutely zero system-managed metadata/preference/cache writes,
the container experiment is **not accepted under that stronger requirement**;
do not create/launch it without resolving that constraint. The distinct outer
bundle ID is a collision-reduction measure, not an additional sandbox.

No additional native/GUI acceptance or ledger gate is awarded by this opinion.
Only this report was edited. Ledger remains 62/76; actual billed USD is unknown.
