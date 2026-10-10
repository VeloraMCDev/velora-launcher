# Velora Core for Calagopus

A [Calagopus](https://calagopus.com) panel extension (`net.velora.core`) that connects a game server to the Velora Panel and installs and updates the Velora Core mod from it. How the pieces fit together, and how a release gets here, is in [docs/VELORA_CORE_DEPLOYMENT.md](../../docs/VELORA_CORE_DEPLOYMENT.md).

It adds:

- a **Velora Core** page to every server (install, update, connect, authlib-injector, automatic updates);
- **Admin → Extensions → Velora Core** settings (Velora Panel address, global switch for automatic updates);
- the permissions `velora-core.read` and `velora-core.manage` (server) and `velora-core.manage` (admin). Server owners and admins have them; grant them to subusers as needed.

Calagopus extensions are native Rust plus React code that is compiled into the panel, so installing one needs the **`:heavy`** image or a development setup. See Calagopus' [installing extensions](https://calagopus.com/docs/panel/extensions/installing-extensions) guide.

## Layout

```text
net_velora_core/
  Cargo.toml, Metadata.toml   package identity and the required Panel version
  src/                        Rust backend: settings, Wings file operations, installer, background task, routes
  frontend/src/               React: the server page and the admin settings page
```

The installer logic that does not need a Panel (address and token checks, jar-name and version parsing, download path rules) has unit tests in `src/velora.rs`.

## Build and package

Extensions are built inside a checkout of the Calagopus panel:

```sh
git clone --branch release-1.2.4 https://github.com/calagopus/panel.git
cp -r integrations/calagopus/net_velora_core panel/backend-extensions/
cd panel
cargo test -p net_velora_core          # add --ignore-rust-version if your toolchain is older than the panel requires
cd frontend && pnpm install && pnpm build:ci && cd ..
panel-rs extensions export net.velora.core   # writes exported-extensions/net_velora_core.c7s.zip
```

Install the `.c7s.zip` as described in the Calagopus guide, then restart the panel. The manual **Calagopus extension** GitHub workflow runs these steps and uploads the package as an artifact.

`Metadata.toml` requires panel `>=1.2.0`. Raise `version` in `Cargo.toml` for each release of the extension.

## First setup

1. In the Velora Panel, approve a Velora Core release (Servers → Velora Core).
2. In Calagopus open Admin → Extensions → Velora Core and enter the Velora Panel's public address.
3. On a server, open **Velora Core**: install the mod, paste the server token from the Velora Panel's Servers page, download authlib-injector, add the shown `-javaagent` flag to the startup command, then restart.

## Status

Written to Calagopus' documented extension API and the panel's own route code. See the validation notes in [docs/VELORA_CORE_DEPLOYMENT.md](../../docs/VELORA_CORE_DEPLOYMENT.md) for what has and has not been exercised, and try it on a disposable server before using it on a live one.
