# Game authentication and gateway development recipes

These assemble the tested public game-authentication components. They do not
replace the complete Panel, private experience host, web account/provider/audit/
presence/deletion composition or existing installation. Use fresh synthetic or
explicitly imported owned stores; do not switch an active mixed legacy database.

## Reviewed sources

Clone the repositories into adjacent `authentication`, `panel` and `infra`
directories and pin these development revisions:

- Authentication: `6c19518686c69f52158644c2165f233b0f71939a`.
- Panel gateway: `6713e9e54090ef04f98149f3f80097ed36027b4c`.

The native runtime was first validated at Authentication `12c98f1` and Panel
`74dadad`; later revisions add container packaging/acceptance without changing
those runtime sources. Dependencies are public registry packages or reviewed
included SDK snapshots. The repositories' current visibility is separate from
source independence; visibility and registry publication have not changed here.

## Native process recipe

Build with Rust 1.98.1 and native C tools:

```sh
cargo +1.98.1 build --manifest-path authentication/Cargo.toml -p velora-auth-service -p velora-auth-tools --release --locked
cargo +1.98.1 build --manifest-path panel/Cargo.toml -p velora-panel-gateway --release --locked
```

Follow [protected storage/bootstrap guidance](../authentication-native/README.md)
and [gateway configuration](https://github.com/VeloraMCDev/panel/blob/scopedd/velora-migration/GATEWAY.md).
Set the same operator public origin, including any prefix, in both processes.
Bind Authentication privately on 8081 and Gateway on 8080; Authentication trusts
only the gateway's actual socket IP. TLS terminates in the operator reverse proxy.
Other application routes require an explicitly configured Panel upstream; no
Panel/private application is secretly started by this recipe.

Run the synthetic real-process checker from the Infra checkout:

```sh
python3 tests/game_auth_gateway.py --auth ../authentication/target/release/velora-auth-server --tools ../authentication/target/release/velora-auth-tools --gateway ../panel/target/release/velora-gateway --scratch-root .runtime-checks
```

On Windows, use Python 3 and the `.exe` binaries. The checker always creates new
synthetic data and retains its protected fixture directory. Native Windows checks
exercise process termination/recovery; Linux exercises actual executable SIGTERM.
Rust tests separately verify graceful incremental-stream drain on both platforms.

## Docker Compose component recipe

Docker Engine and Compose v2 are required. Copy `.env.example` to an untracked
`.env`, set your origin and a protected bootstrap file, and check that source
contexts resolve to the pinned checkouts. Password bytes must be exact UTF-8,
without an accidental newline. Keep the source file in a private directory and
make it readable by container UID 65532. File-backed Compose secrets retain host
permissions; setting a YAML secret mode does not fix unreadable host permissions.

```sh
cd recipes/game-auth-gateway
docker compose --env-file .env config --quiet
docker compose --env-file .env build
docker compose --env-file .env up --detach --wait
```

Images are built locally; no registry account, private experience package or
published image is required. Runtime containers use UID/GID 65532, a read-only
root filesystem, dropped capabilities and no new privileges. Authentication has
protected persistent `/data` and writable `/tmp`; its port is not published. The
gateway alone publishes a loopback port for an operator TLS reverse proxy.

An isolated authority network gives both services distinct fixed addresses; the
gateway's address is used for exact trusted-peer checking. Choose a non-conflicting
subnet and both addresses together if the
example `172.30.50.0/24` is unsuitable. The other gateway network permits external
ingress; Authentication has no external network. Game mode requires no provider
egress. Future web/provider composition needs a separate reviewed network profile.

If the public origin has a path prefix, set VELORA_HEALTH_PATH to that prefix plus
`/health/ready`. Keep both origin settings identical. Health probes are live; an
upstream failure does not authorize requests or activate a fallback authority.

Stopping preserves the named data volume. SIGTERM drains active work with a
30-second development stop grace. Do not use volume deletion as an upgrade step.
Operator migration must close the old writer, validate its checkpoint/assets,
import into a new owned destination and test rollback before a real cutover.

## Acceptance evidence

`tests/container_game_auth_gateway.py` builds these two images and starts only a
generated, disposable Compose project. It refuses external/fixed data volumes and
container names, checks fresh non-root/read-only startup, game login/skin/profile
signatures and public prefixed URLs, stops Authentication to prove fail-closed
outage behavior, then uses the included offline tools to checkpoint/carry assets
and boot the restored owned volume. Existing game tokens, signing identity,
passwords and texture bytes remain usable; live revocation remains enforced.
Cleanup targets only the generated project's volumes/networks.

Panel CI uses reviewed copies of the Infra Compose/checker and an anonymously
fetched pinned public Authentication source. No private repository is fetched.
See [current progress](../../migration/CURRENT_PROGRESS.md) for exact checks/revisions.

Full application install, imported legacy/native upgrades, complete operational
backup/restore/rollback, shared-host/control-panel profiles, real launcher/server
end-to-end parity and public release acceptance remain unfinished. Development
components and passing synthetic checks do not close those parent gates.

The offline tools profile mounts the stopped source volume read-only at /source and a separate destination volume at /data. Restoration never nests its output inside its input tree. The acceptance checker restores into that separate volume before selecting it for Authentication startup. Both Linux container acceptance runs passed: 37360361204 and 37360132106.
