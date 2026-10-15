# Destination-bound Panel credentials

October 15, 2026. Security follow-up to activity-only snapshot acceptance, not a
whole authentication/release checkpoint. Ledger remains **62/76 (81.58%)**.

## Root cause

`commands::bootstrap::fetch_manifest(state, panel)` accepted an explicit requested
Panel, but selected its bearer using the old generic `accounts::panel_token(state)`
helper against the **configured** Panel. `set_panel_url` calls that function for
a prospective destination before committing settings. Thus an existing account's
bearer could be attached to the new destination without any concurrent race.
This is a source-confirmed credential-binding defect, not evidence of an observed
external disclosure. The new regression sends only synthetic data to loopback.

Other callers independently resolved destination, active credential and selected
instance, retaining the mixed-context risk found in the earlier telemetry review.

## Implementation and ownership

New `panel_context.rs` provides three private host helpers:

- `current` holds settings → accounts → existing secret lookup while capturing
  the effective Panel, matching bearer and selected instance.
- `token_for_panel` selects the active account's token for the caller's explicit
  requested destination, without resolving that destination again from settings.
- `token_for_account` applies the reviewed `PanelClient::same_base` validation
  to an explicit account/destination; callers needing a stable account selection
  retain the account read guard while invoking it.

The unsafe ambient destination token helper is removed. Username change,
account-API construction, public-server/leaderboard reads and manifest discovery
now use the appropriate helper. Telemetry reuses the same scoped lookup under
its already-reviewed stable settings/account guards and consent checks. No guard
spans HTTP awaits. The ordinary configured/baked/locked Panel priority, request
paths, IPC names, error strings, headers and payload schemas remain unchanged.

Instance-manifest fetching already used an explicit destination and held-account
snapshot, but accepted a missing stored origin as authority to use a bearer.
It now requires a present validated matching origin. Its existing authority
normalizer, persisted cache shape, denial fencing and transport-only fallback
policy are retained; credential presence correctly marks new legacy-originless
responses unauthenticated.

## Compatibility boundary and remaining gates

Missing, invalid or mismatched stored origins cannot authorize a bearer. Matching
hosts/default ports/trailing separators normalize as before; tenant path casing
is significant. This intentionally tightens the old helper's case-insensitive
whole-URL comparison and missing-origin allowance. It is **not** transparent
legacy-authentication compatibility: originless accounts can require a fresh
sign-in, and their old authenticated cache authority can differ from the new
unauthenticated lookup, denying prior offline reuse until scoped refresh.

No account ID/UUID, selected ID, encrypted file/key format, credential-store name,
updater/signing identity or cache filename is rewritten. Lookup tests assert
byte-preservation of synthetic stores. There is no guessed-origin migration,
automatic sign-out or token deletion. Installed legacy upgrades, recovery and
re-sign-in UX remain release gates; a newly scoped sign-in is not certified here
as an ID-preserving migration of an originless entry.

This fix does not revoke already captured tokens, provide offline authorization
leases, prove redirect/proxy confinement, or make all multi-command context
updates atomic. Stale response publication, map response URL attribution and
post-response account-selection behavior need their own review. It is not
Yggdrasil/authlib/gameplay or whole-product authentication acceptance.

## Verification

Mencius authored only `panel_context/tests.rs` and
`commands/bootstrap/credential_tests.rs`: five direct context tests plus two
actual `fetch_manifest` loopback tests. Prospective lookup must omit the old
bearer, leave the original listener untouched and preserve settings/accounts/
encrypted secret/key bytes. A matching explicit destination still authenticates
even when configured settings name another Panel. Legacy/malformed/mismatched/
offline/missing-active cases and retained selected-instance context are covered.

Lead adds three direct account-request-builder tests and one actual instance
manifest loopback test: request destination/bearer/instance remain captured,
unknown origins and pending transactions cannot build authenticated requests,
and originless manifest fetch sends no bearer and records unauthenticated cache
authority while retaining the stored token.

Initial host run: **160 passed, one failed** in an existing telemetry wire
fixture, whose accepted stream returned `WouldBlock` before data arrived. That
fixture now explicitly sets blocking mode before its read timeout and also sets
a write timeout; no production workaround was added. Initial failure remains in
`panel-context-host.log`. Final default host: **161 passed**; feature host:
**168 passed** (overlapping suites, not 329 independent outcomes); non-test check:
pass. Node 24.21.0 frontend: **102 passed**. Boundary/identity, new-module scoped
formatting and whitespace checks pass. No patched-native GUI replay is claimed.
Final secret scans cover 1,535 tracked files (six reviewed matches, zero
unreviewed) and 191 nonignored untracked additions (zero findings). Local Markdown
link checking passes for 101 targets. These are bounded source checks, not a
content/license audit or proof that runtime privacy is complete.

Independent security/state review owns `panel-context-review.md`; installed/native
and release acceptance remain separate. Nash accepted this bounded change,
independently repeated 19 tests and verified final source/test fingerprints.
The lead frontend run used Node 24.21.0; the reviewer records Node 24.19.0 for
its separately executed Rust-test filters. Logs remain in the existing temporary
October 14 directory, despite this follow-up occurring October 15. No dependency,
lockfile, commit, branch, PR, signing, deployment or paid infrastructure change.
Actual billed USD is unknown.
