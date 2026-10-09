# Native game authentication and gateway

These independently buildable components are maintained in the root Rust workspace.
They do not replace the complete Panel or start its gameplay/application routes.

From the repository root with Rust 1.98.1 and native C build tools:

```sh
cargo build --release --locked -p velora-auth-service -p velora-auth-tools -p velora-panel-gateway
```

Follow [authentication startup](../authentication-native/README.md) and
[gateway configuration](../../../docs/components/panel/GATEWAY.md). Bind Auth
privately on 8081 and Gateway on 8080. Configure the same public origin/prefix
in both; TLS terminates in the operator reverse proxy. Authenticate exact trusted
gateway peers and keep owned SQLite/signing material isolated from fixtures.

From infra/, run the synthetic native-process checker with Python 3:

```sh
python tests/game_auth_gateway.py --auth ../target/release/velora-auth-server --tools ../target/release/velora-auth-tools --gateway ../target/release/velora-gateway --scratch-root .runtime-checks
```

Add .exe suffixes on Windows. The checker creates a fresh fixture and retains it
for inspection. Passing native checks does not prove production TLS, host setup,
live store conversion or full-product upgrades.

The obsolete Compose recipe depended on Dockerfiles in separate repositories and
has been removed. The maintained [complete Panel image](../../../Dockerfile) and
root Compose definition are the full-product container entry points. Exact-image
Docker acceptance remains required before release.
