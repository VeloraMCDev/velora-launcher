# ADR 0004: fixed game-authority gateway ownership

Date: 2026-10-05. Status: implemented component; application/data cutover deferred.
Applies to D1/D8/D14 and PANEL-03/AUTH-08/09/10.

## Boundary

Panel owns a standalone HTTP gateway, without storage or credential keys. It sends
the existing Yggdrasil and texture namespaces only to Authentication, for every
method. Other application routes go to the explicit Panel upstream. The complete
web/admin/provider boundary remains with the compatibility host until ADR 0001/0002
audit/presence/provider and deletion dependencies are implemented.

Owner denial, unknown routes, connection failures and timeouts never fall back to
another authority. There are no cached admission grants, transparent redirects,
request retries, decompression or application-body rewrites. Requests/responses
stream with raw queries, errors, cookies, scoped headers and range/cache/encoding
semantics. Hop headers are stripped; URL credentials and request-selected upstream
addresses are prohibited. Ambiguous owner aliases/traversal are rejected.

The operator supplies one canonical public origin, including any prefix, to both
gateway and Authentication. The prefix is stripped once; signed texture origins
and API-location metadata stay public. The gateway replaces Host/forwarded origin
headers and forwards only the resolved client IP. Forwarded addresses require an
explicit trusted socket peer, retaining rightmost-untrusted-hop precedence. A
malformed trusted chain is rejected before forwarding. Upstreams must be isolated
private HTTP origins; TLS/mTLS hosting profiles remain separate acceptance work.

Readiness probes required owners live, without granting access. Authentication
outages leave independent Panel forwarding usable; Panel outages leave owned game
routes usable. Public-only game mode needs no Panel or Experiences package but
does not provide a replacement Panel interface. Graceful drain retains current
streaming requests; process-manager deadlines remain explicit operator concerns.

## Evidence and remaining gates

Independent tests exercise routing, raw binary/compressed bodies, duplicate cookies,
range/cache headers, scoped credentials, precise error/redirect forwarding,
trusted proxies, timeouts, upstream outages, prefix/path handling, public-only
mode and graceful streaming drain. The Infra process checker invokes the actual
public gateway, Authentication and offline tools with entirely synthetic storage,
accounts, skin pixels and provider-free fixtures. It validates fresh bootstrap,
game login/join, spoof rejection, multipart skin upload, RSA profile signatures,
public prefixed texture URLs, owner outages, closed backup/assets restoration,
credential/token/key/texture continuity and live revocation. Panel CI builds a pinned
anonymous public Authentication clone; no private repository fetch is required.

The compatibility host's routes, writers and data paths remain unchanged. Production
quiescence/import, mixed-store and native upgrades, complete private backend/pages,
public UI/runtime composition, web/provider/audit/presence/deletion boundaries,
Docker/control-panel packaging and full feature-parity/release acceptance remain
open. This component does not authorize deployment or publication.
