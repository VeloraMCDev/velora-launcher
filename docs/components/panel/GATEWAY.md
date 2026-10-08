> Historical owner documentation from `VeloraMCDev/panel` at `a5ebb0e81c653a22095893dc71e7e400e5b0e6cc`. Current segment instructions are in the monorepo READMEs.

# Velora HTTP gateway

`velora-gateway` is a runnable, public-only HTTP forwarding component. It has no
SQLite store, credential keys, private checkout, registration provider or gameplay
implementation. It does not complete the Panel backend/Svelte migration.

## Build and run

Use Rust 1.98.1 and the native C build toolchain:

```sh
cargo +1.98.1 build -p velora-panel-gateway --release --locked
./target/release/velora-gateway --help
export VELORA_GATEWAY_PUBLIC_ORIGIN=https://example.invalid
export VELORA_GATEWAY_AUTH_UPSTREAM=http://127.0.0.1:8081
export VELORA_GATEWAY_BIND=127.0.0.1:8080
./target/release/velora-gateway
```

The example domain is synthetic. Supply your own public origin and TLS reverse
proxy. Upstreams are operator-configured **private HTTP origins**, without URL
credentials, path prefixes, queries or fragments. This native component does not
provide upstream HTTPS/mTLS or internet-facing TLS. Use loopback or an isolated
network; do not expose the Authentication listener as an alternative public origin.

Configure Authentication's `VELORA_AUTH_PUBLIC_ORIGIN` to the exact same public
origin, including any path prefix. Its signed texture URLs must stay public rather
than disclose an internal service address. Configure
`VELORA_AUTH_TRUSTED_PROXIES` with the gateway's actual socket IP. Keep the
Authentication listener inaccessible to untrusted peers that can impersonate that
proxy. Trust in a proxy address is a network boundary, not a credential.

An optional `VELORA_GATEWAY_PANEL_UPSTREAM=http://127.0.0.1:8082` forwards the
remaining routes to the supplied Panel host. The two upstreams must differ.
Without Panel configuration, game routes work and other application routes return
503. There is no replacement Panel interface hidden in this gateway.

## Fixed ownership and wire behavior

- `/api/yggdrasil`, its descendants, `/textures` and its descendants always go to
  Authentication, for every HTTP method. An Authentication 404, denial, timeout or
  outage never retries through Panel or grants access from cached state.
- Other application paths go to the configured Panel upstream. Existing web
  account/admin/provider routes remain there until their full service dependencies
  are implemented. The gateway does not introspect JWTs or duplicate admission.
- A canonical public prefix such as `/velora` is stripped exactly once before
  forwarding. Outside-prefix paths return 404. Raw queries, repeated query keys,
  methods and bodies are retained; no URL join, redirects, content decoding or
  application request retries occur.
- Request/response bodies stream. Bearer credentials, legacy and Velora instance
  headers, cookies, range/cache/content-encoding headers and upstream status/error
  bodies are preserved. Standard hop headers and Connection-nominated fields are
  removed. Streaming bodies are not buffered to create JSON wrappers.
- Ambiguous traversal, encoded separators, recursive percent escapes and encoded
  aliases of reserved owner/health prefixes return 400. Encoded Unicode/spaces and
  ordinary filename escapes remain raw. CONNECT and protocol upgrades return 501.
  The current captured source has no WebSocket/SSE endpoint; future upgrades need
  a reviewed streaming protocol contract.
- Host/forwarded origin metadata uses the operator origin. Incoming forwarded
  addresses are honored only from explicit `VELORA_GATEWAY_TRUSTED_PROXIES` IPs.
  The rightmost untrusted address is selected with the existing chain policy;
  malformed trusted chains return 400. Only that resolved IP is forwarded, never
  an arbitrary client prefix. Untrusted peers use their socket IP.
- Every response includes the fixed public `X-Authlib-Injector-API-Location`.
  Application bodies, signatures, Location and cookie fields are not rewritten.
  Hosts must therefore emit public URLs and cookie paths appropriate to the prefix.

`VELORA_GATEWAY_HEADER_TIMEOUT_SECS` defaults to 30 (range 1-3600). Connection
establishment has a five-second limit; the response-header deadline also covers
request upload time. This is not a total response/download timeout. Operators must
size upload and reverse-proxy deadlines for their actual distribution workloads.
Connection failure returns generic 502; expiration returns generic 504. Internal
addresses, request credentials and raw network errors are not included in responses.

## Health and lifecycle

The configured public prefix contains `/health/live` and `/health/ready`.
Liveness reports that the gateway responds. Readiness probes Authentication's
`/health/ready` and, when configured, Panel's `/healthz` on every call, with bounded
probe size/time. Readiness returns 503 if either required upstream is unavailable;
it is not a complete feature-parity or provider readiness certificate.

Ctrl-C and Unix SIGTERM stop accepting connections and drain current requests.
An unfinished stream can prolong drain; process-manager deadlines remain an
operator concern. The gateway owns no database or service writer lock. Original
credential writers still require coordinated quiescence/import before a real
cutover; never route two authorities against an active mixed database.

## Validation and migration limits

Independent tests cover raw wire forwarding, binary/compressed bodies, duplicate
cookies, range/cache headers, exact errors/redirects, scoped headers, path ownership,
trusted proxy chains, outages, header deadlines, incremental response streaming,
graceful drain and public-only mode. CLI errors do not echo URL credentials.

`scripts/check-game-auth-gateway.py` runs the real gateway, Authentication service
and offline tools with entirely new synthetic data. It tests fresh bootstrap,
game login/join, multipart skin upload, RSA signature verification, prefix/origin
and spoofed-IP behavior, authority outages, closed checkpoint/assets restore,
existing token/key/UUID/password/texture continuity and live revocation. The
canonical checker lives in Infra; its reviewed copy allows Panel CI to use a
pinned anonymous public Authentication clone without private repository access.
Linux tests exercise actual executable SIGTERM; Windows process tests exercise
termination/recovery, while Rust tests check graceful runtime drain.

Complete public Panel/Svelte and private backend/page composition, web account
service dependencies, social/report/deletion boundaries, native upgrades, operator
backup/restore/cutover, complete-stack Docker/control-panel profiles and final release acceptance
remain unfinished. No default legacy deployment, credentials, data paths, service
writers, repository visibility or package publication change here.

## Tested container component

Dockerfile.gateway builds the real gateway under a non-root, read-only runtime. Reviewed Infra Compose/checker copies build it alongside pinned public Authentication, without private fetches. Both Linux CI runs at 6713e9e passed fresh synthetic login, skins, signature/public-origin checks, authority outages and offline restoration into a separate owned volume with token/password/key/texture continuity and live revocation (37360361204, 37360132106). See CONTAINER_PROVENANCE.json and the Infra game-auth-gateway recipe. This component profile does not deploy the complete Panel or certify legacy upgrades/full application parity.
