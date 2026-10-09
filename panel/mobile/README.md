# Velora player apps

The maintained Android and iOS apps wrap the hosted player panel with Capacitor 7.
The server supplies the same player features and instance branding as the browser.
Existing app identity `net.scopenet.player` is retained for upgrades. No private
gameplay source or operator origin is required to build this shell.

Use Node.js 24 and the committed lockfile. Set `VELORA_PANEL_URL` to the deployed
HTTPS player panel origin or base path before preparing either platform. The older
`PANEL_URL` variable remains compatible. `APP_NAME` defaults to Velora.

```sh
npm ci --ignore-scripts --no-audit --no-fund
npm test
npm run prepare:android
# With Android SDK and Java 21:
cd android && ./gradlew assembleRelease
```

On macOS with Xcode and CocoaPods, use `npm run prepare:ios`, then open the generated
Xcode workspace and select a simulator/device to build. Native source is generated
from pinned dependencies rather than copied from another repository.
See the [Capacitor 7 setup requirements](https://capacitorjs.com/docs/v7/getting-started/environment-setup).

The distribution target is free sideloading, without paid store accounts.
Android requires signing even for direct APK installation. The manual workflow's
`mobile-release.yml` builds and verifies a release APK using the operator's persistent
self-signed key. There is no Google Play upload or paid certificate. The operator can
run `powershell -NoProfile -File panel/mobile/setup-android-signing.ps1` once in a normal
Windows terminal. It creates a private key outside the repository, encrypts its
password for that Windows user and configures encrypted GitHub secrets through stdin.
Existing complete setup is reused; existing partial keys are never overwritten.
Back up the key and credentials securely: changing the key breaks in-place updates.
The workflow fails when signing was requested but no valid key is configured.

Source CI checks mobile configuration. Native builds remain manual. The standalone
Mobile app builds workflow produces a signed APK and an unsigned device IPA for
personal AltStore signing, using `VELORA_DEFAULT_PANEL_URL` from repository settings.
The Launcher release workflow calls these same jobs and includes both apps in its
signed release manifest. Approving that release in Operations verifies every file
and publishes desktop downloads, mobile downloads and the AltStore source together.
No app reaches players simply because a GitHub release was built.

For iOS, users can import the IPA into AltStore Classic and sign it on their own
computer with a free Apple Account. Windows is supported. Free provisioning requires
refreshing within seven days; no paid developer membership or App Store publication
is needed for this personal sideloading route. Credentials stay in that local tool.
See [AltStore Windows installation](https://faq.altstore.io/altstore-classic/how-to-install-altstore-windows)
and [Apple's free-account limits](https://developer.apple.com/help/account/basics/about-your-developer-account).
The existing home-screen player web app remains available without native signing.
Some Android regions/devices require the advanced flow for unverified sideloads;
see [Android developer verification](https://developer.android.com/developer-verification).
No workflow replaces an existing signing identity or ignores build failure.
Provide licensed distribution artwork locally; private source artwork is excluded.
