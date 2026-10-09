# Velora Operations

The operator dashboard for a Velora deployment: one place to watch health, review
activity, take backups, deploy new builds and approve launcher releases.

| Page | What it does |
|---|---|
| Overview | Service health, host CPU/memory/disk, endpoint latency, certificates, backups and community totals, with 24 hours of history |
| Services | Velora containers with CPU, memory, logs, restart and pull & recreate; every other container on the host, read-only |
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
