# Independent bounded manifest-cache fallback policy review

Date: October 13, 2026, America/New_York. Separate task from the content-cache
review. Baseline HEAD: `4f639266a515c3cb05769b055a4772c4dcb89cb8`; review is of
the dirty working tree, not pristine HEAD. **No blocker identified for the
specific typed-failure fallback restriction.** This is not acceptance of cached
manifest authority, offline authorization or native launch security.

## Ownership and scope

This reviewer owns and edits only `reports/manifest-cache-policy-review.md`.
No production/test edits, commits, branches, deployment or checklist credit.
Read root `AGENTS.md`, `MONOREPO.md`, `docs/BOUNDARIES.md`,
`docs/VALIDATION.md`, `docs/DEPLOYMENT_TARGETS.md`; no nested instructions found
under launcher or reports. Scope: `instance_manifest` in
`launcher/src-tauri/src/game.rs:88`, its helper module
`game/manifest_cache.rs`, and relevant `PanelClient` failure classification.
Other launch/auth/game logic and a broader PanelClient audit are outside scope.

## Policy and integration findings

- `manifest_cache.rs:4` only invokes the cache closure for an `anyhow` error
  downcastable to `PanelFailure` whose kind is `Network` and whose status is
  absent or in `200..300`. Success returns online data without consulting the
  closure; every other error returns unchanged. This is typed classification,
  not matching error text. Missing or corrupt fallback data remains an error,
  retaining the original typed network cause under added cache-error context.
- `PanelClient::new`/`request` (`panel_client.rs:73`, `:96`) reject invalid base,
  endpoint or bearer configuration as `Config`. `send` (`:113`) maps request
  builder errors to `Config` and other request-send errors to `Network` with no
  status. This latter class is broader than proven Internet disconnection: it
  can include TLS or other transport failures. The policy cannot infer the
  account's authorization state from that classification.
- `PanelClient::success` (`:173`) classifies 401/403 as `Auth` and other
  non-success responses as `Status`. Generic error policy does not consume
  their response body to derive another failure kind. `json_with_limit`
  checks success before reading/deserializing. Body-read failures carry
  `Network` plus the response status (`:205`); a successful-response body
  transport failure can therefore use fallback, but malformed/oversized data
  is `Decode` and cannot. The explicit status guard also rejects a hypothetical
  typed Network failure with a non-2xx status.
- The actual game fetch uses `PanelClient`, not a raw reqwest error, and retains
  the existing untyped 404 denial at `game.rs:95`. That denial and untyped
  `no panel configured` error cannot pass the helper's typed Network gate.
  Online auth/status/config/decode denial thus does not become cached success.
- `game.rs:101` keeps the online branch's existing directory creation and
  serialized-manifest write, with filesystem write failures still ignored.
  The error branch (`:107`) passes a lazy closure reading the existing
  `.scopenet/manifest.json` bytes and deserializing the manifest. It does not
  write, normalize or replace the cached file. Logging occurs only after a
  successful cached parse. “Unchanged raw manifest” means unchanged on-disk
  bytes and no fallback rewrite, not a returned raw-byte API or preserved
  unknown fields after deserialization.

The final integration puts online writes exclusively in the online-success
branch, rather than writing a fallback result. This conclusion is source review;
the tests below do not execute `instance_manifest` against a real `AppState` or
assert file contents before/after its branches. No production edit is requested
for the bounded policy reviewed here.

## Independent final validation

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher --lib game::manifest_cache::
```

Run once after the reported second game integration edit, against the source
hashes below, using the existing target. Cargo initially waited for a build
directory lock; it then completed the current library test profile in **6.37 s**
and ran `velora_launcher_lib-0fa20be1932b3e4c`. No forced clean/rebuild was used
and the captured output does not claim every artifact was recompiled. This is
independent validation of the final snapshot, not adoption of the lead's earlier
five-pass result. Exit **0**: **5 passed, 0 failed, 0 ignored, 0 measured,
70 filtered out**, test execution **0.00 s** as printed by Cargo. Environment:
Darwin 24.6.0 arm64, rustc `1.98.1 (48a229cea 2026-09-01)`, cargo
`1.98.1 (797e8a9bc 2026-08-05)`.

Observed fixtures:

- Loopback 401, 403, 404, 429, 500 and 503: zero cache-closure calls, preserved
  typed status errors, private response-body text absent from displayed errors.
- Loopback malformed 200: `Decode`, zero cache calls.
- Invalid configuration and an untyped denial matching the game's 404 text:
  errors, zero cache calls.
- Loopback valid 200: online value, zero cache calls.
- Closed loopback listener: typed Network; exactly one cache call; valid fallback
  returned, fallback failure retains a downcastable Network cause.

These are five test registrations, not five independent native integration
scenarios. The suite does not directly test interrupted 2xx response bodies,
synthetic Network/non-2xx combinations, oversized manifest responses, redirects,
TLS failures, live filesystem reads/writes, or actual game execution. The relevant
guards were inspected, not all fault-injected. No other PanelClient tests or
broader launcher suite were run by this reviewer.

## Security limitations and remaining gates

**Cross-panel/account legacy-cache provenance is still missing.** The legacy
file path is based on the instance ID, not a verified panel/account identity.
Fallback deserialization supplies no manifest signature, issuer/account binding,
expiry, revocation check or freshness proof. A cache from another panel/account
or earlier entitlement is not made trustworthy by tightening online denial
behavior. It is not the separately reviewed digest-based content cache.

**No offline authorization/revocation proof.** Network failure prevents observing
a current server denial; an attacker-induced outage or an unavailable panel
cannot establish continuing access. The fix prevents explicit online denial
from falling back, not all stale-authority use. These are unaccepted security
gates, not newly solved permissions guarantees.

The fallback uses an unbounded whole-file read and ordinary path-based JSON
deserialization; no local tamper, symlink/path-race, file-size quota, atomic-write
or cache migration defense is established by this change. Online writes remain
non-atomic/best effort as before. Cache error context interpolates local error
text; only the reviewed PanelFailure/status messages and synthetic test assertions
support redaction claims, not a universal error/privacy guarantee.

No native Windows/Linux, native webview/game launch, OAuth/keystore acceptance,
installed-store migration, downloader/integrity/HTTP resume, offline entitlement,
full security acceptance, broader authority claim or checklist credit is awarded.

## Exact source SHA-256s

Hashes were captured before the narrow run and rechecked unchanged after it.
`game.rs` is hashed in full for provenance, but reviewed only for the module
connection and `instance_manifest`; its unrelated edits are not certified.

| Source | SHA-256 |
| --- | --- |
| `launcher/src-tauri/src/game.rs` | `b1d1821025131cdf06d9908718627877ca2c83cbe7a510324a60453ca43301b3` |
| `launcher/src-tauri/src/game/manifest_cache.rs` | `9a4fabd0f33d8c3b9ca32638d0a7842823b3e1ad2af0d534328c22d7b588e2c0` |
| `launcher/src-tauri/src/panel_client.rs` | `0297efe0135d042cca1badf4786c8aeea6392b85135382e662e9c16209be8b8f` |

## Final generic-error-context follow-up — October 13, 2026, 09:17 EDT

Reviewed the final helper change at `manifest_cache.rs:14`. Fallback failure now
discards the cache error and adds only the fixed context
`offline instance manifest unavailable or invalid` to the original typed network
error. Raw cache read/parse error text is no longer interpolated or retained as
its cause by this helper. The preceding cache-error-text limitation describes
the earlier helper hash and is superseded for this final snapshot. It does not
become a universal privacy/redaction guarantee for other paths.

Typed Network/status gating, laziness, successful fallback behavior and the
original downcastable network cause remain intact. `game.rs` and `panel_client.rs`
hashes match the initial independent review; no integration-path change was
observed. **No new blocker identified for this bounded final policy.**

Reran exactly the same narrow command once after this helper change:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher --lib game::manifest_cache::
```

Cargo explicitly compiled `velora-launcher-core` and `velora-launcher` and finished
the current test profile in **6.45 s**. Exit **0**: **5 passed, 0 failed,
0 ignored, 0 measured, 70 filtered out**, execution **0.01 s**. This supersedes
the earlier test result only for final-source validation; earlier results remain
historical, not cumulative coverage. Existing tests verify retained typed error
classification but do not explicitly assert raw cache-error absence throughout
formatted chains; that specific conclusion is source inspection of error discard
and fixed context. No broad suite or native launch ran.

Final hashes captured before this rerun and rechecked unchanged afterward:

| Source | SHA-256 |
| --- | --- |
| `launcher/src-tauri/src/game.rs` | `b1d1821025131cdf06d9908718627877ca2c83cbe7a510324a60453ca43301b3` |
| `launcher/src-tauri/src/game/manifest_cache.rs` | `79a14911a240fdf93b5076a3c3ab2612a3868ec60d8e2cb799df0630a4c09ac5` |
| `launcher/src-tauri/src/panel_client.rs` | `0297efe0135d042cca1badf4786c8aeea6392b85135382e662e9c16209be8b8f` |

Cross-panel/account legacy-cache provenance, offline authorization/revocation
proof and the other stated security/platform/integration limitations remain open.
Only this report was edited. No broader authority, security acceptance or
checklist credit is assigned.
