# Velora Velocity Gateway

Independent Java 21 Velocity plugin for exact hostname routing. Build from
`integrations` with `./gradlew :velocity-gateway:build --no-daemon` (Windows:
`gradlew.bat`). The JAR is in `velocity-gateway/build/libs`. Source belongs to
Velora with the rights restrictions described in the root LICENSE.md; no
repository-wide MIT grant applies. Velocity and Gson retain their own
licenses and are supplied by the proxy, not bundled. No gameplay dependency.

The component retains its own Gradle dependency lockfile. Velocity publishes
snapshot API/Brigadier dependencies, so the lock fixes dependency coordinates
but does not freeze changing snapshot bytes. Pin and verify runtime JAR digests
for deployment. See [third-party notices](NOTICE.md).

## Controller contract

The complete Panel hosts the controller. Admin sessions may GET/PUT
`/api/admin/gateway`. PUT supplies `{revision, routes}`; revision is the previously
read revision. Conflicts return 409; successful writes increment the revision.
Routes have `id`, `experience`, `hostname`, `address`, `port`, `enabled`.
IDs are stable opaque ASCII identifiers. Hostnames are exact DNS names with no
wildcards. Endpoints are literal RFC1918/CGNAT IPv4 addresses, never public DNS.
The UI supports manual endpoints; Calagopus remains the process lifecycle owner.

The plugin GETs `/api/gateway/v1/routes` every five seconds and POSTs
`/api/gateway/v1/status`. Both require a dedicated bearer credential; this
credential cannot access admin APIs. Panel stores desired routes and the latest
status in its existing platform SQLite `kv` table. No database identity changes.
Admin writes persist even while offline; the UI confirms applied revisions only
after a recent plugin report. TCP reachability is not Minecraft login health.

Validated updates register new backends, atomically replace `routes.json`, then
publish one routing snapshot. Failed validation/persistence retains the previous
snapshot. Unchanged routes retain their registered server and active sessions;
changed, removed or disabled routes disconnect affected players. A restart loads
the last-known-good snapshot before polling. No fallback or cross-route switching
is allowed, including the built-in `/server` command. Keep this proxy dedicated:
do not install plugins that override routing/auth events after this plugin.
Connection assignment/closure logs contain no player identities. Panel receives
the latest 50 hostname assignment/closure events, aggregate connection counts,
route health and sync errors; this bounded buffer is not a durable audit history.

## Deployment and identity acceptance

### Plugin settings and startup errors

On first start, the plugin creates `plugins/velora-gateway/gateway.properties`.
If required settings are missing, it refuses connections and names the missing
environment variable and file setting instead of throwing a NullPointerException.
Game-server panels can edit this file without needing custom environment-variable
support. Fill in the existing operator values and restart Velocity:

```properties
panel-url=https://panel.example.org
gateway-token=<same 32+ character gateway credential configured on Panel>
yggdrasil-url=https://auth.example.org/api/yggdrasil
auth-verified=velora-yggdrasil
```

The URLs above are placeholders. Use the existing Panel base URL and the existing
Velora Yggdrasil API root. Set `auth-verified` only after the identity acceptance
tests below. The file contains a credential: keep it private and outside Git.
Generated files are owner-readable/writable only on POSIX filesystems.
`VELORA_GATEWAY_PANEL`, `VELORA_GATEWAY_TOKEN`, `VELORA_GATEWAY_YGGDRASIL`, and
`VELORA_GATEWAY_AUTH_VERIFIED` override their respective file entries. A present
but empty environment variable fails validation; it does not reuse a file value.

The properties file does not load a Java agent. Keep Velocity online mode enabled
and put `-javaagent:authlib-injector.jar=<existing Yggdrasil HTTPS API root>` before
`-jar` in the JVM startup command. Relative and absolute injector paths are
accepted; the injector URL must exactly match `yggdrasil-url`. Retain the pinned
injector, forwarding secret and custom-account setup described below. If you use
the supplied launch script, its existing environment and digest requirements
still apply.

### Deployment steps

Use existing VPS capacity. Do not provision services or change production DNS
as part of source builds. Operator configuration and runtime snapshots are secret
or operational data and must remain outside the checkout.

1. Install a compatible, pinned Velocity build on the VPS. Listen on public
   TCP 25565. Install this JAR into `plugins/`.
2. Connect VPS and backend hosts using existing Headscale/WireGuard. Bind backend
   servers to their tunnel interfaces. On each backend firewall, allow its game
   port **only from the gateway's tunnel address**, deny public ingress, and
   verify from both an allowed tunnel peer and an external host. Do not forward
   home router ports. Restrict tunnel ACLs to the intended backend ports.
3. Set Velocity `online-mode = true`, `player-info-forwarding-mode = "modern"`,
   `[servers] try = []`, and empty `[forced-hosts]`. Create a unique forwarding
   secret outside source. Disable Velocity server-list backend passthrough.
   On Paper enable `proxies.velocity.enabled`, matching secret and online-mode
   policy; backend `server.properties` uses `online-mode=false` behind the
   authenticated proxy. Fabric 1.20.1 needs a compatible FabricProxy-Lite version.
   See [PaperMC forwarding](https://docs.papermc.io/velocity/player-information-forwarding/)
   and [security](https://docs.papermc.io/velocity/security/).
4. **Velora custom Yggdrasil accounts are the selected identity system.** Stock
   Mojang online mode is insufficient. Before enabling routes, qualify a pinned
   authlib-injector Java agent on the **Velocity JVM and backend JVMs** against
   the existing Velora Yggdrasil URL, using the launcher's established custom
   identity flow. Preserve
   the auth service/signing identities. Do not create offline UUIDs or migrate
   existing player data. Prove authenticated login, rejected invalid sessions,
   UUID equality with existing Velora accounts, skins and secure-chat behavior
   on both backends. [Upstream documents Velocity support](https://github.com/yushijinhun/authlib-injector/wiki/%E5%9C%A8-Minecraft-%E6%9C%8D%E5%8A%A1%E7%AB%AF%E4%BD%BF%E7%94%A8-authlib-injector/2ebe822b634cb1fc4cfca4d59debbe23d7cf8ffb),
   but these exact Velora/Velocity/backend combinations remain unverified. If
   unsupported, implement and review a custom session verifier before enabling
   this gateway. This plugin does not implement a Yggdrasil session verifier.
5. Only after that acceptance succeeds, set
   `VELORA_GATEWAY_AUTH_VERIFIED=velora-yggdrasil` on the proxy. Without this
   explicit operator gate the plugin fails closed. Also set
   `VELORA_GATEWAY_YGGDRASIL` to the exact existing HTTPS API root passed to the
   Java agent; startup verifies the `authlib-injector.jar` agent argument matches.
   The gate is an operator attestation, not proof of successful authentication or
   correct forwarding. Disable the agent's Mojang namespace to avoid alternate
   identities. The supplied launch script sets that option.
6. Set `VELORA_GATEWAY_PANEL` to the HTTPS Panel base URL and set
   `VELORA_GATEWAY_TOKEN` to the same randomly generated 32+ character credential
   in Panel and gateway secret environments. Rotate it in both services. Set
   Panel `VELORA_GATEWAY_PUBLIC_ENDPOINT` for the overview. TLS is mandatory;
   custom trusted CAs belong in the JVM trust store. No inbound control API is
   exposed by the plugin.
7. Point `smp.velora.scopedd.lol` and `unleashed.velora.scopedd.lol` A/AAAA records
   to the VPS (only publish AAAA if IPv6 listener/firewall works). Add SRV records
   if using a different public port. Names remain configurable in the Panel.
8. Add disabled routes using the actual tunnel IPs/ports. Perform acceptance in
   an isolated test deployment, then enable confirmed routes.

## Required live acceptance before production

Test full Fabric 1.20.1 SMP modpack login **and gameplay**, including mod channel
negotiation, reconnects, skins, UUIDs, inventory continuity and chat. Test the
actual Unleashed Paper 26.3 build similarly; compiling against Velocity API is
not proof of that version's protocol compatibility. Join each corrected hostname,
mixed case/trailing dot, reject unknown/direct-IP hostnames, attempt `/server` and
backend kicks, change/disable/delete routes with players connected, disconnect
Panel then restart proxy and verify last-known-good recovery. Verify private
firewall isolation and invalid controller credentials. No real servers or
accounts were available to source tests; these remain release gates.

If the modpack cannot traverse Velocity, keep its route disabled. Use a dedicated
public port (or a separate public IP) with a raw TCP stream forwarder over the
same private tunnel and SRV DNS for the SMP hostname. Ordinary L4 TCP proxies
cannot distinguish Minecraft hostnames using TLS SNI; same-port hostname routing
requires a separately reviewed Minecraft handshake-aware router. Raw forwarding
must preserve the existing backend custom authentication configuration and
firewall isolation, rather than applying Velocity modern forwarding to it.

## Troubleshooting and rollout

An offline report is older than 90 seconds. Unconfirmed edits are desired state,
not an applied update; check TLS, credential, plugin startup logs and revision
errors. An unreachable backend means TCP failed, not that Calagopus should be
replaced. Snapshot errors fail closed at startup; back up and repair the local
snapshot from a trusted Panel export, never fall back to another experience.
Back up Panel SQLite and plugin data before upgrades. Restore both together;
older controller revisions are rejected to avoid silent route rollback.
Manual workflow `velocity-gateway.yml` packages the JAR; Panel packaging uses
`panel-release.yml`. Neither deploys the gateway. Deploy pinned bytes only after
review, source CI and live identity/modpack acceptance.

The optional [launch script](deploy/start.sh) verifies operator-pinned SHA256
digests for Velocity, authlib-injector and the plugin before running Java.
The [systemd unit](deploy/velora-gateway.service) runs an unprivileged dedicated
user with only `/var/lib/velora-gateway` writable. Operators create the user and
directories, install the pinned JARs there, copy the script to `/opt/velora-gateway`,
and protect `/etc/velora-gateway/environment` as root-only. Set the above secrets,
acceptance gate and three digests in that file. No credentials, forwarding secret,
DNS settings or real addresses are provided by these templates. Install/enable
the service only after acceptance; no workflow installs or starts it.
