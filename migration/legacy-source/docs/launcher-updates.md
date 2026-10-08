# Launcher releases

In **Admin Panel → Settings → Launcher updates**, select the latest Windows
Setup.exe, enter the version used to build it (for example `v0.10.0`), add optional
release notes, and publish. The server accepts Windows executables within its
configured upload limit, up to 256 MB. Publication also updates the landing page's
Windows download. Use the release workflow's installer so its embedded version
matches the version entered here.

Launchers check the panel and the configured release repository at startup, every
five minutes while open, and when returning to the window. With automatic checks
enabled, a newer release produces a **New update** banner. Dismissing it hides that
version for the current session; later versions appear again. **Settings → About**
shows the installed launcher version, release notes, and manual check/install
controls. The installed version changes after installing and restarting.

The newest available version wins across the two sources; panel uploads take
priority when versions are identical. An unavailable source does not block an
update available from the other source. Downloads must match their advertised
size and, for panel uploads, their SHA-256 checksum. Repository checksums are
verified when supplied by the release API. Each uploaded file has an immutable
checksum URL, so replacing the latest release cannot change an in-progress
download. Only admins can publish; clients do not need to sign in to discover or
download an update.

The **Release** workflow stamps the input tag into the Rust workspace, launcher
package, and Tauri installer configuration, then checks that all three versions
match. About reads the compiled Rust version. The workflow creates a **draft**
GitHub release: press **Publish** for repository-based update checks to discover
it. Creating a tag alone does not advertise an update. Alternatively, publishing
its installer through the admin upload advertises it immediately to clients
connected to that panel. Repository checks use stable published releases;
prerelease installers can be offered explicitly through the panel.

Existing launcher versions check GitHub only. They must install a launcher with
this feature once before they can discover panel-hosted updates. Publish the first
such release on GitHub to reach those clients.

## Interface previews

These previews use a disposable local panel and the launcher's browser fixtures;
the uploaded 256-byte file is a test fixture, not an executable installer.

[Historical screenshot retained in the private source archive.]

[Historical screenshot retained in the private source archive.]
