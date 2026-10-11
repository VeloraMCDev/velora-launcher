# Velora Operations

The operator dashboard for a Velora deployment: one place to watch health, review
activity, take backups, deploy new builds and approve launcher releases.

| Page | What it does |
|---|---|
| Overview | Service health, host CPU/memory/disk, endpoint latency, certificates, backups and community totals, with 24 hours of history |
| Services | Velora compose services with CPU, memory, logs, restart and pull & recreate; unrelated containers are excluded |
| Builds | Download every published release asset and successful main workflow archive, including mods, plugins and extensions |
| Agents | Enroll hosts and monitor their resources and selected Minecraft, HTTP and TCP services |
| Activity | Sign-ins, active players, game launches and admin changes from the Panel's activity log |
| Updates | Launcher releases awaiting approval, Panel and dashboard release builds (deploy or roll back), and container image updates |
| Backups | Last backup, offsite copy, archives and "Back up now" |
| Audit log | Every sign-in, operation and alert handled by the dashboard |
| Settings | Resend email notifications, alert thresholds and an optional GitHub token |

Sign-in uses Velora Panel administrator accounts; sessions keep the admin's Panel
token, are re-validated every five minutes and expire after 12 hours. Changes need a
per-session CSRF token. The dashboard holds the Docker socket, so deploy it behind an
SSO layer as well (see [infra/vps](../../infra/vps/README.md)).

Operations run as tracked jobs with live logs. Compose commands run in short-lived
`docker:cli` helper containers against the deployment directory, so the dashboard can
also update itself. Panel updates request a backup first, pin the new digest in `.env`,
wait for the health check and roll back automatically on failure. Only services in
the Velora compose project can be changed.

Dashboard self-updates run in a detached helper that pulls the pinned image,
checks container health and restores the previous `.env` and image on failure.
The helper records `running`, `succeeded`, `rolled-back` or `rollback-failed` in
`.ops-update-result` in the deployment directory. A failed rollback retains
`.env.ops-rollback` for operator recovery and blocks another self-update.

## Development

```sh
npm ci --no-audit --no-fund
npm test             # server unit tests
npm run check        # svelte-check
npm run check:runes
npm run build        # web/dist
npm start            # needs a Docker socket and a reachable Panel
```

For UI work, run `npm start` with `OPS_INSECURE_COOKIES=1` and `npm run dev` in a
second terminal (Vite proxies `/api` to port 8080).

Release images are built by the manual `Build immutable Operations image` workflow.

## Downloadable builds

Builds discovers GitHub release assets and uploaded Actions artifacts from the
canonical repository. Results are cached for up to five minutes across viewers to
bound API usage. It refreshes the newest page and supports loading older
output. Published prereleases are labeled; draft releases, PR/fork builds and failed
runs are excluded. An output must be uploaded by its workflow or attached to a
release to appear; local build directories are not accessible to Ops. Container
images remain under Updates. Packaging, signing, release approval and installation
retain their existing independent workflows.

Workflow ZIP downloads require the GitHub token in Settings with repository
Actions read permission (and Contents read for private release metadata). Ops
rechecks build provenance and expiry before issuing GitHub's temporary download
redirect. Credentials are sent only to GitHub's API. Expired archives are labeled
unavailable; published release assets provide long-term downloads. Source failures
are displayed so a partial catalog cannot masquerade as an empty repository.

## Monitoring other hosts

Use Agents to enroll a host, add selected probes and download a Docker installation
bundle. Extract it on the host and start with Docker Compose; Node is included in
the container. Linux and Docker Desktop networking options are provided. Native
Node installation remains available for Windows/macOS host resource metrics.
Follow [agent/README.md](agent/README.md) for installation, access headers and disk
mounts. Installer downloads require the admin session, CSRF and a valid enrollment
credential; configuration remains local and is never saved in plaintext by Ops.
Each credential can
only submit bounded monitoring reports for its enrolled host. Revocation takes
effect immediately; hosts become offline after 90 seconds without a received report.
Agent outage and service health failures use the existing service/endpoint alert
settings. Enrollment state persists in the existing Ops data volume.

The agent reports host CPU, memory, uptime, optional disks and service reachability;
Java Minecraft status also provides player counts. It does not inspect containers,
manage servers, read player profiles or expose remote commands. HTTP checks can
monitor Calagopus health endpoints and any selected service accessible from that
host. Installing agents and configuring real probes/SSO remain operator steps.
