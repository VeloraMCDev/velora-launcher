# Single-host (VPS) deployment

Runs the complete Velora Panel, the documentation site and the operations dashboard
on one Docker host behind an existing Traefik (for example a Pangolin stack). Nothing
here needs Cloudflare Workers; the [control plane](../control-plane/README.md) remains
an optional multi-host design.

| Service | Image | Purpose |
|---|---|---|
| `panel` | `ghcr.io/<owner>/velora-panel@sha256:…` | Player/admin websites, launcher API, accounts |
| `docs` | `nginx-unprivileged` + `docs/dist` | Searchable documentation |
| `ops` | `ghcr.io/<owner>/velora-operations@sha256:…` | Health, activity, backups, updates and alerts |

Release images come from the manual `Build immutable Panel image` and
`Build immutable Operations image` workflows, which test the exact image before
publishing it. Digests are pinned in `.env`; the dashboard's Updates page changes
them (Panel updates take a backup first and roll back automatically if the new build
is unhealthy).

## Files on the host

Keep these in the deployment directory (for example `~/velora-platform`). Only
`compose.yaml` and `docs-nginx.conf` come from this directory; the rest are site
values that stay out of Git.

```text
compose.yaml        # copy of infra/vps/compose.yaml
docs-nginx.conf     # copy of infra/vps/docs-nginx.conf
.env                # STACK_DIR, DOCKER_GID, EDGE_NETWORK, PANEL_IMAGE, OPS_IMAGE (no secrets)
panel.env           # PUBLIC_URL, SCOPENET_TRUSTED_PROXIES, CURSEFORGE_API_KEY… (mode 600)
ops.env             # PANEL_INTERNAL_URL, OPS_PUBLIC_URL, OPS_ENDPOINTS, DOCS_SITE_URL (mode 600)
panel-data/         # Panel store, owned by 65532
ops-data/           # dashboard settings, audit log and metric history, owned by 1000
docs-site/          # built documentation (the dashboard can rebuild it from main)
src/                # repository checkout used for documentation builds
```

`ops.env` example:

```sh
PANEL_INTERNAL_URL=http://velora-platform-panel:8080
OPS_PUBLIC_URL=https://ops.example.org
OPS_ENDPOINTS=Panel=https://example.org/health,Docs=https://docs.example.org/
DOCS_SITE_URL=https://docs.example.org
```

## Backups

`velora-backup.sh` copies `panel-data` while the Panel runs, pauses the Panel for about
a second for the final delta, verifies every SQLite store and writes a checksummed
archive. Archives are kept locally and pushed to an offsite host with an `rrsync -wo`
(write-only) key restricted to the VPS address.

```sh
sudo install -m 750 velora-backup.sh /usr/local/sbin/velora-backup
sudo install -m 644 velora-backup.service velora-backup.timer velora-backup-request.path /etc/systemd/system/
sudo install -d -m 700 /etc/velora && sudoedit /etc/velora/backup.env   # DATA, REMOTE, REMOTE_KEY, …
sudo install -d -o root -g docker -m 2770 /var/lib/velora-backup
sudo install -d -o root -g docker -m 750 /var/backups/velora
sudo systemctl daemon-reload
sudo systemctl enable --now velora-backup.timer velora-backup-request.path
```

The dashboard's "Back up now" button (and every Panel update) creates
`/var/lib/velora-backup/request`; `velora-backup-request.path` starts the backup and the
dashboard reads `/var/lib/velora-backup/last.json`.

## Routing and access

Add routers for the Panel, documentation and dashboard hostnames to Traefik pointing at
`http://velora-platform-panel:8080`, `http://velora-platform-docs:8080` and
`http://velora-platform-ops:8080`. The dashboard holds the Docker socket, so put an SSO
layer in front of it (a Pangolin resource with authentication) in addition to its own
Velora administrator sign-in.
