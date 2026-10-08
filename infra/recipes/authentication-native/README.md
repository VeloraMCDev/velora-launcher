# Native game-authentication development recipe

This starts the independently buildable game-authentication component. It is a
migration development checkpoint, not the full Velora deployment or production
cutover. Panel, web account/provider/presence/audit/gateway composition and HTTPS/
process-manager deployment profiles remain pending. No package is published here.

## Build the reviewed public source

Use Rust 1.98.1 and the platform's native C build toolchain. The source and included
SDK snapshot require no private repository credentials:

```sh
git clone --branch scopedd/velora-migration https://github.com/VeloraMCDev/authentication.git velora-auth
git -C velora-auth checkout 12c98f147deaaf1ba06ba4599274db8ed41ece38
cargo +1.98.1 build --manifest-path velora-auth/Cargo.toml -p velora-auth-service --release --locked
./velora-auth/target/release/velora-auth-server --help
```

The pin contains runtime/lifecycle tests; it is not a stable published release.
Build on the intended native host. Independent CI checks the Linux executable's
SIGTERM lifecycle and release build; Windows tests check runtime shutdown and CLI
errors. Cross-platform binary distribution and native upgrade acceptance remain
pending. See the current migration tracker for exact validation evidence.

## Fresh development startup

Choose a new protected directory. Never point the process at the active mixed
monorepo store or start another credential writer against the same data. Import
requires owned offline checkpoint/asset tooling and operator quiescence.

```sh
export VELORA_AUTH_PUBLIC_ORIGIN=https://example.invalid
export VELORA_AUTH_BIND=127.0.0.1:8081
export VELORA_AUTH_DATA_DIR=/path/to/new/protected/auth-data
export VELORA_AUTH_SERVER_NAME=Velora
./velora-auth/target/release/velora-auth-server
```

Replace the example origin/path. The listener is HTTP: external HTTPS and public
prefix forwarding belong to the operator's reverse proxy. For a first account,
provide a protected UTF-8 password file and paired `VELORA_AUTH_BOOTSTRAP_USERNAME`
and `VELORA_AUTH_BOOTSTRAP_PASSWORD_FILE` values. The exact file text is the
password; avoid an unintended newline. Never put that file in Git. Existing
administrators, passwords and UUIDs are retained; bootstrap files need not remain
after initial setup. See [the pinned service guide](https://github.com/VeloraMCDev/authentication/blob/12c98f147deaaf1ba06ba4599274db8ed41ece38/SERVICE.md)
for signing continuity, configuration, storage/proxy details and route scope.

From another terminal:

```sh
curl --fail --silent --show-error http://127.0.0.1:8081/health/live
curl --fail --silent --show-error http://127.0.0.1:8081/health/ready
curl --fail --silent --show-error http://127.0.0.1:8081/api/yggdrasil
```

Readiness validates the live store; liveness does not certify account access.
Only Yggdrasil/texture routes are mounted. Ctrl-C, or SIGTERM on Linux, drains
requests, closes SQLite and releases the OS writer lock. Its empty file remains;
deleting it does not authorize bypassing another writer. The old monorepo does
not use this lock, so coordinated single-writer cutover is still required.

Windows uses the `.exe` executable and PowerShell `$env:` assignments. Signed
packages, service-manager installation, Docker/full-stack composition and full
fresh-install/restore/rollback acceptance remain separate tasks.
