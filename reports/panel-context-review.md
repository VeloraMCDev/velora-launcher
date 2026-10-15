# Independent Panel context security/state review

Date: 2026-10-15. **Verdict: BOUNDED SOURCE ACCEPT for destination-bound
Panel bearer selection and coherent current-context capture.** No must-fix
production defect found in this scope. Lead execution logs were subsequently
inspected; installed-upgrade acceptance remains pending. This is not broad authentication, native, signing,
release or deployment approval. **Ledger stays 62/76. Actual billed USD is
unknown; the initial $10 planning allowance is not a price estimate.**

## Scope and evidence

- Read root `AGENTS.md`, `MONOREPO.md`, `docs/BOUNDARIES.md`,
  `docs/VALIDATION.md`, `docs/DEPLOYMENT_TARGETS.md` and the prior
  `reports/telemetry-snapshot-review.md`. Inspected Panel context/client,
  bootstrap, command helper/maps, account mutation/session refresh, game manifest
  authority/cache, telemetry, settings resolution, Secrets and transaction fences.
- Reviewer owns only this report, written with `apply_patch`. Initial inspection
  involved no production/test edits, Cargo invocation, test execution, real store/keychain access, external
  network, subagent, commit, packaging, release or deployment was performed.
  The checkout contains extensive concurrent work; this report does not approve
  the aggregate diff. Subsequent lead check/test logs were read locally; those
  runs are inspected lead evidence. Reviewer-run focused tests after explicit
  Cargo handover are separately recorded in the final runtime addendum.
- New worker and lead fixtures were inspected as source. Counts below are
  inspected test functions; execution results are separately recorded below. Later source changes require
  reconciling this review and rerunning relevant checks.

## Security finding and request binding

- The original bootstrap issue did not need a race: `set_panel_url` supplied a
  prospective destination to `fetch_manifest`, while generic token lookup used
  configured state. Thus an existing configured-account bearer could accompany
  a request to a different new destination. `commands/bootstrap.rs:68` now calls
  `token_for_panel(state, panel)` with the same explicit base passed to
  `PanelClient::new`; it does not substitute the configured Panel for comparison.
- `panel_context.rs:19` holds accounts read while resolving the active account
  and reading its secret. `token_for_account` requires kind `panel`, an explicit
  account origin and `PanelClient::same_base(account_origin, destination)` before
  reading the stored bearer. Missing/invalid/foreign origin, offline identity,
  absent active account/secret or the existing pending-transaction fence cannot
  borrow a bearer through the corresponding lookup.
- `PanelClient::same_base` uses the constructor's validated URL parser for both
  inputs. Equality includes scheme, host, normalized port and full base path,
  with path case significant. Host/default-port/trailing-slash equivalents can
  match; a different tenant, case, host, scheme or nondefault port cannot.
  Identical invalid strings do not match. Userinfo, query and fragment bases
  are rejected. This is normalized base equality, not host-only trust.
- `rename_panel`, `account_api`, leaderboard and public-server requests consume
  one owned current context; bootstrap uses explicit-destination lookup.
  Telemetry delegates its existing explicit-account policy to the helper.
  No generic `accounts::panel_token` function/call remains in inspected source.
  Command-helper requests also take `X-Velora-Instance` from that context.
  Subsequent settings/account changes cannot retarget an already-built request
  while preserving its previously captured bearer.
- These are acquisition/construction guarantees, not request-time revocation or
  a new transport policy. Owned credentials can still be sent to their captured
  old destination after account removal, opt-out or a Panel switch, subject to
  the caller's existing gates. Redirect/proxy/DNS/server decoding behavior is
  inherited, not newly approved. No network validation was performed here.

## Snapshot and lock order

- `panel_context.rs:11` acquires **settings read -> accounts read -> Secrets
  data mutex**. Settings remain held across account/credential selection and
  selected-instance cloning; all fields are owned before guards drop. There is
  no async boundary or network work while those locks are held. `token_for_panel`
  needs only accounts -> Secrets because its destination is already explicit.
- Account commit/removal holds accounts write through the encrypted-secret and
  account transaction, publishing the candidate account before releasing that
  guard. Current capture cannot observe their intermediate publication.
  `Secrets::get` retains its filesystem `ensure_clean` fence; the new helper does
  not bypass it, open the credential store or mutate persistent account data.
- Inspected production settings/account mutator callbacks do not acquire the
  opposite resource. Secret mutation/transaction callbacks do not reacquire
  AppState settings/accounts. Bootstrap and game account clones release their
  guards before later settings access. No new accounts-held -> settings or
  Secrets-held -> accounts/settings wait cycle was found.
- Telemetry already used this nested order; replacing only token selection does
  not shorten its settings/accounts guard lifetimes or change consent,
  username, retained-recorder or optional-bearer semantics. Unknown-origin
  identities may still contribute a username to an anonymous telemetry event;
  this change is not whole-product privacy acceptance.
- Reads can still block behind synchronous credential/account persistence;
  poisoning retains existing `unwrap` behavior. A snapshot is coherent at
  acquisition, not an atomic transaction across separately issued settings and
  account IPC mutations, nor proof of freshness after capture.
- `token_for_account` deliberately accepts an explicit borrowed account; it does
  not assert that the account is currently active or still in the account file.
  Current production telemetry calls it while holding accounts read. Future
  callers using detached clones must not infer membership/revocation guarantees.

## Compatibility and refresh risks

- **Intentional compatibility break:** legacy Panel accounts with no
  `panel_url` now receive no Panel bearer. Authenticated account requests fail
  locally; optional-auth/bootstrap/game requests can proceed anonymously and
  may be refused by the server. Reauthentication to the correct Panel is the
  recovery path, not automatic assignment of the configured origin.
- Strict path case comparison also denies previously case-insensitive generic
  matches; normalized host/default-port equivalents can now match. Upgrade
  acceptance must cover real deployment base paths, not assume all previously
  accepted strings remain authorized. No installed-store migration was tested.
- `game.rs:95` changes only the missing-origin predicate from `is_none_or` to
  `is_some_and` in bearer selection. Its existing validated
  `Authority::same_panel` normalizer, cache version/format/location and offline
  recovery/denial policy remain in place. **Offline access is not unchanged:**
  an originless account now has `authenticated=false`; its former authenticated
  cache authority will not match, so even network-only fallback can be lost.
  This is intentional fail-closed behavior, not a cache migration or corruption.
- No inspected production change renames persisted IDs, schema, paths,
  credential key names or updater/signing identities. Token lookup does not
  erase or repair credentials. Reauthentication itself is not demonstrated to
  preserve an originless legacy account's ID: existing `AccountsFile::upsert`
  matches kind + UUID + exact stored Panel URL, so a new explicit-origin login
  need not replace the originless entry. Duplicate/selected-account and cache
  continuity belong in the upgrade gate, not an implied automatic migration.
- `ensure_session` remains a separate Yggdrasil validate/refresh policy. It reads
  an owned Secret, releases its mutex for network work, then writes a replacement
  based on that older Secret. Concurrent relogin/rename/removal can therefore
  still risk stale credential replacement or a refreshed secret for a removed
  account. The new context API neither introduces nor resolves this pre-existing
  refresh race; it must not be advertised as universal credential freshness or
  coordinated refresh/revocation. No broad auth approval is given.
- Bootstrap response caching/settings publication is still separate from fetch:
  overlapping refresh/switch operations can publish stale manifest state. Map
  metadata still resolves a separate Panel string for `tile_base` rewriting.
  These existing state-consistency concerns are not fixed by bearer binding;
  they do not negate the inspected same-destination bearer construction.

## Fixtures inspected and remaining gates

- Five `panel_context/tests.rs` tests cover normalized matching, missing-origin
  denial with byte-preserved settings/account/encrypted/key files, tenant/case/
  host/port/scheme/invalid-base denials, offline/missing active/missing secret,
  explicit-account behavior and retained context fields after state mutation.
  The mutation test is sequential; it is not concurrent interleaving coverage.
- Two bootstrap credential tests exercise production `fetch_manifest` against
  synthetic loopback servers: prospective destination gets no Authorization and
  no persistent context publication; matching explicit destination gets its
  bearer even when configured settings point elsewhere. They do not execute
  the full `set_panel_url` settings/cache commit path or a native IPC switch.
- Three lead command-helper tests build the actual `account_api` request and
  check retained destination/bearer/instance, missing/foreign/path-case origin
  rejection and pending-journal refusal with evidence preserved. Building a
  request is not server/native route execution. Under `cfg(test)`, Secrets key
  creation uses temporary fallback files rather than the OS credential store.
- The lead's new actual `instance_manifest` loopback fixture checks no
  Authorization for an originless legacy account with a stored token, successful
  anonymous manifest acquisition, unauthenticated cache authority and retained
  secret. Existing authenticated fixture setup now supplies the explicit origin.
  This documents the source-level legacy gate, not an installed upgrade test or
  proof of continuity from an existing authenticated legacy cache.
- **Validation evidence:** the completed lead logs described below support the
  scoped source acceptance. At this stage the reviewer had not launched Cargo;
  subsequent independent passing execution appears in the final addendum. Final source/command provenance must remain
  reconciled with lead validation; this is not a full workspace/OS matrix.
- **Before release/native acceptance:** document and test legacy unknown-origin
  reauthentication, old authenticated-cache refusal, exact/normalized base
  matching, account selection/duplication and supported-OS credential continuity
  with safely isolated synthetic fixtures. Installed stores/keychain, signing,
  native launch, upgrade/offline recovery and production rollout remain gates.
  No matrix increment, automatic deployment or paid infrastructure is authorized.

## Lead validation and test-harness correction

Inspected local logs under `/tmp/velora-revitalization-20261014/`; at this stage
no commands were rerun by this reviewer. These logs show results, but do not include full shell
invocations/environment headers; exact command provenance remains with the lead.

- `panel-context-check.log`: completed launcher non-test dev check; no warning
  shown. Lead also reports scoped new-test formatting passed. The retained
  `panel-context-format-initial.log` is an initial diagnostic, not evidence of a
  passing final format run.
- `panel-context-host.log`: **160 passed / 1 failed**, including all eleven new
  context/bootstrap/helper/game tests passing. The pre-existing telemetry
  request-contract fixture failed with raw accepted-stream `WouldBlock`.
  The failed log is retained and not represented as a successful first run.
- Inspected `telemetry.rs:199`: inside its `cfg(test)` module only, the accepted
  TCP stream is explicitly restored to blocking mode before the existing
  five-second read timeout; a five-second write timeout is also set. The
  listener intentionally remains nonblocking for bounded accept polling.
  This removes reliance on accepted-socket nonblocking inheritance behavior;
  it does not change production telemetry, retry auth, relax assertions or
  substitute a source workaround for a failing request contract.
- `panel-context-host-fixed.log`: **161 passed / 0 failed**. Includes all eleven
  new tests, existing telemetry snapshot/concurrency tests and Panel-client
  regressions. `panel-context-feature.log`: **168 passed / 0 failed**, including
  native-acceptance guard fixture tests. Feature unit tests do not establish
  installed/native/signing acceptance.
- `panel-context-frontend.log`: launcher `npm test`, **102 passed / 0 failed**.
  No reviewer GUI, browser or installed-runtime exercise is inferred.
- The final inspection also includes formatting-only changes to the command
  helper and game fixtures. Reviewed test hashes are recorded below; neither
  change adds production behavior. This report's own whitespace check found no
  reported issue, but an untracked report is not covered by ordinary `git diff`.

## Reviewed production fingerprints

SHA-256 at inspection time; paths relative to `launcher/src-tauri/src/`.
Concurrent fixture/hook changes after inspection are not covered automatically.

| Path | SHA-256 |
| --- | --- |
| `panel_context.rs` | `57fffd613977db087ac7b74a69eab0360106c60671d1315ce8676c3d7cc7a13d` |
| `panel_client.rs` | `d58a112be1f3bb00247441db9e186dce2c9a21d950add4e2d4921ba41674a464` |
| `accounts.rs` | `e3968b2c10211c744ea8bd5f59573a5af4c4e34c8f745c0084feb3dd341e6c68` |
| `game.rs` | `775cce0ea3ec331ac082945bb6aff1262914d1abbbe3f5a7ab8975587f4547b9` |
| `telemetry.rs` | `6af763bf57184ffd0dc93e3a75a76152579544089d336af84279817879e296bd` |
| `state.rs` | `4757be8daa4206d58923274d9d2f302f667c9ef9efe545e2ed0523d0b16366fc` |
| `commands/bootstrap.rs` | `d2dcabd7c6f8c7486fb875a4265d067ec35d4a28af9c1ccef8e05882282a69fe` |
| `commands/maps.rs` | `39b28b4de8acf6b3e679b2527b29f267b23b21bd7afcb2c54eea50c321c988cb` |
| `commands/panel_client.rs` | `f403006fadb11c00327e79e22371148abac6692a2528a491d3122ae8e22637c4` |
| `panel_context/tests.rs` | `f815cf33812fa1591af962cd2ceff1bd3474a98faebb9b7d2416e5e8fcf57f21` |
| `commands/bootstrap/credential_tests.rs` | `3889636529f48caf10421a2367cfbe2feb255201f850c472b066870f6d176da9` |
| `commands/panel_client/context_tests.rs` | `aa3d1bd6438b02ddbb33cc1736b010fb987db389e2cba5153f6590cd09ec62ce` |
| `game/manifest_cache.rs` | `1f1d060bdc050ff085e631dcb25e68e3a7ef35f97b67f4a6957a90cc7f2205c5` |

The unchanged Panel-client/state hashes match the prior telemetry snapshot
review. This is source provenance only, not provenance for a built binary.

## Final independent runtime addendum

After the lead explicitly handed over Cargo following completed green host and
feature runs, the reviewer independently ran the six filters below serially.
Tool versions: **Node 24.19.0**, supplied by the existing local runtime directory
prepended to PATH, and **Rust 1.98.1**. No tooling installation/download was used.
Each invocation used this exact command prefix, followed by the listed filter:

```sh
CARGO_NET_OFFLINE=true cargo test --locked -j1 -p velora-launcher --lib FILTER
```

| Filter | Independent result |
| --- | --- |
| `panel_context::tests` | 5 passed / 0 failed |
| `commands::bootstrap::credential_tests` | 2 passed / 0 failed |
| `commands::panel_client::context_tests` | 3 passed / 0 failed |
| `game::manifest_cache::tests::actual_instance_fetch_without_account_origin` | 1 passed / 0 failed |
| `telemetry::tests::enabled_statistics_keep_captured_identity_and_request_contract` | 1 passed / 0 failed |
| `telemetry::snapshot_tests` | 7 passed / 0 failed |

**19 independent tests passed; all six commands exited successfully.** Exact
commands, tool versions and test output are preserved in
`/tmp/velora-revitalization-20261014/panel-context-independent.log`. The corrected
telemetry accepted-stream fixture passed without changing production behavior.
Tests used synthetic temporary state/fallback keys and loopback requests;
Cargo dependency fetching was disabled. No real installed store/keychain,
external-network request, installed/native application, signing or deployment
acceptance was exercised. The native-acceptance feature suite and frontend
results remain inspected lead evidence, not reviewer repeats.

Final fingerprints were rechecked against the table after these runs, including
the test-only telemetry socket correction and final helper/game fixture changes.
The bounded verdict and compatibility/refresh limitations remain unchanged.
**Ledger remains 62/76; actual billed USD remains unknown.**
