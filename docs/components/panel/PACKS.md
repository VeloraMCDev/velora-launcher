> Historical owner documentation from `VeloraMCDev/panel` at `a5ebb0e81c653a22095893dc71e7e400e5b0e6cc`. Current segment instructions are in the monorepo READMEs.

# Pack import and distribution boundary

velora-panel-packs independently implements the original Modrinth mrpack,
CurseForge manifest/API and plain instance zip pipeline, provider downloads,
checksums, loader/version normalization, override precedence, safe relative paths,
file URL encoding, transactional metadata application and post-commit file GC.
No account, private gameplay, application state or operator credentials are bundled.

## Host ports

PackHost supplies the existing HTTP client, distribution-files root, metadata write
and read pools, lazy CurseForge key lookup and clock. Provider origins default to
the original APIs; explicit origins allow local fixtures or host-supplied relays.
The host must authorize callers and supply trusted storage/pool/origin choices.
The library neither opens a database nor initializes production schemas.

The compatibility adapter preserves live secret lookup timing: Modrinth, plain
zips and CurseForge packs with no required files need no key. Error status/message
fields preserve original 400/500/502 mappings. Parsing/extraction runs on a blocking
worker. Plain-zip version validation still follows extraction; failed imports may
leave extracted files, matching legacy behavior. Limits and multipart validation
remain caller-owned.

Pack application keeps manual uploads, updates source/version metadata and bumps
revision in one SQL transaction. Its supplied clock is evaluated at the original
late update boundary. File GC runs after commit and retains original best-effort
filesystem behavior; it is not an atomic filesystem/database transaction.

## Build and test

    cargo +1.98.1 test -p velora-panel-packs --locked
    cargo +1.98.1 clippy -p velora-panel-packs --all-targets --locked -- -D warnings
    node scripts/verify-snapshots.mjs sdk-snapshot

Reviewed public SDK Rust sources are included with raw hashes and MIT license, so
builds need no sibling checkout or private Git credentials. They remain temporary
snapshots until versioned public packages are available. Synthetic archives and
local HTTP servers exercise the real code without provider accounts, player files
or external email/network requests. See PACKS_PROVENANCE.json.

Complete route/server/UI composition, public versioned packages and whole-product
installation/upgrade/release acceptance remain unfinished.
