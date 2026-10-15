# Stable activity destination and account snapshot

October 15, 2026. Follow-up to the independently identified mixed-snapshot risk
in `telemetry-consent-review.md`. The master-plan ledger remains **62/76 (81.58%)**;
this narrow security fix does not satisfy a whole native/release checkpoint.

## Root cause and change

Previously activity capture resolved a destination, separately called the generic
Panel-token helper (which resolved the destination again), then read the active
username. Concurrent account/Panel mutations could mix those observations. An
earlier captured destination could receive a token selected against a later
Panel, or a username could describe a different selected account.

Activity capture now holds the settings read guard while checking consent and
resolving the destination, then the accounts read guard through selection and
the existing secret-store lookup. Those local guards end when capture returns;
they do not span request construction, spawning, or network awaits. The Recorder
retains the captured values and the shared current-consent handle from the
previous fix. Consent send-time checks and their admission/in-flight limitations
remain unchanged.

`AppState::panel_url_from_settings` factors the existing configured/baked/locked
priority and trailing-slash policy into a pure helper. Ordinary `panel_url()` uses
the same helper; capture does not recursively acquire settings. `PanelClient`
factors its unchanged constructor URL validation/normalization into `parse_base`,
and its new `same_base` compares two successfully validated request bases.
Host casing/default ports/trailing separators normalize consistently with actual
requests; tenant path case and origin/port differences remain significant.
Credentials in a URL, query/fragment, unsupported scheme, or invalid URL are not
accepted as credential scope.

Only a Panel-kind account with an explicit matching stored Panel URL can supply
an activity bearer. Missing legacy origin now fails closed **for this optional
activity bearer only**; storage/IDs/schema remain untouched, and the existing
unauthenticated activity behavior is retained. No automatic migration, sign-out,
token deletion or stronger offline-authority policy was added. Offline accounts
cannot borrow a leftover stored Panel token. Existing pending account-journal
fencing still flows through `Secrets::get`.

This does not migrate other callers of `accounts::panel_token`, revoke tokens
already captured before logout, freeze independent external file edits, or
certify coherent identity snapshots throughout the application. Other request
callers still need their own context/state review. Holding read guards while
waiting for account persistence can delay capture/settings writers; there is no
claim of wait-free or measured latency improvement.

## Direct regression coverage

Seven new tests exercise the production capture/client/secret paths:

- Twelve stored-origin cases: canonical equivalence, path case and tenant
  differences, host/scheme/port mismatch, URL credentials/query/fragment,
  malformed input and missing legacy origin.
- Invalid destination and offline identity cannot capture a bearer.
- Pending transaction evidence still prevents credential access and is preserved.
- A deterministic held-account-lock test observes the destination settings guard
  remaining held while capture waits for account access.
- Concurrent 2,000-iteration settings/account writes and 2,000 captures preserve
  observed destination/token/username tuples. This is bounded stress coverage,
  not exhaustive scheduling or proof that all application mutations are atomic.
- A real loopback authenticated POST goes only to the earlier captured Panel,
  with its original bearer/username, after the configured Panel and selected
  account switch to a second synthetic listener/identity.

The first test run found two fixture defects, not accepted production failures:
the test helper reused an ID across upserts whose identity comparison includes
Panel URL, producing duplicate test IDs; the wire assertion also assumed header
CRLF placement. The helper now explicitly replaces its owned synthetic ID and
header matching is line-based. Original failed output is retained in
`telemetry-snapshot-tests.log`; corrected focused run has **7 passed** in
`telemetry-snapshot-tests-fixed.log`, under the existing temporary log directory.
Tests never use real accounts, keychains or installed data. The wire server has
a three-second total bound and 8 KiB request bound; other domains use `.invalid`
and are inspected only, not contacted. This is not a patched-native GUI replay.

Independent security/state review is in `telemetry-snapshot-review.md`. Final
default host suite: **150 passed**; feature-enabled host: **157 passed**
(overlapping suites, not 307 distinct outcomes); default non-test check passed.
Node 24.21.0 frontend suite: **102 passed**. Boundary/identity, local Markdown
links (101 checked), whitespace and scoped Rust formatting checks pass. Secret
scans: 1,535 tracked files, six reviewed matches and zero unreviewed; separate
185-file nonignored-untracked snapshot has zero findings. Earlier intermediate
failure/format output is historical, not a passing result. Review disposition
and independent rerun evidence are recorded separately.
The independent reviewer accepted this bounded fix with no blocking finding,
repeated seven snapshot and six Panel-client tests, and confirmed reviewed source
hashes and scoped formatting. No native or auth-wide acceptance follows.
No dependencies, lockfiles, IPC, credential-store identities, persisted filenames,
signing, releases, commits, branches, deployments or paid services change.
Actual billed USD is unavailable.
