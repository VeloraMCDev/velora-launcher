> Historical owner documentation from `VeloraMCDev/authentication` at `5162abd28300300667ea2b029fc63551cb966977`. Current segment instructions are in the monorepo READMEs.

# Authentication store initialization

`velora_auth_core::schema::initialize` starts the owner-local migration series
on an empty SQLite store. Enable the core `sqlite` feature and configure
`SqliteConnectOptions::foreign_keys(true)` on every pooled connection before
calling it. Initialization runs in one transaction and may be repeated on an
intact store. Version one remains unchanged; the additive version-two migration
creates launcher admission records and their indices in the same transaction.
An intact version-one store is validated before upgrade. Existing account/key
records, ID high-water marks and SQLite `user_version` are retained.

The store contains users, groups and memberships, historical name reservations,
capes, game tokens and joins, player certificate keys, provider connections,
OAuth attempts, password resets and launcher admission sessions. Account IDs, UUID constraints, credential
versions, email preferences, case-insensitive name semantics and cascading
authentication records match the final legacy authority schema. The new series
does not execute the legacy mixed application's migrations or gameplay triggers.

Existing databases without the owned marker, unsupported versions, missing or
foreign tables and missing named identity constraints are refused. This is an
initializer, not a comprehensive corruption or tampering detector. Do not use
it to adopt a production legacy database or automatically repair a damaged store.

`schema::validate` checks an existing version-one or version-two store without DDL
and returns its actual version. Immutable asset readers use it instead of attempting
an upgrade. It refuses an empty/mixed file; validation never creates a store.

The host still owns launcher activity observations/audit, mail logs and experience records.
This initializer neither reads those records nor opens a file, imports data,
creates an administrator or generates signing keys. Caller-supplied pools,
existing signing keys and texture paths continue to serve the current deployment.

## Authority-only checkpoint import

`velora_auth_core::import::import_checkpoint` accepts a caller-supplied offline
checkpoint pool and an empty initialized destination. Enable `PRAGMA query_only`
on every source connection. The caller must create a consistent, closed backup
and protect it; this API does not make a live database into a checkpoint.

The importer reads fixed authority projections, copies in a single destination
transaction, and verifies every projected value before commit. It preserves
account/group/cape/launcher-session ID high-water marks as well as records, so previously deleted
IDs cannot be reassigned. Missing tables, uninitialized UUIDs, nonempty
destinations, inconsistent sequences and constraint failures abort the import.
Historical name reservations include deleted players. Only table counts are
returned; credentials and key material are never included in the report.

Session projection preserves original IDs, nullable account/name fields, raw IP
bytes and timestamps, including legacy anonymous records. Source checkpoints must
include the final legacy session table; earlier owned version-one imports lacking
it are refused rather than silently assuming no history. Re-import the original
closed legacy checkpoint into a new destination before considering cutover. Merely
upgrading a version-one store creates an empty session table and cannot reconstruct
excluded history. Panel activity/event history remains in the protected original.

The source is not modified, and its private tables and triggers are not copied.
This library helper is not a deployment or automatic upgrade command. See
[import provenance](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/IMPORT_PROVENANCE.json).

## Offline file commands

The `velora-auth-tools` package provides commands that operate on new destination
files. Run in an existing protected directory. New files use owner-only mode on
Unix; Windows inherits directory permissions, which the operator must configure.

First stop writers and use a closed database with no SQLite sidecars. Preserve a
byte-for-byte checkpoint, including the original mixed/private schema, in a
protected location before importing authority records:

```sh
cargo run --locked -p velora-auth-tools -- checkpoint /protected/closed-panel.db /protected/original-checkpoint.sqlite
```

This refuses overwrites/sidecars, verifies both file hashes and SQLite integrity,
and reports byte count, SHA-256 and the original SQLite user version. The copy
preserves every original byte rather than replaying migrations or rebuilding the
database. A mixed-source checkpoint contains private gameplay and player records;
it is backup material, not public repository content. The operator must stop and
keep writers stopped throughout database and asset checkpoint preparation. This
command does not perform a live backup or prove external writers are stopped.

```sh
cargo run --locked -p velora-auth-tools -- init /protected/new-authority.sqlite
cargo run --locked -p velora-auth-tools -- import /protected/closed-checkpoint.sqlite /protected/new-authority.sqlite
```

The import source must already be a consistent, closed SQLite backup. The command
opens it read-only/query-only and refuses `-wal`, `-shm` or `-journal` sidecars.
It hashes the source before and after import, closes both pools, and reports
schema version, table counts and file hashes as JSON. Treat the files and report
as protected migration material; never commit production material to a repository.

Existing destination files are always refused. A failed operation may leave its
new destination file for inspection; do not deploy a failed import. A changed
source checkpoint produces an error even if the transaction already committed.
Import uses SQLite immutable read-only mode after sidecar preflight, including
closed files that retain WAL journal mode. The tool does not change the source's
journal mode or replay sidecars. It cannot prove the operator stopped writers and
does not authorize deployment.

## Signing continuity and first-admin composition

RSA loading and `secrets::jwt_secret` retain valid existing bytes. Invalid or
unreadable RSA files and short or unreadable persisted JWT files produce an error
without replacement. Explicitly configured legacy JWT bytes are used unchanged.
New key/secret files use exclusive creation, synchronous writes and owner-only
Unix permissions. Restore signing material from the protected checkpoint if it
is damaged; changing it intentionally requires a separate rotation plan.

`velora_auth_http::web::bootstrap_admin` composes first-admin creation through the
normal username-policy, password-hash and UUID allocation path. A conditional
database INSERT arbitrates competing attempts. Existing administrators, including
disabled accounts, retain credentials and UUIDs on restart. A conflicting player
name fails rather than promoting that player. The caller supplies credentials and
delivery policy; the library never logs a generated password. A standalone entrypoint
and its bootstrap configuration/delivery still need implementation.

## Carry signing and texture assets

After importing a closed database checkpoint, copy assets from the corresponding
closed data-directory backup into a new protected directory:

```sh
cargo run --locked -p velora-auth-tools -- assets /protected/closed-data /protected/new-authority.sqlite /protected/new-assets
```

This validates the owned database read-only, verifies that every referenced skin
and cape texture exists and matches its filename hash, and copies all cached
hash-named textures so previously issued texture URLs retain their files. It loads
the existing RSA key without generating one, copies RSA/JWT bytes exactly, verifies
output hashes and RSA public identity, and records `assets.manifest.json`. The
manifest contains hashes and relative file names, not key bytes or player records.
Directories/files use Unix modes 0700/0600; Windows requires protected inherited
directory permissions. Symlinks, unexpected texture files, missing/corrupt assets,
existing destinations and observed checkpoint changes are refused.

If the original deployment used an explicitly configured JWT secret instead of
`jwt.secret`, provide a protected file containing its exact bytes as the final
argument. No whitespace is trimmed. Keep explicit configured-secret mode when
restoring a legacy secret shorter than 32 bytes; the persisted-secret startup
loader deliberately refuses short files. The report distinguishes configured and
persisted JWT sources.

Preflight failure creates no destination. Failure during copying may leave a
partial directory for inspection, and must not be deployed. Source data must be
offline and stable; this tool does not stop writers. Synthetic
restore tests cover password, web/game token, join-session, certificate and RSA
signature continuity plus live revocation after restoration. Complete deployment,
checkpoint/restore/rollback and gateway URL acceptance remain pending.

Before deployment, migration still needs operator checkpoint acceptance, full
restore/rollback and gateway checks, standalone bootstrap delivery, provider/SMTP
configuration, and a verified single-writer cutover.
Starting a second credential writer against the live legacy database is not a
supported migration step. See [schema provenance](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/SCHEMA_PROVENANCE.json).
