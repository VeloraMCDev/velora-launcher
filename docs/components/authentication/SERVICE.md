> Historical owner documentation from `VeloraMCDev/authentication` at `5162abd28300300667ea2b029fc63551cb966977`. Current segment instructions are in the monorepo READMEs.

# Standalone game-authentication service

`velora-auth-server` is a runnable Yggdrasil authority with its own SQLite store,
signing keys, textures, bootstrap, health endpoints and graceful shutdown. It uses
public dependencies and the verified public SDK snapshot, with no Panel or private
Experiences checkout. It serves existing game-authentication routes; web account,
administrator, reset/Discord, presence/audit and Panel gateway cutover remain pending.
It is not the complete replacement Authentication deployment.

Library embedders can now explicitly compose the existing public account HTTP
routes through mandatory platform ports; see [account composition](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/ACCOUNTS_HTTP.md).
The command-line service still mounts game routes only until real adapters and
writer/transaction cutover acceptance are implemented.

## Build and run

Use Rust 1.98.1:

```sh
cargo +1.98.1 build -p velora-auth-service --release --locked
./target/release/velora-auth-server --help
```

Configure an operator-owned external origin and a protected storage directory:

```sh
export VELORA_AUTH_PUBLIC_ORIGIN=https://example.invalid
export VELORA_AUTH_BIND=127.0.0.1:8081
export VELORA_AUTH_DATA_DIR=/path/to/protected/auth-data
export VELORA_AUTH_SERVER_NAME=Velora
./target/release/velora-auth-server
```

Replace the example origin/path with your own. The listener is plain HTTP; expose
it through an operator-managed HTTPS reverse proxy. If the origin includes a path
prefix, the proxy must remove that prefix before forwarding to this service.
The public origin governs metadata, signed texture URLs and the API-location header;
incoming Host/forwarded-host headers cannot change it.

Windows builds use `target\release\velora-auth-server.exe` and PowerShell `$env:`
assignments. Native service-manager packaging and Windows shutdown acceptance are
separate operational gates; the runtime shutdown path is tested on both platforms.

## Storage and bootstrap

See [configuration](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/CONFIGURATION.md) for exact database/key/texture paths, secret
files and trusted proxy settings. New files/directories use Unix 0600/0700 permissions.
Existing operator directories, file permissions and signing bytes are retained;
operators must protect supplied paths and Windows ACLs. No production store is
renamed or moved by this command.

Fresh empty stores initialize the owned schema. Existing stores are validated
read-only before writable startup; mixed or damaged schemas are refused. Import a
closed legacy checkpoint with the [offline tools](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/SCHEMA.md), preserving signing
and texture assets. An intact owned version-one store upgrades additively; omitted
historical launcher records still require re-import from the original checkpoint.

An OS-held `<database>.service.lock` excludes another service writer, including
ordinary symlink aliases. The empty lock file remains after shutdown/crash; file
existence is not the lock. This does not coordinate an old monorepo process which
does not use this lock. Operator quiescence and a single-writer cutover remain required.

For a fresh administrator, create a protected UTF-8 password file containing the
exact password, then set `VELORA_AUTH_BOOTSTRAP_USERNAME` and
`VELORA_AUTH_BOOTSTRAP_PASSWORD_FILE`. No default password is generated or logged.
Account insertion is conditionally atomic. If any administrator already exists,
including a disabled one, startup skips password-file access and never resets or
promotes another account. Unreadable/invalid supplied credentials fail startup.

The RSA and JWT loaders retain existing bytes and refuse silent replacement of
damaged material. Cryptographic work runs before serving requests. A failed startup
can leave newly initialized protected files; inspect/restore them rather than
deleting existing signing material to retry.

## Routes and health

- `/api/yggdrasil` and its existing auth/session/profile/certificate/texture mutation
  routes: game credentials, active-account admission and signing behavior.
- `/textures/{hash}`: existing content-addressed texture delivery.
- `/health/live`: 200 while the HTTP process is responsive.
- `/health/ready`: 200 only while serving and the live owned schema validates;
  returns 503 with generic JSON on store outage/damage or shutdown.

Requests retain the 4 MiB body limit. Game authentication uses the existing shared
login guard and live account/session lookups; database failures never grant access.
Client IP comes from the real socket peer, with forwarded chains accepted only
from explicitly configured trusted proxy addresses.

On Ctrl-C, or SIGTERM on Unix, readiness stops, requests drain, SQLite closes and
the writer lock releases. Forced termination relies on normal SQLite/OS recovery.
The web-policy configuration fields do not enable unmounted web registration/login
routes; Yggdrasil retains its original game-account admission behavior.

## Validation and migration limits

Synthetic live TCP tests cover bootstrap, restart UUID/password/key continuity,
real game login/join/IP matching, profile signature verification, duplicate writers,
store outages, mixed-store and damaged-key refusal, CLI errors and graceful runtime
shutdown. Linux additionally tests the actual executable with SIGTERM and lock
release. No SMTP, Discord or production identities are used.

This is an AUTH-08 runtime checkpoint. AUTH-08/09/10 and final release acceptance
remain open until web/admin/provider/presence/audit composition, compatible gateway,
production-cutover/rollback, deployment profiles and upgrade/restore checks pass.

## Container component

The Dockerfile builds only public Authentication packages and the included SDK
snapshot. It includes the game service and offline tools, runs as UID/GID 65532,
and prepares protected /data plus writable /tmp. Use a persistent owned volume,
read-only root filesystem, a writable /tmp mount and readable protected bootstrap
files. The container listens on port 8081; its public origin remains operator
configuration. Do not publish this internal port instead of the trusted gateway.
CI builds the image, checks unprivileged help/temp/data behavior and initializes a
fresh owned SQLite volume with network access disabled. Images are not pushed or
published. This component recipe does not complete production packaging,
process-manager/cutover, native upgrades or full web/provider/gateway acceptance.
