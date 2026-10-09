# Deployment targets and minimum-cost approach

Reuse existing host capacity and avoid duplicate paid resources. Packaging and
deployment stay manual: release workflows build and test images, and an operator
deploys them from the operations dashboard. Live hostnames, addresses and keys stay
in ignored operator configuration.

## Production: single VPS

Production runs on one existing VPS behind its Traefik reverse proxy, described by
[infra/vps](https://github.com/VeloraMCDev/velora-launcher/blob/main/infra/vps/README.md).

| Component | Implementation | Hosting |
|---|---|---|
| Complete Panel | `panel/server` + `panel/web` image from the Panel release workflow | Docker on the VPS; persistent `panel-data` |
| Documentation | `docs` static bundle served by unprivileged nginx | Docker on the VPS; rebuilt from `main` by the dashboard |
| Operations dashboard | `panel/operations` image from the Operations release workflow | Docker on the VPS; SSO in front, Velora admin sign-in |
| Backups | `infra/vps/velora-backup.sh` (systemd timer) | Nightly cold archive, kept locally and copied offsite |
| Launcher installers | Launcher release workflow → GitHub release | Published to players by the Panel only after admin approval |
| Minecraft servers | Long-running Java workloads | Existing game hosts; plugins talk to the Panel URL |
| SDK/private libraries | Build inputs | No hosting |

The Panel keeps native Rust/SQLite storage; do not replace it with D1 without a
reviewed data/continuity conversion. The legacy SCOPENET Panel 1.2.2 store was
migrated unchanged (schema 46); its signing key and session secret were preserved.

## Optional: multi-host control plane

`infra/control-plane` (Cloudflare Workers, D1, R2 and Workflows) and `infra/agent`
remain the design for coordinating several hosts. They are not required for the
single-VPS deployment and are not currently in use.

## Release gates

Before deploying a build: pass source CI, build the exact image with its release
workflow (which runs the image acceptance tests, including cold backup/restore),
and deploy the pinned digest from the dashboard, which backs up first and rolls back
automatically if the new build is unhealthy. Launcher releases additionally need a
valid release signature and an explicit admin approval before reaching players.
Restore drills from offsite archives should be repeated after storage changes.
