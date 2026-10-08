> Historical owner documentation from `VeloraMCDev/authentication` at `5162abd28300300667ea2b029fc63551cb966977`. Current segment instructions are in the monorepo READMEs.

# Standalone authority configuration

`velora_auth_http::config::RuntimeConfig` parses standalone authority settings and
loads protected secret files. The [game-authentication service](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/SERVICE.md) now
composes it with owned persistence, bootstrap, health and shutdown. Complete web
and provider service composition remains pending.
The legacy host keeps its existing configuration adapter and compatibility aliases;
these settings are not applied to its live pools or persisted paths.

Set `VELORA_AUTH_PUBLIC_ORIGIN` to the operator's external HTTP(S) origin, including
any gateway prefix. It is required; no private network/domain is embedded. URLs
with credentials, query strings or fragments are refused. Trailing slashes are
normalized without dropping an existing path prefix.

Storage and listener settings:

- `VELORA_AUTH_BIND`: numeric IP and port; default `127.0.0.1:8081`.
- `VELORA_AUTH_DATA_DIR`: default `./data`.
- `VELORA_AUTH_DATABASE_PATH`: default `<data-dir>/auth.sqlite`; explicitly select
  the imported authority store at cutover. This does not rename the original database.
- `VELORA_AUTH_RSA_KEY_PATH`: default `<data-dir>/yggdrasil-signing.pem`.
- `VELORA_AUTH_TEXTURES_PATH`: default `<data-dir>/textures`.
- `VELORA_AUTH_JWT_PATH`: default `<data-dir>/jwt.secret`.
- `VELORA_AUTH_JWT_SECRET_FILE`: optional exact configured JWT bytes. Missing,
  unreadable or empty configured files fail without falling back to a new secret.
- `VELORA_AUTH_TRUSTED_PROXIES`: optional comma-separated IP addresses; default none.

Keep database, RSA and JWT files distinct. The parser rejects detected collisions
before opening or creating signing material. Existing-path and existing-parent
canonicalization detects ordinary aliases; filesystem isolation and permissions
still belong to the operator. Configured secret bytes are never trimmed, including
legacy short configured values. The persisted loader preserves valid existing
bytes and refuses automatic replacement of damaged material.

Initial operator policy:

- `VELORA_AUTH_PANEL_ACCOUNTS`: `true` or `false`; default `true`.
- `VELORA_AUTH_REGISTRATION`: `closed`, `open` or `approval`; default `closed`.
- `VELORA_AUTH_BOOTSTRAP_USERNAME` and `VELORA_AUTH_BOOTSTRAP_PASSWORD_FILE`:
  optional paired values; no fixed/default administrator password is provided.
  Usernames use the public SDK's 3–16-character rule. New bootstrap passwords use
  the existing minimum-eight-Unicode-character validator and exact UTF-8 file text.

`bootstrap_credentials(has_admin)` skips password-file access if any administrator
already exists, including a disabled account. The caller supplies the live
administrator check and composes the authority-owned conditional bootstrap workflow.
Loaded credentials have no Debug/Serialize implementation. Configuration Debug
contains paths and policy, not secret-file contents. The game service implements
protected bootstrap-file delivery and live readiness; provider/SMTP/online-presence,
web account routes and gateway cutover still require their own acceptance.

Synthetic tests use injected lookups and temporary files, avoiding process-global
environment mutation. See [configuration provenance](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/CONFIG_PROVENANCE.json),
[store and checkpoint guidance](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/SCHEMA.md) and the migration report for remaining
configuration, service, gateway, rollback and single-writer cutover work.
