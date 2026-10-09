# Native game-authentication development recipe

Build the standalone game-authentication service from this repository with
Rust 1.98.1 and native C build tools. Run from the repository root:

```sh
cargo build --release --locked -p velora-auth-service -p velora-auth-tools
./target/release/velora-auth-server --help
```

See [the service contract](../../../docs/components/authentication/SERVICE.md)
and [current validation](../../../docs/VALIDATION.md). This service does not
replace the complete Panel's web-account and gameplay composition.

## Fresh development startup

Choose a new protected directory. Never point the process at the active mixed
monorepo store or start another credential writer against the same data. Import
requires owned offline checkpoint/asset tooling and operator quiescence.

```sh
export VELORA_AUTH_PUBLIC_ORIGIN=https://example.invalid
export VELORA_AUTH_BIND=127.0.0.1:8081
export VELORA_AUTH_DATA_DIR=/path/to/new/protected/auth-data
export VELORA_AUTH_SERVER_NAME=Velora
./target/release/velora-auth-server
```

Replace the example origin/path. The listener is HTTP: external HTTPS and public
prefix forwarding belong to the operator's reverse proxy. For a first account,
provide a protected UTF-8 password file and paired `VELORA_AUTH_BOOTSTRAP_USERNAME`
and `VELORA_AUTH_BOOTSTRAP_PASSWORD_FILE` values. The exact file text is the
password; avoid an unintended newline. Never put that file in Git. Existing
administrators, passwords and UUIDs are retained; bootstrap files need not remain
after initial setup. See [the service guide](../../../docs/components/authentication/SERVICE.md)
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
deleting it does not authorize bypassing another writer. The complete Panel does
not use this lock, so coordinated single-writer cutover is still required.

Windows uses the `.exe` executable and PowerShell `$env:` assignments. Signed
packages, service-manager installation, Docker/full-stack composition and full
fresh-install/restore/rollback acceptance remain separate tasks.
