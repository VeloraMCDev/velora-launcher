# Independent bounded Panel client review

Date: October 12, 2026. Reviewed root repository instructions and the required monorepo, boundaries, validation and deployment documents. Independent source/local-contract review only. Reviewer ran no Cargo/npm builds, native application, keystore, deployment or live-store operation. Only review reports are modified.

## Verdict and scope

No must-fix defect found in the bounded integrated Panel request migration. Recommend VEL-007 checkpoints 1–4 for the central client, selected typed domain models, structured/redacted failures, local fixtures and this independent review, provided the lead reconciles final build exit and source identity. This does not mean every API has a shared DTO, every network path is migrated, or native/proxy/custom-server compatibility is accepted.

Scope: `launcher/src-tauri/src/panel_client.rs`, HTTP portions of account/command modules, manifest fetching in game.rs, selected shared Discord contracts and real poll adapter. Persistence is separately reviewed in `account-profile-persistence-review.md`. No blanket account/game behavior equivalence claim.

## Request and response policy

- Request construction keeps the configured base-path prefix and appends `/api/v1`; only HTTP(S), host-present, no userinfo/query/fragment bases are accepted. Endpoint construction rejects network-path references, backslashes, fragments and normalized origin/API-prefix escapes. This is URL construction validation, not exhaustive server-side path-decoding containment. No dependencies, default timeout, redirect policy or retry behavior are changed.
- Anonymous login/register/start/poll/reset/public-server/leaderboard requests remain anonymous. Authenticated `account_api` gets the existing active account token and adds the existing selected-instance header. Generic client construction itself does not add that header. Launcher/instance manifest fetching retains optional bearer and its prior header behavior. Query builders now encode guild instance IDs, post UUIDs and leaderboard sort values instead of interpolating raw query strings; names/normal payload meaning remain unchanged.
- `FailureKind` preserves Network/Auth/Status/Decode/Config and HTTP status where available. `PanelFailure` contains only static messages and safe metadata, not reqwest/serde source errors, response excerpts, URL, header values or tokens. Its derived Debug is safe because all stored fields are safe. Client Debug reports only authentication presence. Invalid bearer construction and send errors are converted before crossing IPC. This guarantees the scoped failure object, not arbitrary success payloads or every launcher log.
- Account/fallback API messages are exact-match allowlists mapped back to static literals. Unknown/custom/reflected messages fall back; they are not passed through. Generic error policy does not parse untrusted bodies. Non-generic error bodies are capped at 16 KiB, normal JSON at 4 MiB; declared length and each retained chunk are checked. These bounds do not prevent an internal reqwest chunk allocation, limit outgoing/multipart data, or prove large legitimate payload acceptance.
- Required JSON responses fail closed on malformed/empty data. Existing unit-returning operations and unlink accept successful 204 without requiring JSON. `json_or_null` preserves empty successful responses as null but now rejects malformed nonempty success instead of the old null fallback: a deliberate stricter response policy, not complete behavior identity. JSON parsing/serialization failure messages are static. Body policy remains all 2xx, not exactly 200.
- Profile image fetching remains a distinct unbounded binary path. Yggdrasil session validate/refresh, core vendor/download networking, updater signatures/check/download and telemetry remain separate policies. Scanning raw command sends found only these expected external skin downloads and the unrelated kill channel outside the client helper. The central client must not be described as a universal networking/security layer.

## Cache, auth and IPC compatibility

Manifest refresh still caches only successful typed responses and adds no cache headers. Game instance manifest fetching keeps the same `.scopenet/manifest.json`, writes after successful decoding, and falls back to disk on any fetch failure, including auth/status/404/decoding/size failures; this pre-existing broad fallback is not an authorization-revocation fix. Native offline/cache acceptance and cache durability remain gates.

All 124 IPC names, signatures, argument order/types and return types, registration order and macro paths pass the permanent contract tests after client integration. Existing `Value` signatures are deliberately preserved; typed decoding happens behind them. The network-free `discord_poll_result` saves authenticated account/credentials before returning literal `{account}` and projects Pending/Linked to literal `{pending:true}`/`{linked:true}`. Unknown injected account/credential extensions are stripped, not forwarded. Start/link/connection/info also project only known fields; null connection and omitted-versus-null display_name remain intact. Presence of a malformed `token` field forces auth decoding failure, not fallback to Pending/Linked. Missing game session and persistence failure cannot return signed-in success.

## Independent validation evidence

- Ran the lightweight Node 24 contract tests from `launcher`: four command-surface tests and two API-schema tests passed, zero failed/skipped. The initial invocation from repository root produced missing Svelte/Vite type-library diagnostics; corrected to the documented segment cwd and reran successfully. Negative TS assertions are credited only with the successful positive fixture test, not the misconfigured invocation.
- Inspected lead-run `/private/tmp/velora-revitalization-50/native-final.log`: latest complete log records 70 host tests and 19 shared tests passed, plus 65 core and 10 utility tests; seven online integration tests remain ignored. All six Panel client tests and four actual poll-adapter tests pass in this log. Reviewer did not launch that build. The earlier 68-pass/one-failure rollback assertion was real and subsequently fixed, not ignored or weakened; see companion persistence review.
- Client fixtures exercise base prefix, bearer/instance/query preservation, anonymous/204 behavior, malformed/oversized JSON, known-length/chunked limits, status classification, reflected-error redaction, invalid base/bearer/prefix, safe Debug and Discord field projection. They do not execute every host route or native proxy/redirect scenario.

## Remaining contract inventory and credit limits

Stable manifest/auth/profile/friend/message/economy models already use shared types; the selected Discord family now uses new shared DTOs internally. Stats, leaderboards/public servers, map metadata/overlay, several guild relations/roles/flags/wallet, rewards/collections, notifications, market/vault and other legacy Value responses are **not comprehensively modeled**. Some represent stable producer shapes requiring future review, not inherently dynamic APIs. Casino/board/action families remain deliberately route-polymorphic. No credit for all-API schema completion, native UI/deployed consumers, large-payload performance or new caching policy. The schema companion contains the current migration matrix and corrections to worker handoff inventories.

## Reviewed hashes

SHA-256:

| Path | Hash |
| --- | --- |
| launcher/src-tauri/src/panel_client.rs | 0297efe0135d042cca1badf4786c8aeea6392b85135382e662e9c16209be8b8f |
| launcher/src-tauri/src/accounts.rs | 930f6c3992819a4ee9dd6f94477251d786fbfc3584ac303a241dd6d665c9fdde |
| launcher/src-tauri/src/game.rs | d787bdcf6e5e1b9db11b1158cb0bf91151da7860a323e7e7ea6095e14a8c7c0d |
| launcher/src-tauri/src/commands/accounts.rs | 5b49216bec201e70d578449230a1a8931e83724989316f0a36c139ffb9f17258 |
| launcher/src-tauri/src/commands/profiles.rs | 57fd843ea6342279cadcf234fed8c2a1038b782dff09125f73fe6bebc5e64bcd |
| launcher/src-tauri/src/commands/panel_client.rs | b6aac4936c5cf5d810bd36447de18d73c7d299bc02f8a430a01dc4277220da59 |
| launcher/src-tauri/src/commands/bootstrap.rs | 5208887471a718c5e98c62bd1cbb2d32e1bba36beb1baeeaad22d691a04e398b |

Ordered extraction-file aggregate (commands.rs, lib.rs, sorted commands/*.rs and command-surface.test.mjs), computed as SHA-256 of the `shasum -a 256` output: `c2b2028b20b2d0b347dff4dedea1ad2a95a884cf4f9545462c019d9e8efa1af6`. Later substantive changes require affected-scope re-review; hashes do not prove final build provenance on their own.
