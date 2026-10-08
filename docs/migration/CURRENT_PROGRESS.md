# Velora migration checkpoint — 2026-10-05

The user authorized the complete migration, Velora rebrand, feature preservation,
and preparation of seven open-source repositories, with Experiences kept private.
This supersedes the earlier Stage 1-only implementation scope. The recorded MIT
choice applies to platform code; it grants no rights to private gameplay or artwork.

The migration is **in progress**. This checkpoint is not a replacement deployment,
final public-release approval, or a claim of complete feature parity certification.
The original source repository remains private. The full maintained application
composition is now maintained in private Experiences; public backend, UI and Java
boundaries and live data cutover still require separate acceptance.

## Current priority: merge the migration repositories (2026-10-07)

The operator requested that migration and repository merges take priority over
further deployment work. Owner PRs merged into `main` after exact-head
CI, public-source boundary checks and archive verification.
All eight owner PRs have now merged, and their post-merge CI passed. The compatibility
application PR also merged after fresh full CI.
See [the merge checkpoint](MERGE_CHECKPOINT.md) for reviewed revisions and evidence.

Deployment guide Milestones 3 and 4 have passed: the protected Development control
plane/UI are deployed, and the dedicated Hermes agent is enrolled and reporting
signed heartbeats. Milestone 5 implementation has resumed; service onboarding remains
gated on the complete signed-job execution and rollback chain.
These deployment milestones do not certify replacement application feature parity.
Complete application composition and isolated fresh-install/cold-restore checks have
passed. Data-writer cutover, existing-data upgrades and full release acceptance remain
open; the detailed extraction checkpoints below
record completed subsets rather than completed parent migration gates.

## Complete application baseline and unsigned releases (2026-10-07)

The operator requested complete working applications as the immediate priority,
including the admin/player site, desktop launcher and Android/iOS apps, and then
explicitly requested unsigned artifacts with no developer accounts or signing keys.
The current evidence and outstanding gates are in
[APPLICATION_ACCEPTANCE.md](APPLICATION_ACCEPTANCE.md).

Experiences PR #2 moves the complete maintained compatibility composition into
`application/`, preserving 968 source inputs, owner snapshot checks and private
gameplay. This removes the original repository as a build requirement for the full
site, backend, desktop shell and Minecraft host composition; it does not finish
the separate public application boundaries or perform a live data cutover.

Panel PR #2 merged the independently buildable Android/iOS shell into `mobile/`.
PR #3 implements unsigned release APK, device IPA and simulator builds. The operator
subsequently clarified free native sideloading: merged PR #4 adds persistent Android
self-signing, with operator-managed encrypted Actions secrets, while retaining
unsigned iOS artifacts for local free-account provisioning. The hosted-origin mobile
release passed all jobs, including Android signature verification. Downloaded artifact
identity, endpoint and checksum checks passed; physical-device acceptance remains open.

## Application and execution checkpoint (2026-10-08)

- Experiences PRs #2 and #3 merged after full Windows/Linux/macOS native and application
  CI. Real-listener synthetic installation and cold backup/restore acceptance passed.
  A real Fabric 1.21.1 installation and downloaded Java runtime check also passed;
  interactive game/server acceptance remains pending.
- Infra PR #2 merged signed, bounded Development probe contracts and a typed local
  executor with replay protection, health checks and application rollback. Linux and
  Windows checks passed. Normal agent polling, durable Workflow orchestration,
  local executor installation and deployment-panel timeline are not wired yet.
- Probe release run 37729741128 published one immutable tested container candidate.
  Hermes cannot pull it anonymously yet; registry access must be resolved without
  giving the agent broad GitHub credentials. No application containers were replaced.
- Public Panel/Launcher application ownership boundaries, existing-data upgrade,
  hosted Velora cutover and physical mobile/device acceptance remain open.

Latest additions: a runnable streaming game-authority gateway, independently
buildable non-root Authentication/Gateway containers and a tested two-service
Compose recipe; a typed SDK HTTP client adopted by the compatibility Panel; the
first maintained private Rust gameplay package; and a neutral SDK map viewer
adopted by both compatibility frontends; and an independent Panel instance/file
metadata library. Synthetic component install,
outage and offline restore checks pass. Complete application composition and
whole-stack feature/upgrade/release acceptance remain open.

## Source and repository state

- Frozen source: `57daa92cb6cb4027982d06bfa1c692ab9edb88ff`, including the Docker
  SQLite temporary-directory fix. All 796 source/resource/test/artifact files are
  preserved byte for byte in private Experiences, with hashes and provenance.
- All eight empty destination repositories now have recovered foundation commits
  on `main`. Follow-up work uses `scopedd/velora-migration`; no visibility changed.
- Local checkouts: `.velora-workspace/<repository>`. They are excluded from this
  repository's Git index and Docker build context. Destination code is committed
  to its own repository, rather than embedding eight Git repositories in the source.
- `authentication` was already public. The other seven destinations were private,
  including Experiences. Experiences visibility is checked before archive pushes.
- Public histories start with reviewed foundation code, not monorepo history.
  Mixed code and historical installers/JARs stay in the private archive.

## Completed extraction subtasks

- SDK: complete existing Rust wire declarations plus metadata, path validation,
  HTTP/hash/download and authlib artifact helpers. Private rules/catalogs/presets
  are excluded. Neutral Experience defaults retain the Stage 1 privacy boundary.
- Authentication: original Argon2 password rules and RSA Yggdrasil signing
  implementation are independently packaged. Panel adapters preserve errors and
  signing-key types. Owned schema/import tools are implemented, and a standalone
  game-authentication runtime is now runnable. Complete web/provider/gateway and
  credential-writer cutover remain pending; detailed checkpoints follow below.
  JWT encoding/validation is also independently packaged with legacy claims, HS256,
  30-day expiry and default version=0 retained. Host adapters still check live account
  status/auth versions; signature verification alone never replaces admission.
  Admission/throttling, the authority-local account record and supplied username
  policy are now owner-local. The host keeps a compatible UserRow alias and optional
  SQLite mapping; credentials/auth_version remain excluded from account JSON.
  Existing identity insert/name/id/group SQL is owner-local behind pool-only
  functions; the host retains validation/hash/UUID/time generation and HTTP/conflict
  adapters. This does not migrate data, open another database or change writers.
  Game-token issuance/lookup/client-token handling, invalidation/signout, token caps
  and join-session SQL are also owner-local. The host retains original route errors,
  public origins, profile/signature and IP/server-ID checks; existing token lifetime,
  live account admission and SQL failure behavior remain compatible.
  Skin/cape validation, content-addressed PNG storage and avatar rendering now also
  belong to Authentication behind an optional feature. The source host retains
  supplied storage paths and maps validation/IO errors to the same HTTP responses.
  No player assets were copied. All twelve temporary Rust authority sources match a
  pinned owner commit; CI verifies their hashes without fetching another repository.
  Player key generation/certificate signing/cache SQL also belong to Authentication.
  Legacy Mojang PEM, V1/V2 payloads, 2048-bit keys, 48-hour expiry, 40-hour refresh
  and strict cache boundary remain compatible; the host retains blocking worker
  and protected HTTP response mapping. Stored keys and signing assets stay in place.
  Cape records/lookup/selection, UUID/email-or-name lookups and skin/cape UPDATE SQL
  are now owner-local. The host retains route authorization, canonical wire
  projection, UUID/image/model validation and its original live group lookup/pools.
  Yggdrasil handlers contain no direct SQL; admin/account workflow and service
  ownership still require the remaining extraction/cutover gates.
- Launcher: installation/launch engine extracted with a verified SDK source snapshot;
  no Panel or Experiences source dependency. Tauri/UI extraction remains pending.
- Minecraft-Integrations: neutral region/NBT/palette/PNG/tile production and generic
  uploads now build independently with a pinned SDK transport snapshot. Private
  settings/claim policies are excluded. The source host compiles the reviewed public
  producer and original color table; private composition supplies opaque overlays.
  Full loader adapters, admission and companion/private composition remain pending.
- Source host: Panel no longer imports the launcher engine. Engine compatibility
  exports keep existing callers working while shared utilities have one owner.
- SDK UI / private composition: generic branding/envelope declarations and widget
  rendering are independently packaged in SDK `ui/experience`. Host-supplied local
  registries replace private component imports. Maintained SMP/Frontiers presets,
  capability/navigation policy and the Frontiers component are in Experiences
  `ui/experience`, using a reviewed SDK snapshot. Source-host adapters preserve the
  same composition. This is a bounded SDK-11 extraction, not the full UI/page migration.
- Private Java: 54 maintained implementation files and existing compatibility
  fixtures are now independently packaged in Experiences. Gameplay settings, claims,
  commands, utilities, shop/guild/reward/chat rules, retries and operation receipts
  remain private. Compile-time imports require only the SDK; map production uses
  neutral ports and a reviewed API-versioned local provider. Contracts are compileOnly
  in the private JAR, which contains no duplicate SDK or renderer classes.
  The source host's 73 implementation/test files match the pinned maintained package;
  CI checks drift without fetching the private repository.
- Baseline: 391 static route registrations captured, and all 796 preservation
  hashes compared with the Stage 1 inventory. Runtime response/schema/upgrade
  fixtures are still required before final API/storage cutover.

## Rebrand and compatibility work

New platform display defaults and web/launcher copy use Velora. Saved operator
branding, databases, volumes, account UUIDs, native storage/keyring identities,
permission nodes, companion channels and historical artifact names are retained.
`VELORA_*` configuration takes precedence over non-empty legacy `SCOPENET_*`
values. Both instance headers are accepted; conflicting headers/query scope fail
closed. Existing request clients still use the legacy header for old-server support.
New desktop builds offer host onboarding instead of the original hosted panel;
native mobile builds require the operator's panel URL.

The supplied galaxy emblem and wordmark PNG originals are preserved byte for byte
in source and SDK `branding/originals/`, with hashes and dimensions. Platform icons
are generated from the emblem; default landing/launcher headers use the wordmark.
Operator branding retains precedence. MIT excludes artwork and trademark rights.
Native installer/package identities still require upgrade/signing tests. Old download
endpoints remain in the source host until compatible split releases exist.

## Validation

- Frozen contracts/engine baseline: 36 tests passed; seven existing online install
  tests were ignored as configured. An existing unused-import warning was recorded.
- Independent SDK: 13 tests passed; formatting and warning-free clippy passed.
- Neutral Java server transport: four standalone tests passed. The compatible
  private source adapter passed all 136 common Java regressions. Connection
  validation, bearer headers, endpoint roots, redirect refusal, JSON/byte transport,
  error precedence, dynamic settings and request timeouts are preserved. Canonical
  models and loader/client adapters remain incomplete.
- Legacy Java API declarations are now SDK-owned: nine neutral API/provider/DTO
  files and five optional Bukkit event files. Independent builds, five Java tests
  (four transport plus one provider/consumer compatibility test), compile-only
  consumer compilation and SDK CI passed. The source host uses a verified 14-file
  snapshot; the existing API artifact and Paper plugin build passed after adoption.
- Independent map producer: eighteen renderer/upload/packaging/provider tests and CI passed.
  Generated worlds verify tile/player/overlay uploads, and an actual restart checks
  cached tiles are not replayed. An initially omitted color CSV let the fallback
  palette pass existing tests; the table is now preserved byte for byte, included in
  both producer/Paper JARs, and tested against exact colors that differ from fallback.
  Source common tests (136) and the Paper build passed after adopting the producer.
  Local-provider discovery passes with an isolated server context loader; the
  source Paper JAR includes the versioned provider registration. The private Java
  package passes 122 tests, including useful missing-provider behavior without
  renderer/loader classes installed. Its JAR excludes public contracts and renderer
  implementation; full private loader registration/runtime composition is pending.
- Java presentation strings, client titles/locales and loader display descriptions
  now say Velora. Paper and legacy Fabric declare `/velora` aliases with existing
  permissions; live loader/upgrade acceptance remains pending. Bukkit's legacy
  plugin identity/data directory, Java classes/packages, mod IDs, key IDs, channels,
  namespaces and persisted identifiers remain compatible.
- Independent authentication: twenty-six cryptography/admission/throttling/identity/
  store tests, formatting and warning-free clippy passed with all features enabled.
  Thirty-one focused source-host API/identity/instance/profile/Yggdrasil regressions
  and one protected-route store-outage test passed after adopting owned SQL adapters.
  A cached admin JWT does not grant access when the live pool is unavailable.
  After adopting owner-local token/join storage, 22 focused API/identity/Yggdrasil/
  outage tests passed, including signed profiles, launcher game login and chat
  certificates. Cached game tokens and join sessions cannot admit users during
  authority outages. The new 16-test independent CI run passed.
  After adopting owner-local textures, both host error-mapping tests and all 17
  API/Yggdrasil regressions passed. Independent 20-test CI, formatting and clippy
  passed. Original texture tests plus synthetic legacy-layout, size-limit, IO-error
  and avatar boundary cases preserve validation, SHA256 filenames and rendering.
  After certificate adoption, all six Yggdrasil regressions and both host outage
  tests passed. Independent 23-test CI, formatting and warning-free clippy passed.
  Synthetic keys verify both signature payloads and preserved key-row bytes, strict
  refresh/replacement and outage errors. Final Panel compilation passed with RSA
  removed from production dependencies; its verifier fixtures use a dev dependency.
  After cape/identity adoption, all 12 focused identity/profile/Yggdrasil/outage
  regressions passed, followed by the complete 81-test Panel unit suite and 26
  source Authentication units. The original PEM fixture was found still referencing
  its extracted helper; it moved with the helper before this full unit check.
  Independent 26-test CI, formatting and warning-free clippy passed. Synthetic
  cape/group fixtures preserve fresh membership, visibility, case/order and
  malformed JSON policy; cosmetic updates retain UUIDs and unrelated fields.
  Independent CI for the identity SQL checkpoint passed. Final
  Panel compilation passed after removing unused direct crypto dependencies;
  those implementations now belong to Authentication.
- Compose configuration validation passed; no container deployment or live data
  migration was performed.
- Independent launcher engine: 28 tests and formatting passed; seven existing online
  installation tests remain ignored. No Tauri/UI extraction is included in that result.
  Initial CI could not fetch the still-private SDK; a reviewed MIT source snapshot
  now removes the Git credential requirement, with every source hash checked by CI.
  The snapshot build passed the same 28 tests, formatting and all 17 source hashes.
- Both source-host frontends passed Svelte checks, runes guards and production builds.
  Panel reports existing accessibility/deprecation and bundle-size warnings.
- Generic SDK UI: Svelte check passed with no diagnostics; four standalone rendering,
  navigation/default and safe-link tests passed. Both source-host frontends also
  passed after adopting the injected widget host. Three private preset/navigation
  compatibility tests and the independent private package Svelte check passed.
- The source-host backend, engine, utility and authentication regression suite passed
  with `CARGO_PROFILE_TEST_DEBUG=0 cargo test -j 1 -p velora-platform-utils
  -p velora-auth-core -p scopenet-core -p scopenet-panel --offline`, including the
  new environment/header compatibility tests and existing identity/Yggdrasil flows.
  Shared defaults passed all four tests; native launcher `cargo check` also passed.
- The first full Windows test build exceeded memory through parallel linking; it
  was stopped and restarted with two jobs; that also hit paging-file limits. A one-job
  build with test debug symbols disabled passed. Failed builds do not count
  as passing regression checks.

## Required remaining work

Follow the existing tracker; close a task only when its whole acceptance gate passes.
The immediately relevant incomplete groups are SDK cross-language clients/extension
ports; auth web/provider/presence/audit composition and service cutover; private gameplay backend/UI/Java
packages; public Panel/Tauri/adapter composition; database import/revocation/outage
fixtures; native identities/artwork; host-neutral packaging; published dependency
availability and anonymous builds; public source/artifact audits; full-stack upgrade,
backup/restore and supported loader/platform acceptance. Neither the private archive
nor a repository foundation substitutes for any of these deliverables.

Root migration scripts are source-extraction tools, not application startup commands.
They use the frozen Git revision and never read production data. Rerun extraction
only before reviewing destination edits; do not overwrite subsequent maintained work.

## Review checkpoints

- Source adapters/rebrand: https://github.com/scopeddlol/SCOPENET-MC/pull/41
- Neutral Minecraft map producer: https://github.com/VeloraMCDev/minecraft-integrations/pull/1
- SDK contracts/utilities/assets/widget host: https://github.com/VeloraMCDev/sdk/pull/1
- Authentication cryptography (AUTH-01/02 partial): https://github.com/VeloraMCDev/authentication/pull/1
- Launcher engine (UI/Tauri still pending): https://github.com/VeloraMCDev/launcher/pull/1
- Infra baseline (BASE tasks partial): https://github.com/VeloraMCDev/infra/pull/1
- Docs tracker: https://github.com/VeloraMCDev/docs/pull/1
- Private preservation/composition: https://github.com/VeloraMCDev/experiences/pull/1

All are drafts. SDK (Rust, widget host and Java), Authentication, Launcher, map
producer and private UI/Java CI passed. No parent acceptance gate is closed by these subtasks.

### Authentication HTTP ownership checkpoint

Authentication now maintains an independent Yggdrasil HTTP library with six
reviewed source files. The source host consumes their immutable snapshot and
supplies typed local ports for live operator branding, public origins and trusted
client IPs. The host retains its version, original storage pools/paths, shared
web/game login guard, global middleware and 4 MiB upload limit. Public SDK wire
models are bridged explicitly to the existing launcher/account response types;
protected Yggdrasil endpoints continue to accept game access tokens only.

Independent Authentication CI passed at `12c752e` (runs 37265404371 and
37265401597): 29 tests, formatting, all-feature checks and warning-free clippy.
Synthetic router fixtures cover metadata, login/email lookup, refresh/revocation,
join/IP matching, signed profiles, certificate caching, signout, disabled accounts
and store outages. No production identity records, assets or keys were copied.
All six HTTP, three SDK contract and twelve authority sources match their pinned
owner hashes. Existing public Java/UI/map and private Java drift checks also pass.

This is HTTP library adoption; standalone daemon/bootstrap, owned schema/import,
account/link/reset/admin workflows, credential-writer cutover and backup/restore
acceptance remain incomplete. The source application remains the compatibility
host. No parent acceptance gate is closed by this checkpoint.
Final host adoption validation passed: 25 integration tests (API, identity,
profile, Yggdrasil and outage adapters) and 108 units (81 Panel, 26 authority,
one HTTP helper). New host fixtures verify shared web/game throttling, trusted
proxy join/IP matching, live operator branding and the host version. The initial
new throttle assertion expected the wrong capitalization; it was corrected to
compare the preserved owner constant and the complete suites passed afterward.

### Account mutation transaction checkpoint

Authentication owns the username/auth-version update, game-session revocation
and model-only skin update. Rename mutations require the host's open transaction;
existing name/password validation, online-server checks, reserved-name conflict
mapping, launcher-session cleanup and audit insertion remain in the host. Account
reload/skin clearing also use existing authority functions. No table is moved or
opened, and no cross-domain write is split into a separate transaction.

Independent Authentication passed all 30 tests, formatting and warning-free clippy;
CI passed at `f522a04` (runs 37266921245 and 37266917958). A synthetic audit failure
rolls back the username, auth version and game-session deletion together. Successful
commit preserves UUIDs and unrelated users; uniqueness conflicts retain identity.
This is mutation ownership. Full account/admin workflows and credential-writer,
owned-schema/import and standalone service cutover gates remain incomplete.
Host adoption passed all 15 focused identity/profile/Yggdrasil/outage regressions,
81 Panel units and 27 authority units. The HTTP audit-failure fixture verifies
rollback keeps the old name, UUID and valid web/game sessions; a successful retry
keeps the UUID and revokes both old sessions. The original reserved-name/stat
preservation fixture also passed. All twelve authority sources match `f522a04`.

### Launcher account workflow checkpoint

Authentication owns launcher login, registration, session response and public-user
projection workflows behind explicit operator policy, live username policy and
login audit ports. Source launcher routes supply existing request/identity pools,
shared JWT/guard instances and exhaustive typed SDK wire/registration-mode bridges.
Operation/error ordering, pending responses, open/closed/approval registration,
admin exceptions, random UUIDs, email normalization, last-login updates and audit
behavior remain compatible. Account/link/reset/admin/service cutover is still pending.

Independent Authentication passed 32 tests, formatting and warning-free clippy;
CI passed at `01980fc` (runs 37267744898 and 37267740708). Source adoption passed
26 focused API/identity/profile/Yggdrasil/outage regressions and 109 units (81 Panel,
27 authority, one HTTP helper). All 10 persisted-instance/access/isolation/cleanup
regressions and the configuration-alias fixture also passed. Seven HTTP and twelve
authority sources match their pinned owner; the SDK wire snapshot has no private
build dependency. Snapshot refresh refuses maintained-source drift/deletion.

The Docker web stage was missing the adopted SDK widget sources; it now copies
`packages/platform-ui` along with the shared wrapper. Local Docker daemon access
is unavailable, so local container builds have not been verified. A manual source
CI job now verifies widget hashes and builds the web target in a working container
environment. Container/release/deployment acceptance remains pending until it passes.

### Password recovery workflow checkpoint

Authentication owns forgot/reset workflows through typed email-readiness,
configured-origin and email-delivery ports. The source host retains its provider
configuration/credentials, current pools and SMTP implementation. Generic responses,
active email lookup, token format/SHA256-only storage, five-minute suppression,
30-minute lifetime, failed-delivery cleanup and atomic single-use/password/version/
game-token/join-session updates remain compatible. No real email was sent by tests.

Independent Authentication passed all 34 tests, formatting and warning-free clippy;
CI passed at `d003361` (runs 37268999015 and 37268995029). Source adoption passed
28 focused reset/API/identity/profile/Yggdrasil/outage regressions and 109 units.
New actual-route fixtures verify old sessions survive a failed password mutation,
the same reset can retry, successful reset keeps the UUID and revokes old web/game/
join sessions, and email configuration responses remain unchanged. Eight HTTP and
twelve authority source hashes match the owner; account linking, admin/account
mutation workflows, owned schema/import and standalone service are still pending.

Source CI run 37268422213 at `0dcee9e` passed Docker web composition, both frontends,
Java/Paper, Fabric 26.3 companion and the native Linux launcher. Its full Rust job
failed during linking after the runner reported only 91 MB disk free; this is not
a passing backend CI result. Source Rust/native CI now disables test debug symbols
and incremental output and uses one locked build job. A full rerun is required.
Docker web-stage validation is now confirmed by CI; full container/deployment and
public/private-stack acceptance remain incomplete.
Final production Panel compilation passed after reset adoption, with only the two
existing unused-function warnings. No reset extraction import warning remains.

### Discord workflow and shared allocation checkpoint

Authentication owns Discord authorization state, callback admission/allocation,
linking, polling and unlinking. Typed host ports retain provider credentials,
HTTP exchange, configured origins and live operator policy. Launcher registration,
Discord registration and existing host admin/bootstrap callers share the same
authority account allocation with original password/time/UUID ordering. Existing
HTTP routes, ten-minute expiry, one-use results, cancellation and ownership errors,
verified-email rules and disabled/approval/closed account behavior are preserved.

Independent Authentication passed 36 tests, formatting and warning-free clippy;
CI passed at `df12530` (runs 37270633899 and 37270630495). Source adoption passed
30 focused integrations and 109 units; all 10 persisted-instance regressions and
the configuration-alias fixture passed. Actual-route tests cover link admission,
cancellation/replay, polling expiry/single use and provider-specific unlinking.
Nine HTTP and twelve authority sources match pinned owner hashes. Fixtures use
synthetic provider responses and perform no external Discord or email requests.

The full source CI rerun 37269657865 at `ba0a5f8`, before this Discord adoption,
passed all seven jobs: Rust, native Linux launcher, both frontends, Docker web,
Java/Paper and Fabric 26.3 companion. Rust reported 293 passed and seven existing
online-installation tests ignored. Disabling debug/incremental output resolved
the previous runner disk failure. This verifies that checkpoint, not deployment
or full public/private-stack acceptance. Complete account/admin workflows,
owned schema/import, standalone configuration/service and credential-writer
cutover remain incomplete. No parent acceptance gate is closed here.

### Player account workflow checkpoint

Authentication owns username/profile/skin/model/cape/avatar workflows. Typed
host ports retain the original 90-second live server-presence check and perform
launcher-session cleanup/audit insertion in the authority's rename transaction.
Name/password/guard/error ordering, conflicts, UUIDs, session revocation and
rollback stay compatible. Explicit SDK projection preserves profile responses;
multipart extraction, avatar headers and artifact mirroring stay in the host.

Independent Authentication passed 38 tests, formatting and warning-free clippy;
CI passed at `a0656a0` (runs 37273504726 and 37273500299). Source adoption passed
33 focused integrations and 109 units. Final edited identity/Yggdrasil/board
fixtures passed, along with all 11 persisted-instance/config fixtures. New route
assertions cover live/stale server presence, model retention on skin deletion,
unchanged skin URL/UUID and missing avatars. Ten HTTP sources match the pinned
owner; administrator account/group/cape workflows and service/schema/import
cutover remain incomplete.

Source CI 37272879005 at `af10637` passed six jobs; full Rust failed one existing
contract-board exhaustion assertion. The submission endpoint legitimately refills
the board and can generate another diamond contract after fixture deletion. The
fixture now temporarily reaches its daily limit for that assertion and restores
the original limit before refill checks; production gameplay is unchanged. The
updated board fixture passes locally. Full source CI must verify this newer
checkpoint before claiming its complete CI acceptance.

### Administrator identity/group checkpoint

Authentication owns administrator identity create/update/list, deletion admission
and group create/delete/list. Existing AdminUser guards remain in the source routes;
gameplay/activity aggregation and multi-experience account purge remain host-owned.
Pending-first case-insensitive listing, group defaults/mapping fields, unknown or
duplicate memberships, self-protection, UUIDs and live admission are preserved.
Password/version/game/join revocation remains atomic; other field updates retain
legacy ordering and partial-update semantics rather than silently changing behavior.

Independent Authentication passed 40 tests, formatting and warning-free clippy;
CI passed at `a8bb153` (runs 37274667895 and 37274662929). Source adoption passed
54 integrations and 109 units, including all 10 persisted-instance fixtures.
Two new actual-route fixtures verify administrator admission, group projection,
self-protection, partial updates and password-revocation rollback/retry. Their
initial use of PUT returned 405; they now use the existing PATCH route and pass.
Eleven HTTP and twelve authority sources match pinned owner hashes. Player-account
and launcher-account provenance now use distinct files, retaining both records.
Administrator cosmetics, provider/SMTP configuration, full deletion/service writer
cutover, owned schema/import and standalone deployment remain incomplete.

The full source rerun 37274215552 at `2800f84`, before administrator adoption,
passed all seven jobs including the repaired board fixture, Rust, native Linux
launcher, both frontends, Docker web, Java/Paper and Fabric 26.3 companion.
Rust reported 295 passed and seven existing online-installation tests ignored.
Administrator adoption still requires its full source CI checkpoint.

### Administrator cosmetics and repository presentation checkpoint

Authentication owns administrator skin/model/cape mutations, cape CRUD/projection
and identity counts. Host routes retain AdminUser admission, multipart extraction
and private aggregate views. Strict administrator model validation, missing-user
error distinctions, private-cape assignment, validation/error ordering, sorting,
wearer counts and model/UUID retention remain compatible. Cape deletion retains
legacy wearer-clearing-before-delete behavior, including partial failure.

Independent Authentication passed 42 tests, formatting and warning-free clippy;
CI passed at `3acf551` (runs 37298729691 and 37298725916). Source adoption passed
37 focused integrations and 109 units, including all 10 persisted-instance fixtures.
Actual-route assertions verify private-cape administrator assignment, strict model
validation, cape deletion/unassignment and stable UUIDs. Twelve HTTP and twelve
authority sources match pinned owner hashes. The previous full source run
37275392590 at `dda7ffa` passed all seven jobs before cosmetics adoption. Cosmetic
adoption passed all seven source jobs in run 37299475171 at `d274135`. Provider/SMTP configuration,
owned schema/import, full deletion/credential-writer cutover, standalone service
and public/private deployment/upgrade acceptance remain incomplete.

All eight destination READMEs are now customized with repository-specific features,
segment descriptions, real development commands where available, compatibility and
ownership boundaries, contribution/license guidance and accurate migration status.
Each has a distinct official Velora wordmark banner with editable SVG, PNG output,
source/output hashes and artwork terms. Status/license/stack badges and existing
public workflow badges are included; foundations advertise no nonexistent build.
Local README links and banner hashes passed checks; banner layouts were visually
reviewed. Git attributes preserve SVG bytes across checkouts. These changes are
committed and pushed to destination review branches, and Panel draft PR #1 is now
attached. Experiences privacy was verified before its documentation push. No
repository visibility, release or deployment changed.

### Owned Authentication persistence and offline tooling

Authentication now owns a new version-one schema containing only authority tables.
Fresh initialization preserves account/group/cape IDs, immutable UUIDs, NOCASE
names and permanent reservations, group integration mappings, credential versions,
email opt-out and cascading game/provider/recovery/key records. Existing mixed
databases, unsupported versions, missing/foreign tables, missing named identity
constraints and disabled foreign keys are refused. Released legacy migrations and
the live application database are unchanged. HTTP fixtures now use the actual
owned schema rather than simplified handwritten tables.

An explicit offline checkpoint importer uses fixed authority projections and a
single destination transaction. It preserves every projected value, signing key
record bytes and AUTOINCREMENT high-water marks, verifies values/counts before
commit and rejects unprepared UUIDs, nonempty destinations, invalid sequences,
orphaned rows and missing tables. Private gameplay and mixed triggers are excluded.
Synthetic damaged-checkpoint fixtures verify complete imported-record rollback.

The new `velora-auth-tools` command initializes or imports into new files, refuses
overwrites and SQLite sidecars, opens the checkpoint read-only/query-only, and
reports schema version, table counts and source/destination SHA-256 hashes. Tests
verify source bytes remain unchanged and invoke the actual binary with spaced
paths. Unix outputs use owner-only permissions; Windows requires a protected
parent directory. A failed command may leave its new destination for inspection.
No production checkpoint, signing asset or texture was imported.

Independent Authentication passed 52 tests, formatting and warning-free clippy
at `6818209`; CI passed in runs 37302655206 and 37302650433. Schema CI passed at `ea64fc6` (37301058677,
37301053943) and importer CI passed at `17e9b8d` (37301979411, 37301974186).
The compatibility host passed all 34 authority tests and verifies 14 pinned core
sources at `17e9b8d`, alongside 12 unchanged HTTP sources at `3acf551`. A guarded
core refresh refuses modified, undeclared or deleted host sources before adoption.
The newly adopted unused initialization/import APIs do not change live pools.

Consistent backup creation, external RSA/JWT signing and texture checkpoints,
restore/rollback acceptance, bootstrap, provider/SMTP service configuration,
full multi-experience deletion and verified single-writer cutover remain pending.
This is partial progress on AUTH-05/AUTH-07, not standalone deployment or full
feature-parity/publication acceptance.

### Signing continuity and first-admin library adoption

Existing valid RSA/JWT signing bytes and explicitly configured legacy JWT bytes
are retained. RSA/JWT read failures and invalid persisted material now produce
actionable errors rather than falling through to automatic replacement. New files
use exclusive creation, synchronous writes and Unix owner-only permissions.
Windows still relies on protected parent-directory permissions. No signing identity
or valid existing credential is rotated by this change.

Authentication owns first-admin library composition through the common username
policy/hash/UUID allocation path. Conditional SQL arbitrates competing attempts;
restarts preserve existing administrators, including disabled accounts, and never
promote a conflicting player. The host retains credential selection/delivery and
the existing generated-password startup messaging until standalone composition.

Independent Authentication passed 58 tests, formatting, warning-free clippy and
both CI runs at `836034c` (37304103491, 37304098555). A multi-connection synthetic
store verifies competing bootstrap attempts. Source adoption passed 37 focused
integrations and 120 units, including two new startup fixtures and all ten persisted
instance fixtures. A persisted JWT remains usable after a host restart; admin
ID/UUID/hash and signing-file bytes are unchanged. Damaged JWT material blocks
startup without creating users or rotating the file; explicit legacy config still
works. Fifteen core and twelve HTTP sources match the reviewed owner pin.

The preceding owned-schema/import source checkpoint `bc039ba` passed all seven
jobs in run 37302996158, including both web builds, Docker web, native Linux
launcher, Rust, Java/Paper and Fabric 26.3. Startup adoption passed all seven jobs
in run 37304917835 at `6044495`. Standalone configuration/entrypoint/bootstrap delivery,
external signing/texture checkpoints and restore/rollback, full deletion and
credential-writer cutover remain incomplete.

### Closed checkpoints, signing/texture carry and synthetic restoration

`velora-auth-tools checkpoint` exclusively creates a byte-exact copy of a closed
SQLite database, preserving the original private/mixed schema, data, row identities
and SQLite user version. Source/output hashes and SQLite integrity are checked.
The original checkpoint remains protected private backup material, not public
repository content. Source sidecars and existing destinations are refused.

The new `assets` command validates an imported owned database read-only, verifies
every referenced skin/cape texture, retains all cached hash-named textures,
copies exact RSA/JWT bytes and checks RSA public identity and every file hash.
It writes a protected integrity manifest into an exclusively created directory.
Explicit configured JWT bytes can be supplied from a protected file without
trimming; short legacy configured values still require explicit-secret runtime
configuration. Symlinks, directory escapes, malformed/missing texture hashes,
invalid signing files and observed checkpoint changes are refused. Unix directory
and file modes are 0700/0600; Windows inherits protected-directory permissions.
Copy-time failure may preserve a partial directory for inspection, never deployment.

Checkpoint readers use SQLite immutable read-only mode after canonical sidecar
preflight. A closed WAL-mode fixture retains its source bytes without journal-mode
changes or sidecar replay. The existing-key loader never generates missing material.
An end-to-end synthetic restore verifies original Argon2 credentials, UUIDs, web
JWTs, game tokens, join records, player certificate bytes and RSA signatures.
Fresh live revocation still denies the restored sessions; private gameplay is
excluded from the imported authority store and preserved in the original checkpoint.

Independent Authentication passed 64 tests, formatting and warning-free clippy
at `39b0045`; both CI runs passed (37307097256, 37307090897). The host also passed
all 39 core tests with fifteen sources pinned at that revision; twelve HTTP sources
remain pinned at `836034c`. No production database, key, cosmetic asset or checkpoint
was imported. Operator quiescence/checkpoint acceptance, full restore/rollback and
gateway URL acceptance, standalone config/bootstrap delivery/entrypoint, provider/SMTP
configuration, full multi-experience deletion and single-writer cutover remain pending.

### Standalone configuration library

Authentication now supplies typed operator origin/listener/storage/proxy/policy
configuration, exact configured JWT/bootstrap file loading and detected database/
signing-path collision rejection. External origin is required; credentials/query/
fragment URLs and invalid addresses/policy values produce actionable errors without
secret echoes. Default standalone signup policy is closed. Existing RSA/JWT filenames
are retained; database/asset paths can select the imported outputs explicitly.
Loaded bootstrap credentials have no Debug/Serialize, and password-file access is
skipped when any administrator exists. Missing configured JWT files never fall back
to generation. The legacy host's environment adapter and live paths are unchanged.

Independent Authentication passed 69 tests, formatting, warning-free clippy and
both CI runs at `4db0692` (37309803249, 37309795396). Five new configuration fixtures
use injected lookups/temporary files rather than process-global environment changes.
The host snapshot passed all six HTTP/config units; thirteen HTTP sources match
that owner pin, with fifteen core sources retained at `39b0045`. Only existing
anyhow/url package references were added to the lockfiles; no versions changed.

The preceding source checkpoint `c0d2785` passed all seven jobs in run 37307578887.
Configuration adoption at `ae67e99` passed all seven source jobs in run 37310863743. The
separate process is not yet runnable: launcher-session/audit transaction ownership,
provider/SMTP/online-presence ports, setup/readiness/shutdown, full deletion and
single-writer/gateway/rollback acceptance must be completed before service cutover.

### Launcher admission compatibility boundary

Authentication owns session insertion/global retention, per-account and legacy
anonymous-IP verification, mapped-IP comparison and open-transaction revocation.
Panel still owns event observations. The source adapter preserves insertion,
cutoff calculation, pruning and telemetry order, including partial commits after
later failures. Rename cleanup stays inside the existing identity/audit transaction.
No mixed server/gameplay implementation or production records were copied.

Independent Authentication passed 74 tests, formatting, warning-free clippy and
both CI runs at `30caae9` (37313088023, 37313081003). Sixteen core sources are pinned
there; thirteen HTTP sources remain at `4db0692`. Source adoption passed 81 Panel
units, 44 core units and all five identity integrations. A new real-router fixture
verifies bearer-only identity, IP recording, event truncation/source, rejected
event kinds, retained launches after telemetry failure, global retention and
failed-audit rename rollback followed by successful revocation.

The owned session schema/import conversion and durable audit/service interface
remain pending at this checkpoint. The separate process is not runnable and no
writer/store/deployment cutover was performed. Full source CI for this adoption
passed all seven jobs at `268e9f4` in run 37313872649.

### Additive owned launcher-session schema and import

Authentication keeps version-one SQL unchanged and adds version two for launcher
admission rows/indices. Initialization validates intact old stores before upgrading
in one transaction, preserving identity records, ID sequences and SQLite user version.
Read-only validation accepts supported versions without creating or upgrading tables;
asset-copy manifests report the actual source database version.

The importer now copies exact session IDs, nullable account/name fields, raw IP
bytes and timestamps, including anonymous history, and preserves the deleted-ID
high-water mark. Missing session tables, invalid sequences and orphaned sessions
abort the whole data import. Earlier version-one imports must be re-imported from
the original closed legacy checkpoint before cutover: an empty-table upgrade cannot
reconstruct the omitted session history. Mixed Panel events remain excluded.

Independent Authentication passed 77 tests, formatting and warning-free clippy at
`0a9f0fb`; both CI runs passed (37314936986, 37314928213). The source host passed all 47 core tests with sixteen sources pinned at
that revision; thirteen HTTP sources remain pinned at `4db0692`. Session tests now
use the actual owned schema. No production database or deployment was touched.
The additive schema snapshot at `e14981a` passed all seven source jobs in run 37315322254.

### Trusted-proxy resolver ownership

Authentication now owns the pure socket-peer/forwarded-IP resolver. The source host
still supplies its existing proxy policy, branding and operator public URL settings.
Missing/untrusted peers cannot authorize forwarded headers; right-to-left chain
selection, malformed-hop failure and real-IP/peer fallbacks remain compatible.
Synthetic fixtures cover spoofed prefixes, invalid/trailing hops and IPv6 formatting.

Independent Authentication passed 80 tests, formatting and warning-free clippy at
`b6e92bb`; both CI runs passed (37315959487, 37315953947). Thirteen HTTP sources match that pin; sixteen core sources remain at
`0a9f0fb`. Source adoption passed 90 units (81 Panel and nine HTTP) plus all nineteen
identity/server/Yggdrasil integrations, including the existing forged-IP and trusted
join-address fixtures. Full source CI at `de48928` passed all seven jobs in run 37316545961.
Experiences visibility was verified PRIVATE. No deployment or visibility change occurred.

### Public Panel activity library

Panel now independently builds neutral metadata-write and activity-report projection
operations. Caller-supplied pools/identity/time and live administrator guards retain
the compatibility boundary. Credential middleware, private gameplay and mixed
migrations were not imported. Wire fields/nulls, unknown historical payloads, IDs,
server-name joins, bound filters, Unicode detail sanitation, sorting and pagination
remain compatible. Complete Panel service/Svelte and separate-store projections
are still incomplete.

Three independent tests, formatting, strict clippy and both Panel CI runs passed
at `fca92d9` (37317824265, 37317820349). Source adoption passed 84 units and twelve
identity/server/progression integrations, including guarded activity identity,
audit failures/rename rollback and progression reports. The source copies library,
adapted manifest and MIT license with three verified hashes; raw snapshot bytes
are preserved by Git attributes. The lockfile adds only the new local package
and host dependency, with no existing package versions changed. Root CI now
verifies and tests the public activity snapshot. Full source CI at `e568cfc` passed
all seven jobs in run 37318507105.

Panel's customized README now has its real workflow badge, library information,
tested commands and accurate limitations. Its original banner hashes are unchanged.
The README generator supports selecting repositories so unrelated landing pages
need not be regenerated for a component update. No publication/visibility change
or production data import occurred.

The implemented D3/D8/D9 boundary is recorded in [ADR 0001](https://github.com/VeloraMCDev/docs/blob/scopedd/velora-migration/migration/adr/0001-launcher-admission-and-platform-observations.md). Separate-service audit/presence/coordination decisions remain deferred.


## Generic communications extraction — AUTH-04 / D7

Public Panel now owns generic connection settings, SMTP message/delivery and
Discord OAuth token/profile transport in velora-panel-communications. The protected
JSON schema, empty/clear/replace secret policy, validation precedence, masked flags,
plain-text/alternative-HTML MIME and existing Resend relay remain compatible.
Discord retains form encoding, request ordering, bearer profile lookup, unknown
fields and 400/502 error distinctions, using the caller's existing HTTP client.

The extraction excludes mixed route guards, protected persistence, account
allocation/reset/link state, bulk audiences/templates and private gameplay.
Authentication retains identity/reset/link lifecycle; the host retains live settings,
callback origin, authorization, recipient scope and existing retry/queue policy.
A single SMTP acceptance attempt remains distinct from final inbox delivery.

Eleven independent Panel tests (three activity and eight communications), formatting
and strict clippy passed locally. Synthetic MIME and loopback HTTP tests use no
external provider accounts or delivery. Existing owner dependency versions were
preserved. Frozen-source and output hashes are recorded in Panel's communications
provenance. Owner code at bbcab9a passed both independent CI runs (37323333883 and
37323324608). Host adoption passed 89 unit tests and eleven settings/mail/
OAuth/password-reset/identity integrations. Full source CI at 823383f passed
all seven jobs in run 37324044602: Rust, native Linux launcher, both web builds,
Docker web composition, Java/Paper and Fabric 26.3. The README-only owner update
b5a78ad also passed both CI runs (37323959233, 37323951895).
The host snapshot pins that revision with six raw file hashes and Git attributes;
its lockfile adds only the new local package and host dependency.

AUTH-04 remains incomplete: standalone provider/secret configuration, durable retry,
separate-service composition, complete bulk/admin separation and deployment/upgrade
acceptance are still pending. Panel's customized README includes both libraries,
boundary links and its real CI badge; original banner assets are unchanged.

The implemented boundary is recorded in [ADR 0002](https://github.com/VeloraMCDev/docs/blob/scopedd/velora-migration/migration/adr/0002-provider-transport-and-account-lifecycle.md).


## Generic mail layout and template validation — AUTH-04 / PANEL-07

Public Panel's communications library now owns the generic branded HTML renderer,
the persisted EmailTemplate shape, Unicode size validation and bulk replacement
validation. Four synthetic HTML outputs generated by compiling only the frozen
neutral renderer assert byte compatibility. Escaping, link/button policy, headings,
lists, accent fallback, bold and literal single-star text retain legacy behavior.
Callers supply resolved text and a trusted HTML footer; the renderer does not
sanitize arbitrary caller HTML. The extraction excludes private rank/guild/level
queries, starter catalogs, recipient selection, unsubscribe secrets and queue SQL.

Template validation preserves four-field/default JSON, unknown placeholders,
character counts, ID trimming/case, duplicates, count limits and error order.
Bulk replacement intentionally retains its different legacy ID-length rule;
individual create endpoints still enforce their own 50-character ID limit and
conflict-before-size ordering. Storage and live administrator guards stay in the
host. Owner provenance records both frozen source files and output/fixture hashes.

Sixteen independent public Panel tests, formatting and strict clippy passed locally.
Both independent owner CI runs passed at bcd6aee (37326311577 and 37326296700).
Host adoption passed 92 units and nine integrations: two frozen renderer fixtures
and seven email/settings/OAuth/password-reset cases. Full source CI at 41edf56
passed all seven jobs in run 37327336741: Rust, native Linux launcher, both web
builds, Docker web composition, Java/Paper and Fabric 26.3. The
compatibility snapshot now includes fourteen verified source/manifest/license/test
and fixture files with raw-byte attributes. No dependency versions changed.
The snapshot refresh checks maintained files before updating and rejects undeclared
source/fixtures. A maintainer script reproduces the synthetic frozen renderer
fixtures without importing the mixed private route file into public code.

The Panel README describes rendering/validation and excluded policy. ADR 0002 now
includes the boundary. Complete bulk/admin separation, retryable service delivery,
standalone applications and final migration/release acceptance remain incomplete.


## Standalone game-authentication runtime — AUTH-08

Authentication now builds a real velora-auth-server executable using public SDK,
owned Core/HTTP libraries and no Panel/Experiences dependency. It serves existing
Yggdrasil and texture routes with the 4 MiB body limit, socket-peer trusted proxy
policy, operator public origin/name and API-location header. Host-header spoofing
cannot change signed public URLs. Web/admin/reset/Discord routes are deliberately
unmounted until audit/presence/provider and gateway composition are implemented.

Owned-store startup validates existing schemas read-only, initializes fresh owned
storage and refuses mixed/damaged databases. Canonical OS-held writer locking
excludes duplicate service instances and ordinary symlink aliases. Protected
bootstrap-file loading and conditional first-admin insertion preserve existing
administrators, UUIDs and password hashes, including missing bootstrap files on
restart. Signing loaders preserve original RSA/JWT bytes and refuse silent rotation.
New Unix storage permissions are 0600/0700; existing operator permissions remain
unchanged. No production data, credentials or assets were imported or deployed.

Live readiness validates the current owned schema and fails on store outages;
liveness stays separate. Graceful shutdown drains requests, closes SQLite and
releases the writer lock. CLI help/configuration errors are usable without storage
side effects or credential echoes. SERVICE.md and the customized README document
actual build/run/configuration commands and the incomplete full-service boundary.
Runtime composition provenance records the owned library baseline and file hashes.

The complete independent workspace passed 84 Windows tests, formatting and strict
clippy. Four new tests exercise real TCP login/join, profile signatures, restart
UUID/password/key/game-token continuity, duplicate writers, mixed-store/damaged-key
refusal, outages, CLI errors and runtime shutdown. Both independent Linux CI runs
passed 85 tests, formatting and strict clippy at 12c98f1 (37347108416 and 37347100703),
including the actual-executable SIGTERM and symlink-lock case. Existing dependency
versions are preserved. Both CI runs at 3c7f70b also passed the release-profile
binary build and CLI smoke check (37348143891 and 37348134496).

The compatibility host has not switched writers or routes. The new lock does not
coordinate a legacy process. Production quiescence/import, full web/admin/provider/
presence/audit/gateway composition, deployment profiles and upgrade/restore/rollback
acceptance remain incomplete. AUTH-08 and parent release gates remain open.

A pinned native game-auth development recipe now lives in Infra. It builds only the
public Authentication source and included SDK snapshot, documents fresh protected
storage, bootstrap/health/shutdown, and explicitly excludes complete deployment.
An anonymous clone at 12c98f1 with credential helpers and authorization headers
disabled succeeded; all local package manifests resolve inside that checkout.
A fresh-target, network-disabled `cargo build -p velora-auth-service --locked
--offline -j1` and the resulting executable's `--help` passed on Windows. Registry
dependencies came from the existing local Cargo cache; no private or sibling
repository supplied sources. This verifies source independence, not an empty-cache
or complete-stack installation.

The implemented boundary is recorded in [ADR 0003](https://github.com/VeloraMCDev/docs/blob/scopedd/velora-migration/migration/adr/0003-game-authority-runtime-before-web-cutover.md).

## Streaming gateway and container acceptance checkpoint

Public Panel now builds velora-gateway independently, without private gameplay or database ownership. Fixed all-method Yggdrasil/texture routes always use Authentication; authority errors/outages never fall back to Panel. Raw streaming methods, queries, bodies, status/errors, cookies, cache/range/encoding headers and operator public prefixes are preserved. Supplied public origin and exact trusted socket peers prevent forwarded-origin/IP spoofing. Health/readiness and graceful stream drain are tested; HTTP upgrades remain explicitly unsupported. Full Panel/web/provider composition is still pending. ADR 0004 records this boundary.

Panel runtime 74dadad passed all 22 independent tests, formatting, strict clippy, release builds and native real-process acceptance in Linux CI 37353884452 and 37353878810. Infra native Windows evidence records actual binary hashes, synthetic login/join/skin/RSA signatures, prefixed origins, outage refusal, offline checkpoint/assets restoration with UUID/password/key/token/texture continuity and live revocation. Linux exercises executable SIGTERM; Windows termination/recovery is separately described.

Authentication container packaging at 6c19518 passed both core/container CI runs 37356615490 and 37356607765. Panel container composition at 6713e9e passed both runs 37360361204 and 37360132106. Builds anonymously fetch only pinned public Authentication; runtime containers use UID/GID 65532, read-only roots and dropped capabilities. Authentication has no published port; Gateway uses a loopback publication. Distinct authority-network addresses avoid address collisions. The offline tools profile reads the stopped source volume at /source and restores into a separate owned /data volume, preserving source files and refusing nested input/output. Synthetic Compose tests prove fresh login, uploaded skin bytes, RSA signatures/public URLs, fail-closed authority outage, restored game-token/password/key/texture continuity and revocation. No registry images or production resources were deployed.

Canonical recipes/checkers live in Infra; Panel uses reviewed copies with hashes in CONTAINER_PROVENANCE.json. The recipe is game-component development packaging. Legacy imports/upgrades, complete operational rollback/backup, full Panel/private hosting, loader/launcher end-to-end behavior and public release gates remain incomplete.

## Typed SDK HTTP client adoption checkpoint

SDK 02adc44 supplies a dependency-free browser/Node HTTP runtime and TypeScript declarations with live token/scope/unauthorized ports, legacy and Velora instance headers, body/error compatibility, supplied origin/prefix, deadlines/cancellation and actual launcher-manifest API-version validation. Seventeen tests include eleven comparisons against the frozen neutral original transport. Both four-job owner CI runs passed (37355106323 and 37355099110), type checking and offline npm installation passed, and npm pack --dry-run contains only the five intended public files. The package remains unpublished/private:true pending release readiness. Complete generated endpoint clients and extension ports remain open.

The source Panel at 187a428 adopts a reviewed raw-hash SDK snapshot, keeping its live session/logout/navigation behavior and original unlimited request deadline. Upload helpers and route ownership are unchanged. Svelte checks/builds passed; full source CI 37355832765 passed all seven jobs, including Rust, both web builds, Docker web composition, native launcher, Java/Paper and Fabric 26.3.

## Private Rust gameplay ownership checkpoint

Experiences now maintains the pure gameplay settings/validation/algorithms package and all fifteen original Rust regressions. Frozen source comparison permits Rustfmt changes only. Owner d92ddbb preserves the original formatting configuration and passed both independent three-job CI runs (37361135497 and 37361129761): Rust tests/formatting/strict clippy, maintained Java and private UI checks. Gameplay carries no MIT/public redistribution grant; Experiences remains private.

The compatibility host uses a reviewed private snapshot and facade. Authenticated routes, storage, transactions and game ledger writes remain in the existing host. Public Panel and other public owners never depend on this package. Complete private backend/page composition and service extension interfaces remain unfinished. Source ea88c65 passes 66 host unit tests, all 15 moved private units, eleven casino route and five economy-admin integrations on Windows. Full seven-job CI 37361775009 passed at ea88c65: Rust, both frontends, Docker web composition, native launcher, Java/Paper and Fabric 26.3.

## Public source and build-artifact preflight checkpoint

Infra now supplies a repeatable redacted preflight for the seven prepared public checkouts. It checks tracked dependency/workflow directions and local path closure, snapshot digests, operator-material/artifact paths and high-confidence credential patterns across all reachable local Git blobs. Experiences is refused as an input; candidate credential contents and credential-bearing remote URLs are never emitted. Five synthetic tests prove historical detection after deletion, snapshot drift, private dependencies/outside paths, redaction/refusal and exact artifact provenance guards.

The scan passed for 395 tracked files and 675 reachable blobs across the seven pinned checkouts; evidence/public-source-preflight.json records each revision and scope. The two original Gradle wrapper JARs match the official Gradle 8.10.2 SHA-256 (shared across 8.10 through 8.12.1), and the original embedded Apache-2.0 license is copied verbatim alongside exact artifact provenance in SDK and Minecraft Integrations. Their separately downloaded 9.6.0 distribution now has the official SHA-256 pinned. No wrapper executable bytes or gameplay behavior changed. Official checksum reference: https://gradle.org/release-checksums/.

This preflight does not close publication/release gates: unfetched refs, remote releases/LFS, full third-party obligations, generated/minified bundles/source maps, all credential forms, whole-product anonymous builds and full feature/upgrade acceptance remain unverified. Infra CI tests the checker and its own reachable source with read-only permissions; SDK Gradle 9.6 Java build/tests passed locally on Windows; Minecraft Integrations checksum/Java CI passed at dacfebc (37363836144). New SDK and Infra Linux CI runs are queued, so no success is claimed for those runs.

## Neutral map viewer ownership checkpoint (SDK-11)

SDK 737f6c7 independently packages the five original shared/map sources: API v1
DTOs/dimension aliases, canvas renderer, live feed, exports and Svelte viewer.
Executable source matches frozen 57daa92 byte for byte; only the viewer comment
uses Velora. All endpoints, tokens, scope, overlays and player/toolbar snippets are
supplied by the host. Claim authorization, guild policy and overlay production stay
private. The public package has no application-state, private repository or preset
dependency, and remains private:true/unpublished pending release readiness.

Offline npm installation, Svelte checks with zero errors/warnings and all nine
independent tests pass. Synthetic browser ports exercise the actual controller's
bounds/dimensions, selection priority/claim holes, token renewal/cache reuse,
following, input callbacks and listener teardown. Polling tests cover visibility,
renewal, outages/retry and stopped info responses; Svelte SSR tests cover empty,
error/layer states and escaped supplied labels. npm pack --dry-run contains exactly
nine reviewed source/documentation/manifest/license files. The bounded public-source
preflight passes at this SDK revision (116 tracked files, 142 reachable Git blobs).

Both source frontends adopt six raw-hash-pinned SDK files through the existing
shared/map imports. The Svelte adapter forwards the original props/snippets; URLs,
live transport, instance context and private parent behavior stay unchanged.
Launcher and Panel Svelte/runes checks and production builds pass locally; Panel
retains its existing unrelated component warnings. CI now checks map snapshot drift
in both frontend and container composition jobs, and SDK CI checks both UI packages.
New exact-head Linux runs are pending during the hosted-runner incident; no success
is claimed for them. This completes the bounded map presentation extraction, not
all SDK-11 primitives, full map service separation or browser/whole-product parity.

## Instance metadata ownership checkpoint (PANEL-02 subset)

Panel 6c04f6f independently packages the original instance/file records and metadata
queries in velora-panel-instances. Callers supply the platform pool, timestamp and
reserved-storage directory. SQL order, aggregate casts, nullable/unknown fields,
missing records and retired-name collisions retain existing semantics. The library
opens no database and imports no identity store, private gameplay or sibling source.

Six synthetic tests pass, including restart persistence, unavailable/incompatible
pools, scope isolation, exact file fields and slug reservation. Formatting and
strict all-target Clippy pass. The bounded public-source preflight passes at this
owner revision: 53 tracked files and 79 reachable Git blobs, no findings.

The compatibility host adopts a three-file raw-hash-pinned snapshot. Private
Experience defaults/projections, enabled/group visibility, HTTP errors, clocks and
data_dir/experiences stay host-owned. CI verifies snapshot hashes and includes the
new crate. Host regressions pass: 66 units, 11 API tests and 10 instance integration
tests, including imports, visibility, legacy adoption, storage reservations, live
private admission, credential isolation and immutable resource-pack URLs. Complete
installation/import/search/settings/publishing, UI and standalone control-plane
composition remain open; PANEL-02 is not complete.

The seven prepared public checkout preflights pass after this extraction: 416
tracked files and 707 reachable local Git blobs. Panel exact-head Linux run
37386099825 passed all jobs: workspace formatting/tests/strict Clippy, gateway
release build and real process/container fresh-install, outage and offline-restore
acceptance. Full source CI is running at code commit bc15d2b. These are bounded component
checks and do not close full product or public release gates.

## Instance registry mutation ownership checkpoint (PANEL-02 subset)

Panel 9abb4b0 owns the original create/update/delete, clean epoch, icon,
upload/file-delete and transactional pack-replacement SQL. Host adapters retain
input validation, original create/update media-string differences, live authorization,
private retirement, files/provider operations and post-commit garbage collection.
Pack replacement evaluates the supplied clock at the original late update boundary;
upload DELETE/upsert retain their original separate-statement behavior.

All eleven independent metadata/mutation tests, formatting and strict Clippy pass.
Synthetic triggers prove pack and instance deletion rollback, and tests retain
manual files, opaque metadata, scope isolation and database outage failures. Host
adoption passes 66 units, eleven API and ten instance integrations. Four reviewed
owner files are hash-pinned. The public source/history preflight passes with 56
tracked files and 85 reachable blobs. Complete pack pipeline extraction is now in
progress; public control-plane/UI composition and full PANEL-02 acceptance remain
open. Source implementation CI at bc15d2b also completed successfully after the
preceding metadata checkpoint.

## Complete pack pipeline library ownership checkpoint (PANEL-02 subset)

Panel c0162c5 independently packages original Modrinth mrpack, CurseForge/API and
plain zip import, provider download/checksum, loader normalization, overrides,
file URLs, transactional pack metadata application and post-commit GC. Explicit
PackHost ports supply HTTP client, exact read/write pools, distribution paths, lazy
key lookup, trusted provider origins and clock; private implementation is excluded.
Eleven raw-hashed public SDK files remove sibling/private build dependencies.

All four original pack units and eight new real-archive/local-provider tests pass,
with strict Clippy, workspace formatting and SDK snapshot verification. The source
host adopts five pinned pack files plus seven pinned SDK utility files; explicit
loader bridges retain the original host types. The SDK utilities use a standalone
manifest to coexist with legacy utilities; CI selects the legacy version explicitly.
All twelve pinned-copy tests and 62 remaining host units, eleven API and ten
instance integrations pass. Four original units moved into the owned library.

The owner source/history preflight passes at this revision: 75 tracked files,
108 reachable blobs and one verified public SDK snapshot. Both full Linux CI
runs passed at c0162c5 (37390091540 and 37390084908), including native and
container installation/outage/offline restore acceptance.
Full control-plane routes/UI composition, independent platform storage ownership,
public packages and whole-product install/upgrade/release gates remain incomplete.
Settings/KV library extraction is under development.

## Settings/KV ownership checkpoint (PANEL-02 subset)

Panel 7031145 owns neutral settings fields/default rules, key replacement and input
validation, branding-name validation and original supplied-pool KV queries. The
auth model is generic; the compatibility alias retains its original auth type and
defaults. Host code retains live guards, secret masking/flags, environment priority,
exact read/write pool selection, derived auth URL clearing and private branding.

Five independent tests and strict Clippy pass, including custom auth defaults,
malformed stored JSON, two-pool isolation, key keep/clear/replace, validation/error
ordering, unavailable stores and failed serialization without overwrites. The four
pinned host files pass the same five tests. Host regressions pass 62 units and
27 integrations (11 API, 10 instances, two each email/OAuth/password-reset), with
credential masking, private settings and reset/revocation behavior retained.
The owner public preflight passes: 80 files, 116 reachable blobs and one verified
SDK snapshot. Linux CI and complete settings/server/UI composition remain open.
Neutral landing/download model and helper extraction is under development; private
landing defaults and all real artifact filenames/URLs remain host-owned.

## Distribution model/helper ownership checkpoint (PANEL-02 subset)

Panel fd87aea owns neutral hosted-download, FAQ, block and theme records plus
repository, installer, version and display-filename helpers. Private landing
defaults and provider/cache/upload/download routes remain in the compatibility
host. Empty/non-ASCII branding now displays Velora; explicit operator branding,
stored artifact names, URLs and installer/signing identities remain unchanged.

Five independent tests, formatting and strict Clippy pass. Three pinned host files
pass those five tests; 60 remaining host units, eleven API and ten instance
integrations pass, including landing/download permissions and original stored
filenames. Two original helper tests moved into the owned library, with the
empty-brand expected display name deliberately updated. Public preflight passes
at fd87aea: 84 tracked files, 123 reachable blobs and one verified SDK snapshot.
Full landing/page/service ownership, fresh product installation and upgrade gates
remain incomplete. Both Linux owner runs at settings revision 7031145 passed
(37391375039 and 37391368765); latest distribution Linux checks are in progress.
SDK endpoint-client work is next; this checkpoint does not close SDK-02.

## Typed platform endpoint client checkpoint (SDK-02 subset)

SDK 84a14ef supplies fifteen typed API v1 launcher/auth/account endpoints with
neutral models and opaque Experience module values. Live session/scope/lifecycle,
timeouts and cancellation use the existing dependency-free transport. The route
review corrected the earlier manifest helper to /api/v1/launcher/manifest.
Eleven raw-hashed SDK files are adopted by the host; source CI independently
compares the fifteen reviewed endpoint/method pairs with its actual router.

All 23 SDK and pinned-copy tests and TypeScript declarations pass, including real
loopback request methods/bodies/live headers, multipart bytes, ID encoding,
null updates, incompatible versions, 401 errors and external cancellation. Package
dry-run contains the intended seven files and no bundled dependencies. Host login,
registration, password recovery/reset and player profile/skin/cape/name operations
now use these client methods; original session callbacks, notices, scope and unlimited
deadline remain. Host frontend typecheck (zero errors, 16 existing warnings) and
production build pass. Both complete SDK CI runs pass at 84a14ef (37393520275,
37393513505). Seven public preflights pass across 462 files and 787 reachable blobs.

Full source CI passes at aa662d7 (37392905469), including all four newly adopted
Panel libraries. Both Panel distribution runs pass at fd87aea (37392326551,
37392321388), including native/container install/outage/offline restore. Source CI
for the subsequent typed-client frontend adoption has not yet run. Neutral login
UI extraction is in development. Generated/cross-language endpoint coverage,
OAuth/admin/gameplay/binary clients, complete service/UI ownership and public
release/full product gates remain open; SDK-02 and PANEL-02 remain incomplete.

## Neutral login and skin UI ownership checkpoint (PANEL-02 / SDK-02 subsets)

Panel 578e8f2 independently packages the original login, registration/approval,
recovery/reset and Discord sign-in page. SDK client and token callback are explicit
props; original strings, markup/CSS, field constraints, reset hash navigation,
popup identity and bounded polling remain. Seven actual-script/compiled-render
tests and zero-error/zero-warning typecheck pass. Five raw-hashed public SDK inputs
remove private/sibling build dependencies. The host uses a two-file pinned Panel
copy, adapting only the existing SDK type import and supplying its live client/session
callback and shared styles. This is a component, not the complete public Panel UI.

SDK fe16aac independently packages the original flat Minecraft skin/cape viewer.
The only original host difference is retained through placeholder opacity (Panel
0.07, Launcher 0.08). Seven tests preserve modern/legacy/slim/front/back geometry,
overlays, mirrored limbs, HD/back-only capes, failed/stale image cancellation and
compiled canvas markup. Typecheck has zero errors/warnings. Both SDK full Linux
runs pass (37394821916 and 37394815604). Both host frontends now consume the same
two-file raw-hashed SDK copy. Panel check/build and Launcher check/build pass;
Panel retains its 16 pre-existing unrelated warnings and Launcher has none.
Docker web composition now copies both new public component snapshots explicitly.

Public preflights pass at these owner heads: SDK 130 files/162 reachable blobs,
Panel 99 files/140 reachable blobs, with both Panel SDK snapshots verified.
Panel's latest full Linux checks and source CI for this UI adoption remain pending.
Complete admin/player/landing shells, private page composition, account-service
ports/cutover, real browser/native upgrades and whole-product release gates remain
open. The compatibility application remains the functional product host.

## Explicit account HTTP composition checkpoint (AUTH-08 subset)

Authentication 0c98dc7 mounts twelve existing API v1 method/path pairs for
login/register/me, recovery/reset, profile/name/skin/model/cape and avatars. It
delegates to owned workflows and retains body limits, original multipart handling,
texture URLs/bytes, avatar headers and live JWT status/version precedence. Service
library router/serve methods compose accounts with persistent keys, owned storage,
writer exclusion, readiness and drain through mandatory AccountHost, MutationHost
and ResetHost ports. There are no default/no-op policy/audit/presence/email adapters.

All 32 local HTTP/service regressions pass (25 original HTTP, four existing runtime,
three new real-listener account scenarios), plus formatting and strict all-target
Clippy. New tests cover live registration/approval, audit failure before issuing
tokens, online rename denial, transaction rollback, reset/replay/revocation, JWT/UUID
restart continuity, actual multipart PNG byte retrieval, model/cape/profile/avatar
behavior and unavailable identity/readiness. Public preflight passes at 0c98dc7:
97 tracked files, 239 reachable blobs. Independent Linux/container CI is running.

The CLI/container still mounts game routes only. Real platform policy/audit/presence/
transaction/email adapters, admin/Discord routing, compatible gateway web ownership,
credential-writer/data cutover and whole-stack acceptance remain incomplete. This
implemented composition API does not substitute for those gates. Experiences remains
private (reverified); private economy operation storage extraction is in progress.

Full source CI at a3a7e51 passed (37395149966), including typed-client/login/skin
adoption and Docker web composition. Both Panel runs at 578e8f2 passed (37394833412,
37394827372), including independent login UI and native/container component acceptance.


## Private economy operation/ledger ownership checkpoint (EXP-04 / EXP-08 subsets)

Owner: Codex, 2026-10-07. Status: verified bounded extraction; parent gates open.
Experiences bc74336fa03f67d1dba120c27c9d5e15b61deece owns original operation
reservation/write locking, response replay/commit, insert-only balance and debit/
credit ledger SQL in rust/storage. Ten storage tests preserve concurrent SQLite
writers, replay values/opaque fields, server/two-instance isolation, outages,
insert conflicts, escrow labels/scopes, clock order and failed-write rollback.
The independent private Rust workspace passes all 25 tests, formatting and strict
all-target Clippy. Both complete Linux owner runs passed (37653593305,
37653587209), including Rust, Java and private UI.

Compatibility source c6744ae8e84a0f2fc294b479e30aa38e5bde17f9 adopts five
raw-hashed files. Host facades keep admission, starting-balance policy reads,
original clock boundaries and HTTP response/error mapping. The same scoped pool
and transaction remain in use; no distributed financial transaction, schema/data
migration, persisted identity rename or public gameplay dependency is introduced.
Original ordered SQL and provenance input hashes were independently compared.

All seven source CI jobs passed at c6744ae (37654101575), including 59 remaining
host units, all host integration suites, ten adopted storage tests, both frontends,
Docker web composition, Java, Fabric and Launcher. The original normalization
regression moved into the owned library. Seven tests remain
intentionally ignored by the existing suite; they are not new extraction failures.
The slower duplicate Windows host build was stopped after complete Linux source
CI passed; it is not reported as a completed local regression run.

All 796 archived blobs/modes still match the frozen source. Seven public HEAD/
history preflights pass across 496 tracked files and 835 reachable blobs; this
checkpoint introduces no public source dependency on private Experiences. Infra
records source-to-destination ownership and provenance, leaving partial-file final
paths and parent task acceptance open. Full gameplay routes/migrations/service,
public admin/player/landing shells, optional private composition and whole-product
fresh/upgrade/restore/release acceptance remain incomplete. No production deployment,
repository visibility change or package publication occurred.

## Deployment strategy superseding the previous release plan ? 2026-10-07

The operator supplied [the final deployment implementation guide](../deployment/DEPLOYMENT_GUIDE.md).
It is the ordered contract for ongoing work: audit the eight existing owners, harden
CI/artifact identities, prepare Development Cloudflare bindings, then prove one
harmless deployment and rollback through the control plane and Hermes agent before
onboarding remaining services or completing the dashboard. Infrastructure control
API/Workflows/D1/agent belong to Infra; the deployment interface belongs to Panel.
Preserve extracted behavior/private boundaries and use immutable artifact promotion.
Hermes is operator-confirmed Linux Mint with Docker. The protected Development
control plane and UI are deployed, and its dedicated agent is enrolled and healthy.
Milestones 3 and 4 passed; further deployment work is paused by the latest operator
priority above. Production is not ready and no production cutover occurred.
See Infra deployment/repository-catalog.json, repository-audit.json and
IMPLEMENTATION.md for the live verified matrix, conflicts and milestone evidence.
