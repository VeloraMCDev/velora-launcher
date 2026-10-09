# Deploying Velora

## Build releases

Every deployable artifact comes from a manual workflow that builds once, tests the
exact output and publishes it immutably:

| Workflow | Produces | Tested before publishing |
|---|---|---|
| Build immutable Panel image | `ghcr.io/<owner>/velora-panel` | Image checks, instance isolation, restarts, cold backup/restore |
| Build immutable Operations image | `ghcr.io/<owner>/velora-operations` | Type checks, server tests, non-root smoke test |
| Launcher release | GitHub release `launcher-v<version>` | Launcher tests on each OS; every installer signed |

Run them from the Actions tab on `main` after source CI passes.

## Deploy builds

Open the operations dashboard's Updates page. Panel and dashboard builds are listed
by commit; deploying a Panel build takes a backup, pins the new digest in the
deployment's `.env`, recreates the service and waits for its health check. If the
build does not become healthy, the previous digest is restored automatically.
Older builds stay listed for deliberate rollbacks.

## Publish launcher releases

The launcher release workflow signs each installer with the Velora release key
(`crates/shared/release-signing.pub` holds the public half; the private half is the
`VELORA_RELEASE_SIGNING_KEY` repository secret). Releases wait on the Updates page
until an administrator approves them. Approval makes the Panel download every
installer, verify size, checksum and signature, and publish it to the launcher update
feeds and the website's download buttons. Installed launchers only accept updates
carrying a valid signature, and builds made by the workflow open the community's
Panel by default while letting players switch.

The launcher has its own version number (`launcher/src-tauri/tauri.conf.json`,
`Cargo.toml` and `package.json` must match). Bump it before running the workflow.

## Preserve state and control cost

Keep installed identities, volume names, owned SQLite stores and signing material.
Do not replace a native store with D1 without a reviewed data/continuity conversion.
Keep runtime SQL migrations; they create and upgrade real service schemas. Permit
only one writer per credential store, verify that backups restore, and record image
digests and source commits for recovery. Reuse existing machines and keep expensive
packaging manual.
