> Historical owner documentation from `VeloraMCDev/authentication` at `5162abd28300300667ea2b029fc63551cb966977`. Current segment instructions are in the monorepo READMEs.

# Provenance

Extracted identity cryptography from `scopeddlol/SCOPENET-MC@57daa92cb6cb4027982d06bfa1c692ab9edb88ff`: `panel/server/src/yggdrasil/keys.rs` unchanged; the password functions from `panel/server/src/auth.rs` retain the original Argon2 parameters and validation rules with HTTP errors mapped by the host adapter. No signing keys, credentials, identity database, gameplay code or operator data is included.

JWT primitives are extracted from the same source's `auth.rs`: HS256, claim names,
30-day expiry and default version=0 for legacy tokens are retained. A small borrowed
identity subject replaces the Panel `UserRow` input; the host maps errors and retains
its live account status/auth-version admission checks. No SQL or Panel state enters
this package.

This package is not the complete authentication service. Persistence, token revocation
authority, identity/groups/account workflows and Yggdrasil HTTP extraction remain pending.
