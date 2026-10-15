# Revitalization testing candidate

October 15, 2026. This snapshot is a testing candidate, not a certified production
release. Engineering completion remains 62/76; merging does not waive native,
upgrade/restore, gameplay, security or accessibility acceptance gates. See
`revitalization-status.md`, `release-readiness.md` and `compatibility-matrix.md`.

## Builds

The manual **Application testing artifacts** workflow builds the selected Git
ref without publishing a GitHub release, pushing registry images or deploying:

- Windows NSIS installer, Linux AppImage/Debian packages and macOS app/DMG.
- Launcher and Panel frontend bundles, plus the static documentation bundle.
- Tested Linux amd64 Panel and Operations Docker images, exported as compressed
  `docker save` archives with SHA-256 checksums.

Artifact names include the exact commit SHA. Container/web/docs artifacts expire
after three days; desktop artifacts use the existing seven-day retention. These
are normal desktop builds, not the opt-in synthetic `native-acceptance` binary.
Desktop packages do not receive production signing/notarization from this
workflow. The existing product/version/credential/updater identities are retained;
the version still reads 1.3.6, so identify this candidate by commit/artifact name,
not version number alone. Automatic update distribution remains unchanged.

Separate manual workflows cover application install/restore acceptance, Fabric
Core jars, Paper and Velocity jars, the Java/UI segment, mobile packages and the
Calagopus extension. Each result is independent; a green source CI job does not
mean these packaging jobs succeeded. Mobile Android requires the existing
configured signing identity; iOS requires personal signing before installation.
No new signing identity is generated or production release workflow dispatched.

## Testing safely

Use a disposable OS user or VM and synthetic accounts/instances first. These
packages retain the real installed product identity and can access an existing
store or replace an installed application; they are not a sandbox. Back up any
existing launcher data before an explicitly planned upgrade/rollback test.
Do not use the testing snapshot to validate continuity by opening your only copy
of real account/player data.

Download the artifact for your OS from the exact successful workflow run. For
containers, extract the artifact, verify `SHA256SUMS`, then load the archive:

```sh
sha256sum -c SHA256SUMS
docker load --input velora-panel-linux-amd64.tar.gz
```

On macOS use `shasum -a 256` to compare the recorded checksum. Load Operations
similarly from its own artifact. Run against new disposable volumes and local
ports, not production services/volumes; loading an image does not deploy it.
The embedded tags are `velora-panel:testing` and `velora-operations:testing`.
The Panel health response records its source revision. Frontend archives are
static build output, not substitutes for the corresponding backend/native host.

Priority checks: cold/warm start; successful install → verify → deliberate
managed-file corruption → repair → launch; cancellation acknowledgement and
retry; restart after interruption; account/panel switching; keyboard/scaling/
system reduced motion; diagnostics clipboard/export; and controlled upgrade/
restore on each supported OS. Record the exact candidate SHA and platform.

Legacy originless accounts intentionally no longer send stored Panel bearers;
fresh scoped sign-in may be necessary, and old authenticated offline caches may
be unavailable until refresh. No transparent legacy-account migration is claimed.
The cache prototype is not a production resume downloader. The full known-gate
list remains in `90-percent-gates.md`; packaging success alone earns no new gate.
