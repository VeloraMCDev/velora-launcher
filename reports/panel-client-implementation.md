# Bounded native Panel client implementation

## Scope and status

The initially isolated client is now registered and integrated into the scoped
Panel HTTP portions of the extracted commands and root account login/register/
rename paths. All 124 IPC names, signatures, arguments, return shapes and
registration order pass the existing four command-surface tests (Node 24.21.0).
No Cargo build/test was run, per coordination instructions. Six loopback client
tests and four actual host poll-adapter tests are authored; coordinated native
execution and independent review remain gates. This is not self-approval of
VEL-007/VEL-009 or a claim of 50% completion.

Changed paths: root `launcher/src-tauri/src/panel_client.rs`; one module declaration
in `lib.rs`; only Panel HTTP snippets/imports in root `accounts.rs`; HTTP portions
of `commands/{accounts,bootstrap,casino,economy,guilds,maps,market,panel_client,
profiles,progression,social}.rs`; this report. Root account persistence helpers,
account selection/removal, profile persistence bodies, state and shared schemas
were not edited. New native consumer tests live in `commands/accounts.rs`.

## API and ownership

`PanelClient::new(existing_http_client, resolved_panel_base, optional_bearer)`
retains the configured HTTP client's timeouts, pooling, proxy, redirect,
compression and transport behavior. It does not construct a production HTTP
client or override cache headers. Continue using `AppState::panel_url()` to
preserve baked/locked precedence and existing account-token selection.
`request(method, endpoint)` preserves the configured base path and appends
`/api/v1` plus the endpoint. Supply suffixes like `/account/discord`, not full
URLs or `/api/v1/...`. Embedded queries like `/auth/discord/start?kind=link`
remain supported; use RequestBuilder `.query()` for dynamic value encoding.

Ordinary `RequestBuilder` results permit existing JSON, multipart, query and
`X-Velora-Instance` composition without payload-shape changes. Optional bearer
headers are marked sensitive. Anonymous requests remain anonymous; mandatory
account checks remain caller responsibilities. Associated `send(builder)` and
response decoders support callers that already hold builders. Use these instead
of formatting reqwest/serde errors into command strings.

Bases reject credentials, queries, fragments, unsupported schemes and missing
hosts. API suffixes reject absolute/network paths, backslashes, fragments and
normalization escaping the configured API prefix. Configuration failures never
retain the offending URL or token. No DNS allowlist or changed redirect/SSRF
policy is introduced: those require separate compatibility decisions.

## Existing endpoint behavior snapshot

Inspected accounts/state and extracted commands: panel_client, accounts,
bootstrap, profiles, guilds and progression.

| Existing behavior | Migration primitive / policy |
| --- | --- |
| Login/register accept any 2xx and require typed AuthResponse | `json::<AuthResponse>(..., Account)`; pending registration stays caller-owned |
| Manifest requires LauncherManifest, optional bearer, caches only after successful parse | `json::<LauncherManifest>(..., Generic)`; retain caller cache ordering |
| Discord start/connection/info and poll returned Value | shared Discord DTO decoding, then known-field IPC projection; poll adapter saves auth before returning only account |
| Gameplay endpoints with existing shared DTOs | existing return-type inference decodes shared FriendInfo/MemberProfile/DirectMessage/GameInvite/Guild/GuildClaim/GuildMember/GuildPost/UserQuest/Achievement/economy/profile DTOs |
| Delete/unlink/equip accept successful 204/2xx without decoding | `empty`; drops successful body without accumulating it |
| ok_or_panel_error mapped empty/malformed successful JSON to Null | `json_or_null` preserves empty 200/204, but malformed nonempty JSON intentionally errors; size/transport failures are not swallowed |
| Account failure prefers error, then errorMessage, else panel returned status | `Account` retains key precedence; only audited literal messages are displayed; otherwise generic failure |
| Gameplay errors prefer arbitrary error/message, else endpoint fallback | `Fallback("static endpoint message")` retains fallback wording, suppresses arbitrary server text |
| Password reset defaults to Check your email for a reset link. if message absent | leave successful default/shape in caller |
| Res<T> is Result<T, String> displayed as-is | map `PanelFailure::to_string()`; retain existing IPC result shape |

This table identifies deliberate error-policy and malformed-success changes,
not byte-for-byte equivalence for unsafe historical text. Normal routing, base
prefix, optional/public/required auth choice, instance headers and manifest cache
ordering remain unchanged. Dynamic sort/instance/post query values now use
RequestBuilder encoding rather than string concatenation. Legacy get_my_guild
non-success -> None and reward-bundles non-success -> empty object remain intact.

## Failures and account compatibility

`PanelFailure` stores only kind, optional numeric status and a static message.
Display emits the message exactly; Debug adds kind/status/static message. No
source chain, response body, raw reqwest/serde error, operator URL, query,
username, password or bearer is retained. No production logging is added.

Categories: network (send/read), auth (401/403), status (other non-2xx), decode
(invalid/oversized successful JSON), config (base/endpoint/bearer/builder).
Failed error-body reading/parsing does not replace auth/status classification.

Account messages are exact source-audited literals from auth-http web/account,
auth-core password and login_guard: invalid credentials, pending/disabled default,
closed sign-in/sign-up, validation, name conflicts and throttle. OAuth literals
are also audited against `crates/auth-http/src/oauth.rs` (expired/consumed state,
disabled/unconfigured flow, linking/account policy). Known literals
preserve old wording, without a status suffix. Operator-configured disabled
reasons, custom restrictions, SQL/internal errors and novel server strings are
not echoed. Unknown/malformed/oversized error bodies become `panel request
failed`; the old `panel returned {status}` wording changes, but numeric status
remains structured. Future additions require review of exact literals, not a
permissive raw-message escape hatch.

`Fallback` accepts a static string, not runtime server/URL data. It additionally
permits exact audited operator-free literals from Panel routes guild_flags, map,
collections and social (land-rule permission, missing player/faction/unlock,
offline map targets, self-action, invitation and message validation errors).
It retains error/message precedence; other bodies use the unchanged endpoint
fallback. No interpolation, substring sanitization or raw-message escape hatch.
Integrators must pass non-sensitive source literals. Successful JSON remains business data and
may contain tokens/URLs. This client does not make logging successful payloads,
RequestBuilder or Response safe. Password-reset success messages remain caller
business data, not sanitized failures.

## Bounds, tests and remaining gates

Success JSON has a 4 MiB ceiling; `json_with_limit` can lower, not raise it.
Account/fallback error bodies have a 16 KiB ceiling. Content-Length is checked first;
streamed chunks are checked before accumulation, including chunked/decoded
responses without known length. Generic errors never parse/read bodies.
These bounds limit retained buffers, not reqwest internal chunk allocations.
Timeouts remain the supplied client's policy. No retry/service/dependency upgrade.

Six authored client test functions cover:

- Typed JSON, base prefix, bearer, instance header, query encoding, no new cache headers.
- Anonymous requests, empty 204, dynamic JSON and intentional malformed-success rejection.
- 400/401/403/500 classification, untrusted-text redaction, audited account text,
  secondary errorMessage, static malformed-error fallback and no error source.
- Required malformed/empty JSON, known-length/chunked success size limits,
  oversized chunked account error fallback.
- Connection failure, redacted Display/Debug, invalid bases/bearer and escaped prefix.
- Typed Discord known-field projection strips unknown token/account fields,
  preserves null connection and missing-versus-null display_name, rejects malformed start.

The four existing command-surface tests pass. Scoped rustfmt parses all touched
command/account source without rewriting persistence bodies; the new client is
formatted. Scoped whitespace checks pass. No native tests have been executed by
this worker. Lead must run coordinated locked native tests, verify endpoint
call sites and manifest-cache/account-header policy, obtain independent review,
and exercise native UI acceptance before advancing VEL-007. Large legitimate
payload compatibility, native networking/proxy/redirect behavior and custom
server message support remain explicit acceptance gates.

## Discord consumer boundary and persistence gates

`discord_poll_result(&AppState, panel_base, DiscordPollResponse)` is a synchronous
network-free adapter used by the real handler. Authenticated responses call the
existing save_panel_account transaction and only then return literal `{account}`.
Linked/Pending states return literal `{linked:true}` or `{pending:true}`. Panel
producer DTO extension round trips remain supported, but the host intentionally
strips all poll extensions: neither injected account objects nor secret-like
extensions reach token-free IPC. Decoder failures propagate before the adapter;
presence of malformed token/auth fields cannot fall through to Pending/Linked.

Start/link-start use typed DiscordStartResponse and project only url/state;
connection projects only id/name (or null); info projects invite_url/linked/
display_name, preserving omitted versus null display_name. Their unknown
extensions are also stripped rather than assuming extension content is safe.
Known URL/name fields remain explicit success data, not permission to log them.
Unlink retains empty-success acceptance, including 204, without requiring JSON.
TypeScript consumer annotations and shared producer fixture evidence remain
lead-owned; IPC Value signatures are unchanged.

Four permanent actual-adapter tests use UUID temporary directories and cfg(test)
Secrets::open fallback-key behavior (no OS credential-store prompts). They cover
Pending/Linked account/secret injection stripping; malformed auth discrimination
and missing game-session failure; saved-account-only success with credentials
persisted privately; and corrupt-account publication failure preserving previous
memory/credential bytes and returning no signed-in success. RAII removes fixture
directories. This evidence is authored, not yet executed or independently reviewed.

## Explicit separate trust paths and incomplete contracts

All scoped Panel command/account request construction, sends, status handling and
response decoding now pass through PanelClient/PanelRequest. External skin image
downloads remain raw image-fetch paths in profiles; updater check/download/signing
stays in updater; Yggdrasil validate/refresh stays in root accounts. These are not
claimed unified, bounded or redacted by this work. The lead additionally migrated
game instance-manifest fetching and telemetry Recorder.send to PanelClient.
Telemetry retains captured identity, optional bearer, its five-second request
timeout, opt-in/background policy and best-effort lifecycle. Game instance-manifest
fetching retains its 404 wording and cache fallback. Core download networking
remains separately scoped; this is not a claim that every network trust path is
unified. These lead-owned migrations are recorded without modifying their source.

Stable endpoints with existing shared DTOs use them, and Discord uses the new
shared DTOs. Legacy Value signatures with no reviewed shared shape (stats,
leaderboard/public-server/map metadata/overlay, guild roles/relations/flags/wallet,
collections/reward bundles, notifications and market/vault results) retain bounded
Value decoding. Route-polymorphic casino/board/faction-upgrade endpoints also use
Value. **Comprehensive stable DTO modeling for these remaining legacy Value
families is incomplete**, not evidence that every such endpoint is inherently
dynamic. It requires producer/consumer shape review without dropping fields.
No percentage/checklist milestone is claimed from incomplete contract coverage.
