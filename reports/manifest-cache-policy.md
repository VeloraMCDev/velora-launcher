# Instance-manifest cache failure policy

## Later live-denial follow-up — October 14, 2026

An independently reviewed process-local gate now prevents transport fallback
after known-denial invalidation failure in the same process, and generation
matching prevents an older success from clearing a newer denial. See
`manifest-denial-fence.md` and `manifest-denial-review.md`. Disk failure followed
by process restart, credential epochs and an explicit offline authorization
lease remain unaccepted. Earlier limitation statements are historical for the
same-process case, not a claim that durable revocation is solved.

## Authority follow-up — October 14, 2026

The source now adds exact Panel/account/authentication/instance provenance,
atomic compatible-shape publication and known-denial invalidation. Legacy
unscoped files require online refresh before offline use. Fifteen focused tests
include actual production-caller loops with synthetic `AppState` stores.
See `manifest-cache-authority.md` for current behavior and remaining gates.
The October 13 raw-format/write and provenance limitations below are historical,
not the latest implementation. No additional ledger checkpoint is awarded.

October 13, 2026. The previous all-error fallback could use an installed cached
manifest after online authorization denial, removal or malformed responses.
`game::instance_manifest` now delegates failed fetches to a typed policy helper:
only `PanelFailure::Network` with no HTTP status or a 2xx status may read cache.
Auth, HTTP status, decode, config and untyped instance-removal errors return
without consulting cached authority. No error-text matching is used.

The raw `.scopenet/manifest.json` format/path, instance identities and successful
online write behavior remain unchanged. The genuine transport-failure path reads
cache without rewriting it. Missing/invalid cache stays an error with the original
typed network cause and a generic context, not raw parse/file/private response
text. HTTP 429/500/503 now fail rather than claiming an offline fallback; this is
a deliberate availability/security policy change.

Five focused tests pass using synthetic loopback responses, authenticated request
headers and a refused local connection:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher --lib game::manifest_cache::
```

Tests prove no cache-closure access for 401/403/404/429/500/503, malformed success,
invalid configuration and untyped removal, no old-cache read on successful online
response, and one fallback consultation for a typed transport error. Missing
fallback retains the original typed cause. The exact production caller is compiled
in the final host suite; tests do not construct a live `AppState`, read native
keyrings or execute a game. Separate review in `manifest-cache-policy-review.md`
accepts this restriction only, not cached authority in general.

Legacy cache is still not bound to origin/account identity or a validity lease.
Network classification includes TLS/other transport failures, not proof of an
Internet outage. Offline revocation cannot be established from old cache alone.
Atomic cache writes, cross-panel/account provenance, native acceptance and an
explicit offline-authority policy remain gates. No cache deletion, credential
mutation, wire command/signature change, migration, dependency or deployment.
