# Process-local instance-manifest denial fence

October 14, 2026. This closes the known same-process fallback gap after a
401/403/404 invalidation-write failure. It does not establish durable revocation,
an offline authorization lease, credential epochs or production content caching.
The fixed ledger remains **62/76 (81.58%)**; no checkpoint is inferred.

## Implemented behavior

`AppState` owns an in-memory `ManifestDenials` gate keyed by the existing exact
instance-cache path. A known denial records a fresh generation **before** trying
to invalidate cached authority on disk. Failure to invalidate retains the gate;
a later network-only failure cannot read that path's otherwise valid stale cache.
The fence intentionally spans accounts/Panel changes for that instance path:
any subsequent offline access needs a successful online refresh first. Other
instance paths are unaffected.

An online fetch observes the generation before dispatch. A successful response
publishes cache and clears a denial only if no newer denial was recorded while
the request was in flight. A stale success still returns its online response to
that caller but neither publishes nor clears the newer denial. An attempt begun
after a denial can clear it after successful publication; failed publication
retains it. This is local response ordering, not a remote authorization epoch.

One short synchronous mutex region covers each gate/associated disk operation;
no mutex is held across HTTP awaits. Offline reads are checked while holding
that gate. Already returned manifests and already issued requests are not
retroactively revoked. IDs, legacy-readable manifest fields, cache version/path,
authentication selection and transport-only fallback policy remain unchanged.

## Executed regressions

- Two gate unit tests cover failed invalidation, path isolation, failed refresh,
  successful refresh, and an old success both before an initial denial and before
  a newer denial. The denied read/publish closures must not execute.
- A production-caller Unix fixture obtains a manifest, substitutes a synthetic
  symlink to make invalidation fail, observes 403, then restores the unchanged
  regular cache and drops its loopback server. Cached authority is still present
  on disk; the next transport failure nevertheless refuses offline reuse.
- A second production-caller fixture obtains a manifest, observes 401, refreshes
  successfully online, then loses its server. Offline reuse succeeds only after
  that successful publication has removed the gate.

The focused `game::manifest` suite passes **19** tests (17 cache + 2 gate).
The final host sweep passes **138** tests, zero failures; non-test host check
passes. These totals overlap the preceding 134-test transaction sweep, not 272
independent product assertions. Logs are `manifest-denial-{focused,host,check}.log`
under `/tmp/velora-revitalization-20261014/`. Separate independent review is in
`manifest-denial-review.md`; author observations are not self-approval.

## Limits

The in-memory gate disappears at process exit. If disk invalidation failed,
restart can still find an old authorized cache; no durable-denial claim is made.
There is no TTL/offline lease, token epoch, signed cache, hostile path-race defense,
or native/supported-OS acceptance. The map retains one generation per denied path
until successful refresh or state destruction; it has no eviction bound. Shared
gate locking serializes cache I/O even for different instances and has not been
profiled as a performance improvement. Ordinary 429/5xx errors still do not use
cache and are not treated as permanent account/instance denials.

Only synthetic temporary stores and loopback HTTP are used. No real account,
keychain/player store, new dependency, lockfile, commit, release or deployment
is introduced.
