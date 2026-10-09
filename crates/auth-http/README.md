# Authentication HTTP compatibility snapshot

This directory consumes the reviewed `velora-auth-http` library from
VeloraMCDev/Authentication. `SNAPSHOT.json` pins all twelve Rust sources. Build
metadata uses the source workspace version and authority core to preserve type
identity; public wire models come from the immutable SDK contract snapshot in
`packages/rust-platform-contracts`. Registry publishing is disabled; complete source is public.

`panel/server/src/yggdrasil/mod.rs` supplies the existing pools, texture path,
signing key, shared login guard and typed host ports. It preserves live operator
branding, public origin resolution, trusted proxy/client IP handling and the
legacy host implementation version. The surrounding host keeps its middleware
and 4 MiB upload limit. Game-token protection and wire error shapes remain in
the owned router. The account/launcher response bridge converts fields explicitly.

Verify drift with:

```sh
```

This library adoption does not open or import databases, change key/asset paths,
complete account/admin workflows or establish the standalone service deployment.

`panel/server/src/auth/account.rs` supplies account policy, live username policy
and login audit ports. Existing launcher sign-in/register/session routes delegate
to the owned workflows and bridge SDK response fields explicitly. The host passes
its current request state, original identity pool and shared JWT/guard instances;
registration uses random UUIDs and keeps approval-mode responses unchanged.

`panel/server/src/routes/connections.rs` supplies password-reset email readiness,
configured public origin and SMTP delivery ports. Forgot/reset workflows are
owned by `reset`; current pools and provider credentials stay in the host. Host
fixtures verify actual-route one-use revocation, rollback/retry and configuration
responses without sending email.

Discord start/callback/poll/link/unlink workflows consume typed host configuration,
origin, provider-profile and live allocation-policy ports. The host's credentialed
HTTP token/profile exchange is retained exactly; fixtures contact no provider.
Common account allocation now belongs to the authority for registration, OAuth,
admin creation and bootstrap; caller validation and live username policy remain
explicit. Existing identity/group/cross-domain cleanup and service cutover remain
separate acceptance gates.

Player rename/profile/cosmetic/avatar workflows use the account module. Typed
host ports retain live server presence and launcher/audit cleanup inside the same
transaction, preserving rollback and UUID/session behavior. Multipart extraction
and avatar HTTP headers stay with the existing routes.

Administrator identity and group routes delegate to admin with the existing
AdminUser guard. Host view aggregation and multi-experience purge remain local;
owned mutation functions preserve field ordering, password transaction rollback,
self protection and group wire defaults.

Administrator skin/model/cape operations, cape CRUD and identity counts are
authority-owned. Existing guards, multipart handling and private view aggregation
remain in host adapters; route fixtures cover assignment, validation and removal.
