# Velora monitoring agent

## Repository Compose example

[compose.yml](compose.yml) runs the tested published agent image on Linux Docker
Engine. Enroll a host in Ops **Agents** and save its downloaded configuration as
`velora-monitor-config.json` beside the Compose file. The configuration contains
the Ops HTTPS URL, host ID and token; it is ignored by Git. Do not commit it.

From this directory:

```sh
sudo chown 1000:1000 velora-monitor-config.json
sudo chmod 600 velora-monitor-config.json
docker compose -f compose.yml pull
docker compose -f compose.yml up -d
docker compose -f compose.yml logs --tail=50 monitor
```

Wait for **Online** in Ops. Stop with `docker compose -f compose.yml down`.
The example pins linux/amd64; for an ARM host without emulation, use Ops's local
build bundle. On Docker Desktop remove `network_mode: host` and use
`host.docker.internal` for host service probes. The guided Ops installer below
generates the appropriate networking automatically. Replace the image digest only
when intentionally upgrading to another tested agent release.

## Docker installation (recommended)

1. In Ops **Agents**, enroll a host, choose **Linux Docker Engine** or **Docker
   Desktop**, select a published agent image, and add the Minecraft,
   HTTP/Calagopus or TCP services to check.
2. Download the Docker bundle. Extract the ZIP into a private folder on that host.
   It contains Compose, Dockerfile, agent source and the enrolled configuration.
3. On Linux, give the container's unprivileged UID access to its config:

   ```sh
   sudo chown 1000:1000 velora-monitor-config.json
   sudo chmod 600 velora-monitor-config.json
   ```

4. From the extracted folder, start the agent:

   ```sh
   docker compose pull
   docker compose up -d
   docker compose logs --tail=50 monitor
   ```

Docker with Compose is the only host prerequisite. Published agent images are
discovered from GHCR and pinned by digest in the downloaded Compose file. The
manual **Build immutable Monitoring Agent image** workflow builds linux/amd64,
tests authenticated Ops reporting and health probes with the exact image, and
publishes `ghcr.io/veloramcdev/velora-monitor-agent` under immutable build tags.
Registry package visibility must allow anonymous reads for Ops discovery and host
pulls; if private, authenticate Docker with read access and use a known digest or
choose the local build option. Registry credentials never go in the agent config.

For ARM hosts without amd64 emulation, choose **Build locally from bundled
source** and run `docker compose up -d --build`. That downloads the official Node
24 Alpine base for the host architecture. It restarts the agent unless
explicitly stopped, including after reboot if Docker starts at boot. There is no
agent registry login, Git clone, separate Node install or automatic agent update.
Stop with `docker compose down`; after editing probes in the JSON file, run
`docker compose restart monitor`. To upgrade a published image, replace only the
image digest in Compose with a new tested agent digest, then pull/start again.
Retain the existing configuration and host credential. For a local-build upgrade,
download the current Node agent
from Ops, replace the bundle's `monitor.mjs`, and run the build/start command again;
keep the original enrollment configuration. Enrollment never changes the services.

## Connecting to Ops

Enroll the host under Ops **Agents**. Its generated JSON contains `ops_url` (the
HTTPS address of that Ops deployment), `id` and the one-time `token`. The container
reads this file at `/config/velora-monitor-config.json` and sends authenticated
reports to `POST /api/agents/<id>/report` every 30 seconds. No token goes in the
image, Compose environment or command line. Outbound HTTPS access is enough;
there is no inbound agent port. Wait for **Online** in Ops after startup.

If it remains **Awaiting first report**, check `docker compose logs --tail=50
monitor`, DNS/TLS access to the Ops URL and the existing SSO policy. A 401 means an
invalid/revoked credential; an SSO redirect requires service access headers in
`ops_headers` or a narrowly scoped report-route policy as described below.
An offline agent stops reporting for 90 seconds; unhealthy probes can still appear
while the agent is online. Do not use the Panel login token for enrollment.

Linux Compose uses host networking so `127.0.0.1` can reach host services. Docker
Desktop uses bridge networking: use `host.docker.internal` for host services or
reachable DNS/IP targets. Existing probes are not rewritten when changing install
mode; review their targets. Docker Desktop metrics describe its Linux VM, not the
native Windows/macOS host; use the standalone agent for native metrics.
See [Docker networking](https://docs.docker.com/engine/network/drivers/host/).

The container runs as UID/GID 1000 with a read-only filesystem, dropped
capabilities, bounded logs and no Docker socket, host PID access, inbound ports or
privileged mode. Only its configuration file is mounted by default. To monitor a
disk, add a read-only bind mount for that particular directory in Compose and use
the container path in `disks`. Do not mount the whole host filesystem. Keep SSO
service access headers in the local JSON as described below.

The ZIP contains a credential: restrict folder access, keep it out of Git/backups
shared with others, and remove extra downloaded copies after installation. Protect
the extracted configuration using the Linux ownership/mode above or a restricted
Windows ACL. Revoking the agent in Ops immediately invalidates its credential.

## Standalone Node installation

Download the Node agent from Ops **Agents** or **Builds**. Requires Node 24 on Windows,
Linux or macOS. Enroll each host separately in Ops, save the one-time configuration,
and edit its `services` and `disks` locally. Protect the file as a credential: on
Linux use mode 0600; on Windows restrict its ACL to the service account. Never add
the real configuration to Git. Revoke and re-enroll to rotate a credential.

The generated configuration contains `ops_url`, `id`, `token`,
`interval_seconds` (15–60, default 30), `services` and `disks`. Keep the generated
identity/token values. Example probe lists (synthetic targets):

```json
{
  "services": [
    { "name": "SMP", "kind": "minecraft", "host": "127.0.0.1", "port": 25565, "virtual_host": "smp.velora.scopedd.lol" },
    { "name": "Unleashed", "kind": "minecraft", "host": "127.0.0.1", "port": 25566, "virtual_host": "unleashed.velora.scopedd.lol" },
    { "name": "Calagopus", "kind": "http", "url": "http://127.0.0.1:8080/health" },
    { "name": "Other TCP service", "kind": "tcp", "host": "127.0.0.1", "port": 9000 }
  ],
  "disks": [{ "name": "Game data", "path": "/srv/games" }]
}
```

Replace targets with the health endpoints and local ports for your installation.
HTTP health means a 2xx response; redirects and login pages returning redirects
are unhealthy. Optional per-target `headers` support authenticated health checks.
Minecraft probes use Java status, not login, and forward only online/max counts,
never status player samples. A gateway status check measures the gateway's response;
probe each backend directly to establish backend health. TCP checks establish only
that a port accepts connections. Missing disk paths are omitted; use a drive path
such as `C:\\` for Windows.

Run as an unprivileged service account with read access to configured disk paths:

```sh
node velora-monitor-agent.mjs /path/to/velora-monitor-config.json
```

Use your existing systemd service or Windows process supervisor to restart the
process after reboot. No inbound agent port, Docker socket or administrator access
is needed. The host requires outbound HTTPS access to Ops and network access to
its selected probes. The agent sends reports every configured interval; Ops marks
them stale after 90 seconds. Probe batches are bounded and can add latency with
many unreachable targets. First CPU sample is unavailable until a second sample.

Keep Ops behind its existing SSO. For noninteractive service access, add
`ops_headers` to the local configuration with your SSO's service access headers;
these cannot override the agent's Authorization, Content-Type or Host headers.
Alternatively scope an SSO exception strictly to `POST /api/agents/<id>/report`
and retain TLS and the Ops bearer check. Do not expose other Ops routes through
that exception. Reports refuse redirects to avoid forwarding credentials.

Ops stores only a hash of the enrollment credential and sanitized metrics.
Target URLs, HTTP headers, filesystem paths and player identities stay on the
agent. This process provides monitoring only; it cannot restart game servers,
install artifacts or administer Calagopus. Licensing follows the repository's
[license boundaries](https://github.com/VeloraMCDev/velora-launcher/blob/main/LICENSE.md).
