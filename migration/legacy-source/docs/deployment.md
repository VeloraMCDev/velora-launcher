# Docker Compose deployment

This deployment builds the panel and web UI from the checked-out source. It does
not need GitHub Actions or a published GHCR image.

1. Install Docker Engine with the Compose plugin on a Linux host. Point a DNS
   name at that host and arrange HTTPS through your reverse proxy.
2. Check out the commit you want to deploy, then copy `.env.example` to `.env`.
   Set `ADMIN_PASSWORD` to a unique password and `PUBLIC_URL` to the public
   `https://` URL. Set `SCOPENET_TRUSTED_PROXIES` only to the IP addresses of
   proxies that send forwarding headers.
3. Run:

   ```bash
   docker compose config --quiet
   docker compose up -d --build
   docker compose ps
   docker compose logs --tail=100 panel
   ```

The panel listens on `PANEL_PORT` (default `8080`). Docker checks
`/scopenet-panel healthcheck` inside the container. Wait for `healthy` in
`docker compose ps` before directing players to it. If `ADMIN_PASSWORD` is
blank on first start, the generated password appears in the panel logs.

Servers draw their own world map and send it to the panel; nothing extra to
install or host. See [The SCOPENET Map](map.md).

The named `panel-data` volume holds the database, uploads, hosted launcher
downloads, and the generated JWT signing key. Preserve that volume when
upgrading. To back it up, stop the panel, archive the volume with a temporary
container, then start it again:

```bash
docker compose stop panel
docker run --rm -v scopenet-mc_panel-data:/data:ro -v "$PWD":/backup alpine tar czf /backup/panel-data.tar.gz -C /data .
docker compose up -d
```

If your Compose project has a different name, use the volume name shown by
`docker volume ls`. To upgrade, pull the desired commit and run
`docker compose up -d --build` again. Do not use `docker compose down -v` unless
you intend to delete the panel data.

## SQLite temporary-directory error during upgrade

SQLite error `6410` (`SQLITE_IOERR_GETTEMPPATH`) means it cannot find a writable
temporary directory. Older `scratch` images omitted `/tmp`; instance database
migrations and temporary views can expose this when upgrading an existing panel.
Current images provide `/tmp` with mode `1777` for the non-root runtime user.

For an affected published image, add a temporary filesystem under your existing
panel service in `docker-compose.yml`:

```yaml
services:
  panel:
    tmpfs:
      - /tmp:rw,nosuid,nodev,mode=1777
```

Then run `docker compose up -d --force-recreate`. Keep the existing `/data` volume
mounted: this workaround supplies SQLite scratch space without changing stored
panel data. Alternatively, pin the last working image digest until a corrected
release is available.

## HTTPS and phones

The [player panel](player-panel.md) is meant to be opened on phones, so put the panel behind HTTPS before announcing it (Caddy, Traefik, nginx or a Cloudflare Tunnel all work; see the [admin guide](admin-guide.md#https)). Browsers only offer **Install app** on secure origins, and sign-in tokens should never cross the network in the clear. Set `PUBLIC_URL` to the public HTTPS address so links the plugin and emails hand out (`/scopenet panel`, password resets) point to the right place.
