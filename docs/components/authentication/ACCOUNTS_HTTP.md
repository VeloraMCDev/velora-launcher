> Historical owner documentation from `VeloraMCDev/authentication` at `5162abd28300300667ea2b029fc63551cb966977`. Current segment instructions are in the monorepo READMEs.

# Public account HTTP composition

`velora_auth_http::web_routes::routes` mounts twelve existing method/path pairs
for login, registration, current identity, recovery/reset, profile, username,
skin upload/removal/model, cape selection and avatars under `/api/v1`. It delegates
to the existing extracted authority workflows and retains JSON/errors, four-MiB
body limits, multipart field handling, live JWT admission and avatar cache headers.

`Runtime::router_with_accounts(AccountPorts)` and `serve_with_accounts` compose
these routes with the independent game authority, persistent signing/JWT material,
owned store/writer lock, readiness and graceful shutdown. They require all ports:

- `AccountHost`: live sign-in/registration policy, username rules and login audit.
- `MutationHost`: live online-player lookup and original transaction-bound rename
  effects, including launcher admission revocation and activity observations.
- `ResetHost`: configured email readiness/origin and actual delivery.

There are no default host implementations. Supply real public platform adapters;
an audit, identity-store or presence failure must propagate, and rename side effects
must participate in the supplied transaction. A cross-service implementation must
prove its transaction/observation semantics before credential-writer cutover.
Do not ship synthetic test adapters or replace these ports with no-ops.

JWT signature/expiry alone does not admit an account: every protected request reads
its current authority identity and status/auth version. Missing bearer, expiry,
pending status and disabled/removed/revoked sessions retain original errors and
precedence. Cached JWT names/roles do not replace live records. The policy and
username ports are called at the existing workflow boundaries. Login audit failure
prevents token issuance, retaining the original earlier last-login write boundary.

The command-line/container service still mounts game routes only. Real policy,
audit/presence/transaction/email adapters, standalone administrator/Discord routes,
gateway web ownership and whole-product writer/data cutover remain incomplete.
This composition API is implemented and tested, but does not close that gate.

Run `cargo test -p velora-auth-service --test accounts --locked` with Rust 1.98.1.
Synthetic real-listener tests exercise live policy, approval, admission, audit
failures, rename rollback, password-reset revocation and persistent restart
continuity. Tests create only isolated owned authority stores and a separate
synthetic audit store. No operator/player data or external providers are used.
