# Complete application readiness

The immediate priority is a working admin/player site, desktop launcher on Windows,
Linux and macOS, and Android/iOS player applications. Repository foundation merges
are complete. Current application work preserves the full private composition
instead of removing features to produce smaller public shells.

## Maintained source

[Experiences PR #2](https://github.com/VeloraMCDev/experiences/pull/2) moves the full
application into `application/` at reviewed head
`35cc5d581907ff34ee078f20ab0ffda5dd1074d4`. Its 968 imported source files are traced
to compatibility source `2db2426648574dd374f6a5a26ca16e4f92f0864c`; existing owner
snapshot verifiers still pass, including Authentication, SDK, Panel and private
Rust/Java inputs. No source checkout outside this private repository is required
for application builds. The frozen 796-file archive remains separate and unchanged.

Two recorded adaptations preserve original source bytes and reject executable
updates without valid checksums. Existing launcher/application IDs, OS credential
store namespace, API routes, data/schema paths and volume names stay compatible.
Linux and macOS bundle configuration is explicit rather than inheriting Windows NSIS.

This is a maintained complete private application baseline. Public application
separation and credential-writer/data cutover remain separate migration work.

## Current validation

[Complete application run 37723797330](https://github.com/VeloraMCDev/experiences/actions/runs/37723797330)
checks imported source ownership, both complete web builds, the full backend/core/
private-domain/storage test suites and Windows/Linux/macOS native tests/installers.
All jobs passed, including Windows NSIS, Linux AppImage/Debian packages and macOS
app/DMG. PR #2 merged as `0799b7359a8c04c327ba18aeccf08926e2bfe61e`.
CI artifacts identify the tested PR merge revision separately from its reviewed head.

[Experiences PR #3](https://github.com/VeloraMCDev/experiences/pull/3) merged as
`0bec1515a2e90cf4c0dec2883987ef4fdb86fc4a` after every native and application check passed. Its
[installation/restore run](https://github.com/VeloraMCDev/experiences/actions/runs/37728343075)
starts the actual backend against isolated synthetic data, verifies admin/player
permissions and persistent credentials across restart, takes a cold backup, and
checks a separate restored directory against file hashes and API state. It never
opens an operator database. This demonstrates fresh-install and cold-restore
behavior; it does not certify a live database upgrade or an operational backup policy.

A local online engine test also installed Minecraft 1.21.1 with Fabric, verified
79 classpath entries and launch arguments, and ran the downloaded Java 21 runtime.
This is installation evidence, not a real game session or client/server admission test.

Both full web applications also pass local production builds. Launcher typecheck
has zero errors/warnings; the admin/player site has zero errors and 16 existing
warnings. The private backend compiled from this workspace and passed real-listener
checks using fresh synthetic data: health, admin login, instance creation, player
creation/login, player denial of admin APIs, manifest visibility and static SPA serving.
Browser checks confirmed the admin instance/player views, player login into the full
site, no player admin link, and the native player layout in a 390-by-844 same-origin
frame with no horizontal overflow. This layout check is not physical-device testing.
Missing avatar/map resources in a fresh fixture use their existing empty fallbacks.

The initial runtime fixture timed out on workspace storage; the same freshly built
application passed with an isolated local temporary data directory. Development
SQLite data must use storage supporting its locking requirements; do not use a shared
data directory or start parallel credential writers to work around storage problems.

## Android and iOS

[Panel PR #2](https://github.com/VeloraMCDev/panel/pull/2) merged as
`504575299e422d5832589616bd503f64f0393776`. Its independent Android debug APK and
iOS simulator builds passed [run 37723272280](https://github.com/VeloraMCDev/panel/actions/runs/37723272280).
App identity `net.scopenet.player`, player routing and legacy URL settings are retained.
HTTPS is required except explicitly opted-in loopback development. No private source
or operator URL is a public build dependency.

The operator subsequently requested unsigned-only releases, with no developer accounts
or signing keys. [Panel PR #3](https://github.com/VeloraMCDev/panel/pull/3) merged as
`ab8bef3f350a705bccc29f98da94650af552e27d` and builds unsigned
Android release APKs and iOS device IPAs, alongside the simulator app. All three mobile
jobs passed [run 37724598445](https://github.com/VeloraMCDev/panel/actions/runs/37724598445).
These PR artifacts use a generic example origin for build validation. Manual builds
accept the actual hosted HTTPS origin before preparing native projects.

Downloaded artifacts also pass container-format checks: the 3,107,605-byte APK
contains `AndroidManifest.xml`, and the 718,186-byte IPA contains
`Payload/App.app/Info.plist`. Neither archive contains embedded signing files.
Their SHA-256 checksums are respectively
`39b72377b0029c7867ff67369439eb43fa9c8760db42850f79c7311db943be08` and
`6fe1171278ea6d93a3b0a8263b8fdd09bdda206501afd820a02dd8d78ffa83ce`.
These checks supplement successful native compilation; they do not certify
installation, code-signature parsing or real-device operation.

Neither mobile OS installs genuinely unsigned device artifacts. The existing manifest,
touch icons and standalone player mode support home-screen web use without native
developer accounts. See [Android's installation requirements](https://developer.android.com/studio/publish/preparing)
and [Apple's device/simulator requirements](https://developer.apple.com/documentation/xcode/running-your-app-on-simulated-or-physical-devices).
These unsigned artifacts remain available; no store publication is attempted.

The operator clarified that free native sideloading is acceptable.
[Panel PR #4](https://github.com/VeloraMCDev/panel/pull/4) merged as
`5bdd24a2f681103bb1657f60616f7c850886f015` after Android signature verification,
iOS device/simulator builds and configuration checks passed. The workflow supports
an existing persistent Android self-signing key through encrypted Actions secrets.
The operator completed the local setup; key material and passwords were not read
by the coding agent or committed. Disposable CI signing fixtures exercise the same
release signing path without using the distribution identity on pull requests.

[Hosted-origin release run 37730471262](https://github.com/VeloraMCDev/panel/actions/runs/37730471262)
passed all jobs with persistent Android signing and a verified existing HTTPS player
site. Android's release signature verification passed. Downloaded APK and IPA checks
confirm preserved application identity, Velora native display name, the expected
HTTPS player route and cleartext disabled. Their SHA-256 checksums are
`fec81c861c749cf384de1f0e9928aa9558d01d5cec68933f7ee2d0afcc40fbc2`
(3,153,459-byte APK) and
`63db46c225fd662722dba4908ffaa0415a0a51652a130ba7534f0a6c01a870f1`
(718,187-byte IPA). Physical-device installation remains unverified.
The current hosted site retains legacy
branding; this acceptance build does not perform the Velora deployment cutover.
The operator origin is supplied only as a workflow input, not a public source default.
The iOS device IPA remains unsigned for local provisioning. Free Apple Account
provisioning via AltStore Classic requires periodic renewal; no paid developer
membership or store submission is required for that personal sideloading route.
See [Apple account capabilities](https://developer.apple.com/help/account/basics/about-your-developer-account)
and [AltStore Classic on Windows](https://faq.altstore.io/altstore-classic/how-to-install-altstore-windows).

## Outstanding gates

- Preserve private optional composition while finishing public Panel/Launcher host boundaries.
- Verify existing-data upgrades, game-server/client operation and whole-stack restoration
  against real deployment fixtures before replacing an existing installation.
- Verify installation and application flows on physical mobile devices; hosted-origin
  native builds and artifact inspection have passed.
- Complete deployment guide execution/rollback/service onboarding. Development control
  plane and Hermes heartbeat milestones already passed; full application deployment,
  production cutover and whole-product release acceptance are not claimed here.
