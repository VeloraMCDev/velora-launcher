# Independent bounded API schema review

Date: October 12, 2026. Root instructions and required repository boundary/validation/deployment documents read. Reviewed selected producer/shared/Rust/TS consumer migration independently; no source modifications or heavy builds. This is not an all-API or deployed-client acceptance review.

## Verdict and checkpoint recommendation

No must-fix schema or IPC compatibility defect found in the selected migration. Recommend VEL-009 checkpoints 1–4 for the owned family inventory, selected shared consumer types, external validation fixtures and migration/compatibility matrix, conditional on lead final source/build reconciliation. Do not label remaining stable Value routes fully typed or count native/deployed consumers. Local source/contract evidence, not release readiness, is the credit boundary.

## Current migration and producer matrix

| Family | Producer/shared wire | Rust and TS consumer integration | Remaining limits |
| --- | --- | --- | --- |
| Connections config | Producer `public_config` now validates/returns ConnectionsConfig; both bool flags required, extensions retained | TS contract and common fixture exist | No launcher runtime config consumer found; export is not consumer migration |
| Discord start/link-start | Producer typed DiscordStartResponse retains url/state and extensions | Both commands decode typed and project url/state; SignIn/Settings use shared TS type | Known successful URL is still user-facing data, not an allowlisted navigation policy |
| Nullable Discord connection | Producer typed Option<DiscordConnection>, ID/name strings; null retained | Host known-field projection and Settings nullable type | Unreviewed future fields are intentionally not IPC-forwarded |
| Discord poll | Producer typed authenticated/pending/linked response with extensions | Host typed decoder plus real persistence adapter; TS consumers use token-free discriminated IPC union | No raw auth tokens or arbitrary extensions forwarded; actual account persistence separately reviewed |
| Discord info | Existing public_info producer stays unchanged; shared nullable/optional name model | Host typed/projected info; Settings uses shared type | Producer is not itself converted to the DTO in this patch |
| Discord unlink | Producer returns typed ok response | Host preserves unit success/204; TS DTO exported | Host intentionally does not require a JSON unlink response or return ok DTO over IPC |
| Login/register/username, manifests, profile/social/economy | Existing shared models | Existing typed consumers routed through client | Not a new comprehensive schema redesign |
| Stats/leaderboards/server/map metadata, several guild/reward/notification/market/vault families | Existing producer shapes, often stable | Bounded Value decoding retained where no reviewed shared model exists | Comprehensive stable DTO coverage is incomplete; these must not all be called inherently dynamic |
| Casino/board/action families | Route-polymorphic gameplay contracts | Existing bounded dynamic decoding | Explicit dynamic boundary, not shared-platform gameplay coupling |

`api-schema-inventory.md` and the initial implementation handoff contain now-historical “pending/exported only” integration wording. This table reconciles actual selected consumers: start/poll/connection/info are integrated, config has no located runtime consumer, unlink deliberately retains unit IPC. Inventory is family-level rather than exhaustive OpenAPI. Native/deployed consumer and remaining endpoint modeling gates stay open.

## Wire and trust-boundary findings

- New shared types are neutral models with no gameplay/application-host dependency. Producer methods/route registrations, query names, existing auth host/store behavior and ordinary success JSON are preserved. Ignoring EOL normalization, connections.rs changes only add the typed conversion helper/import and five response wrappers; stats routes/shapes are not modified. Static conversion failures use `invalid launcher contract`, not reflected serde payload text.
- Unknown extensions survive selected producer/shared DTO round trips, including nested user/game-token objects. Known field types remain validated. `Option<Option<T>>` plus present-nullable deserialization preserves missing versus explicit null for info display_name and auth yggdrasil; absent auth pending defaults to false when converted to existing AuthResponse while explicit null pending is rejected. No extra field is lost from the ordinary producer wire solely by these wrappers.
- Poll discrimination tests token presence first. Malformed auth cannot fall through to a non-auth success. Non-auth linked/pending discriminants must be true; TS uses the same literal discriminants. Rust and TS numeric/user/token requirements agree within the existing numeric-ID representation; this is not a new safe-integer migration.
- Producer extension preservation is deliberately distinct from the host IPC boundary. Authenticated poll returns only a persisted Account; Pending/Linked return one literal known flag. Start/connection/info drop unknown fields, preventing injected account/secret extensions from traversing IPC. The TS token-free poll union has no index signature and rejects raw Panel auth; runtime projection, not TS annotation alone, provides that guarantee.
- All 124 IPC signatures/registrations remain unchanged. Value result types are retained rather than replacing their public Rust signature with a DTO. New internal DTOs preserve known existing JSON shapes; removed arbitrary extensions and stricter malformed-response errors are explicit safety changes, not a promise of every possible extension's compatibility.

## Independent evidence and gates

- Ran Node 24 `--test tests/command-surface.test.mjs tests/api-schema.test.mjs` from launcher: six passed (four IPC plus two schema), no failures/skips. One shared JSON file feeds the Rust round-trip and actual TS compiler tests. Positive fixtures cover config/start/nullable connection/info/pending/linked/auth omission/null/token variants/unlink/token-free IPC. Negative TS tests reject raw auth IPC, missing start URL, numeric Discord ID and false pending. A root-cwd attempt failed type-library resolution; the supported segment-cwd rerun passed, so that failure is not a product schema defect.
- Read shared deserializer/negative fixtures for malformed auth, required fields, nested types, unknown extensions and omission/default compatibility. Latest lead-run native-final.log contains 19 shared tests and 70 host tests passed, including shared common-fixture test, six client tests and four actual poll-adapter tests. No reviewer Cargo execution; final build exit/source identity remains lead-owned.
- Inspected panel-schema-check.log: Panel check completed with only the two reported pre-existing dead-code warnings (`op`, `clip`). This is compile evidence, not live OAuth/HTTP routing or deployed consumer acceptance. No producer stores were opened.

## Reviewed snapshot

| Path | SHA-256 |
| --- | --- |
| crates/shared/src/launcher_contracts.rs | 6cb44d80b8933d49145adca699ea636c143da836d66f2811f8626e50b7a028c3 |
| crates/shared/test-fixtures/launcher-contracts.json | 2ab6860a73265a8a6de443e9ce54e5348fccb2373b3f11daf53ab36681ebd0d3 |
| panel/server/src/routes/connections.rs | 73b05262a4533151537decd8e2765b6eccbf0aa2f511e964fe241ed39dda163d |
| launcher/src/lib/types.ts | 5a7715fd9c5a5f76424bb0d6884ed2b2a7f5b4ad854969e038d7f662d56ec1b3 |
| launcher/src/components/SignIn.svelte | 99e28c6661cb9a7c92110c12f0408b63ff95066eac18909ac5b01cc9a631ab49 |
| launcher/src/pages/Settings.svelte | 85d2c28ab374ef799adc4a665ca87e8e33cc125f7ab91f14422ecd951bd8d95b |
| launcher/tests/api-schema.test.mjs | 26ff6aed7de9482d44b6aa05fad277f8ed89364d51ff54d8249a353b5ac06c55 |

Client/handler hashes are recorded in the companion client report. Substantive later changes require focused reconciliation rather than automatic extension of this verdict.
