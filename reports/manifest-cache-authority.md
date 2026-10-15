# Instance-manifest cache authority follow-up

## Later same-process denial fence

The subsequent `manifest-denial-fence.md` / `manifest-denial-review.md` close the
same-process reuse gap after invalidation failure and fence older success
responses against newer denials. The remaining restart/lease/epoch limits below
still apply; the earlier same-process-gap statement describes the prior snapshot.
Final focused cache/gate tests: 19; whole host: 138. No ledger credit.

October 14, 2026. This is implemented source work toward the remaining cache
security gate, not a new accepted checkpoint. The fixed ledger stays **62/76
(81.58%)**; VEL-012 checkpoint 4 and native/release gates remain open.

## Implemented behavior

The actual `game::instance_manifest` caller now captures the Panel URL, selected
account identity and request authentication state together before fetching. The
request token comes from that captured account, not a second account lookup.
Cache authority includes normalized Panel scheme/host/port/base path, account
ID/kind/UUID and whether the request was authenticated. It stores no bearer or
game token. Path case remains significant; HTTPS/default-port/host normalization
does not silently merge different Panel tenants or schemes.

Only the existing typed transport-failure policy can attempt offline fallback.
Fallback additionally requires version-1 provenance, exact authority and the
requested instance ID. Changing Panels, accounts, account UUID/kind, instance ID
or credential availability rejects fallback. Online response instance identity
is also checked before use or cache publication.

The persisted `.scopenet/manifest.json` destination and instance IDs are unchanged.
Provenance fields are added alongside the existing flattened instance/files
fields. A regression deserializes new cache bytes with the legacy
`InstanceManifest` type and verifies the same manifest. Legacy files have no
verified authority, so they require a successful online refresh before offline
use; valid legacy files upgrade atomically in place. This deliberate security
restriction does not invent provenance for old cached private manifests.

Publication uses the existing exclusive-sibling, synced atomic persistence
helper rather than truncating the destination. Corrupt evidence and symlink
destinations are preserved on rejection; immediate symlink cache parents are
also rejected. Cache publication failure is redacted and does not turn a
successful online response into an offline response.

Known online 401/403/404 responses invalidate matching cached authority while
retaining the manifest's legacy-readable payload. A different authority cannot
invalidate the cache through that helper. A subsequent successful online fetch
can authorize and republish it. Typed 404 status is retained rather than replaced
with an untyped string, allowing the caller to apply the same invalidation policy.

## Validation

Fifteen focused tests pass, including two actual `AppState`/production-caller
loopback tests. They cover cross-account/cross-Panel fallback, missing credentials,
successful offline reuse for the original scope, and known 401/403/404 denial
followed by refused-network fallback. All fixtures are disposable synthetic
stores. Unit-test secret storage uses generated temporary fallback keys, not
the native OS keychain or any player/operator store.

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher --lib game::manifest_cache::
```

Retained run log: `/tmp/velora-revitalization-20261014/manifest-authority-integration.log`.
The final locked whole-host library sweep passes **102 tests**, zero failures.
The non-test launcher library check also passes. Logs are
`/tmp/velora-revitalization-20261014/manifest-authority-final-host.log` and
`/tmp/velora-revitalization-20261014/manifest-authority-host-check.log`.
These checks do not replace independent security or native acceptance.

## Remaining limitations

Separate `authority-accessibility-review.md` accepts this bounded implementation
after independent focused checks, not the full cache/security checkpoint.

This does not establish a signed offline authorization lease, discover revocation
while genuinely offline, or protect against an attacker who can rewrite the
user's local cache/identity stores. Failed invalidation writes preserve evidence
and are reported, but can leave fallback usable on later calls even before a
restart. Credential-presence binding is not a credential epoch or revocation lease.
Immediate parent checks are not descriptor-relative protection against an active
ancestor-directory swap. No cache TTL, full content-cache production integration,
complete fault/security audit, native game/loader/keystore, supported-OS or
installed upgrade acceptance is claimed. No credential-store identity, IPC
signature, dependency, lockfile, purchase or deployment changes in this slice.
