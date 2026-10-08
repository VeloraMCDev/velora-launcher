# Phone apps (Android and iOS)

The [player panel](player-panel.md) is also built as a phone app, so players get an icon on their home screen, full-screen display and no browser chrome. The app is a thin native shell (Capacitor, in `mobile/`) around the player panel served by **your own panel**, so every feature, update and the branding you set in the admin panel show up in the app immediately, with no new release needed.

The release workflow builds both and attaches them to the draft release:

| File | What it is |
|---|---|
| `scopenet-player-<version>.apk` | Android app, signed. Download it on the phone and allow installs from your browser. |
| `scopenet-player-<version>-unsigned.ipa` | iOS app, **unsigned**. Apple only runs apps signed by a developer account, see below. |

> The phone-app jobs are marked `continue-on-error` in `.github/workflows/release.yml` because they have not been proven on your repository yet. If one fails, the release is still published without that file, and the release notes say so. Once a run has produced both files, remove `continue-on-error` so a broken phone build blocks the release like everything else.

## How the app behaves

The app opens straight to the **sign-in screen**, then to the player panel. There is no landing page and no launcher-download page inside the app; admins get an **Admin panel** entry in their account menu, players do not. The app recognises itself (native shell, `?app=1`, or an installed home-screen app) and also hides text selection and rubber-banding so it feels native. Sign-out returns to the sign-in screen.

## Hosting the apps on your own panel

Instead of sending players to GitHub, upload the files to your panel: **Admin panel → Settings → Launcher & apps → Phone apps**. Each upload is checked (an `.apk` must contain `AndroidManifest.xml`, an `.ipa` a `Payload/*.app`), stored in the panel's data volume and offered to players:

* In the player panel's **Launcher** page and the landing page download list.
* Direct links: `/api/v1/mobile-apps/android/download` and `/api/v1/mobile-apps/ios/download`; the current versions as JSON at `/api/v1/mobile-apps`.
* **AltStore source** for the iOS app: `https://your-panel/api/v1/mobile-apps/altstore.json`. Players open `altstore://source?url=<that URL>` (the **Add to AltStore** button does this), or paste the URL under *Sources* in AltStore / SideStore. The bundle identifier defaults to `net.scopenet.player`; set it when uploading if you re-signed the app with another one.

Uploading again replaces the file; **Remove** takes it off the website.

## Pointing the app at your panel

The apps are built with `https://scopenetmcpanel.scopedd.lol` embedded by default. To use another panel, set the repository variable `PANEL_URL` (the same one the launcher uses, **Settings → Secrets and variables → Actions → Variables**). The app opens `PANEL_URL/#/play` directly. Optionally set `MOBILE_APP_NAME` for the name under the icon (default `SCOPENET`).

If the embedded address were ever removed, the app falls back to an **address screen** where the player types the panel address once.

Use HTTPS. Android blocks plain HTTP unless the address starts with `http://` at build time.

## Android signing

Android only installs signed apps, and an update only installs over an older build signed with **the same key**. Create a keystore once and store it as repository secrets:

```bash
keytool -genkeypair -keystore release.jks -alias scopenet -keyalg RSA -keysize 2048 -validity 10000
base64 -w0 release.jks     # paste into ANDROID_KEYSTORE_BASE64
```

Secrets: `ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, optionally `ANDROID_KEY_PASSWORD` (defaults to the keystore password). Without them the workflow signs with a one-off key and warns, which is fine for a first try but means players must uninstall before each update. Keep the keystore safe: losing it means a new app identity.

## iOS

Apple does not allow installing arbitrary `.ipa` files. The workflow therefore builds the app **unsigned**. To use it:

* **Your own devices or a small group:** sideload with a tool such as AltStore or Sideloadly, which re-sign the file with a free or paid Apple ID (free IDs need a re-sign every 7 days).
* **TestFlight / App Store:** open the generated project (`npx cap add ios` in `mobile/`, then Xcode), sign it with your Apple developer account (US$99 a year) and upload it. SCOPENET does not hold any Apple credentials.

Most iPhone players will prefer **Add to Home Screen** in Safari instead (the Account page walks them through it), which needs no signing at all and gives the same full-screen app.

## Building locally

```bash
cd mobile
npm ci
PANEL_URL=https://panel.example.com npx capacitor-assets generate
PANEL_URL=https://panel.example.com npx cap add android     # or ios (macOS only)
npx cap open android                                         # opens Android Studio / Xcode
```

The generated `mobile/android` and `mobile/ios` folders are git-ignored; CI recreates them every run.
