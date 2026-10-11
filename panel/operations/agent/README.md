# Velora monitoring agent

Download the agent from Ops **Agents** or **Builds**. Requires Node 24 on Windows,
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
[license boundaries](../../../LICENSE.md).
