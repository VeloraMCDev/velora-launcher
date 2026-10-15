# Independent bounded core-boundaries review

October 12, 2026. Reviewed the core dependency-edge cleanup and synthetic loader
profile tests against actual production parsing, merging and rules code. No
Cargo commands, services, live stores, production/test edits or commits by this
reviewer. Other downloader/sync/native host changes are outside this verdict.

## Verdict and checkpoint scope

No blocking compatibility or boundary defect found in this scoped diff.
VEL-010 checkpoints 1 and 2 have narrow source evidence: unused direct dependency
edges removed and existing reusable boundaries retained. Checkpoint 3 has source
fixtures validated in the inspected lead log, supporting bounded checkpoint 3
acceptance. Checkpoint 4 remains open: this is not independent native
acceptance or an actual loader/toolchain/game-version matrix.

## Dependency and production comparison

- `crates/core/Cargo.toml` removes exactly three direct edges: `sha1`, `sha2`,
  `hex`. `Cargo.lock` removes those same three entries from only the
  `velora-launcher-core` dependency list. Package versions/checksums and other
  consumers' dependency edges are unchanged in the inspected diff.
- Source search finds no `sha1::`, `sha2::` or `hex::` use in core source. Core
  continues using existing `velora-platform-utils` reexports for HTTP, metadata,
  paths, authlib and textures. Digest implementations/identity semantics are not
  moved or removed by this change. No new gameplay or complete-host dependency
  is introduced and no workspace membership/lockfile lifecycle is altered.
- `loaders/mod.rs` only adds the test-only module declaration (and normalizes the
  existing Forge declaration line ending). Fabric/Forge production source,
  `VersionJson`, `merge`, `rules`, launch selection and protocol strings are
  unchanged by these edits. No new Quilt/NeoForge loader implementation is
  claimed or supplied by the test names.

## Fixture review

`crates/core/src/loaders/compatibility_tests.rs` invokes actual serde parsing and
`crate::version::merge`, not a copied merge. The four tests exercise:

1. Synthetic Fabric/Quilt/Forge/NeoForge-labelled children retain their literal
   ID/main class, inherit parent Java/assets/client metadata, concatenate
   arguments and prefer the child's same-Maven-key library. Unknown extension
   acceptance matches the current serde model without `deny_unknown_fields`.
2. Regular and natives-map libraries with the same Maven key remain distinct;
   Windows/Linux/macOS classifier strings survive. This matches `merge`'s
   `#natives` distinction; it does not test native extraction or OS selection.
3. Child legacy `minecraftArguments` wins while inherited modern arguments retain
   existing semantics. Quick-play detection uses production
   `supports_quick_play`, which checks argument values, not rule eligibility.
   The ruled fixture therefore tests parser/detection shape only. Production
   `rule_matches` and `allowed` still enforce their separate environment behavior;
   the new fixtures do not evaluate a runtime rules matrix or actual launch args.
4. Missing/non-string IDs and null required library names fail production serde
   deserialization. This is intentionally not a full loader schema validator.

No live URLs are contacted: fixtures use `synthetic.invalid`. Main-class strings
are literal existing protocol-facing examples, not renamed runtime identities.
Tests add coverage without modifying persisted IDs, stores, download locations,
updater/keyring/signing identities, credentials or application lifecycle.

## Validation provenance and remaining gates

The inspected prior `/tmp/velora-revitalization-milestone/native-full.log` contains
57 passing core tests, one ignored regression and seven ignored online tests.
That prior run does not contain `compatibility_tests`. Re-review inspected
`/tmp/velora-revitalization-50/download-final.log`: all four named loader tests
pass; core reports 65 passed, zero failed/ignored, utility reports 10 passed,
zero failed/ignored, and seven online tests remain ignored. The eight additional
core passes relative to 57 comprise four loader fixtures, one enabled cancellation
cleanup contract and three explicit-limit fixtures, not eight loader-matrix tests.
No duplicate tests were run by this reviewer. Frontend whole-tree validation
remains lead-owned. Native launches, installer upgrades, real profile
downloads, loader processors, OS classifiers and game compatibility remain
unverified by this bounded review. No false native/loader-matrix claim found in
the inspected fixture source or checkpoint definitions.

## Reviewed SHA-256 snapshots

| Path | SHA-256 |
| --- | --- |
| `crates/core/Cargo.toml` | `20284cd750f187bd7eaa862de1dde37736fb371bb45ee73f76cbf878911aa002` |
| `Cargo.lock` | `a47e070cd2fcbf55cdf21339d11a57c8f7ac65a3f07ee5ea433dfa1fa688a8ab` |
| `crates/core/src/loaders/mod.rs` | `20b37ae7ae6684e518cb12cf87476a89921365af1326fa7a8e0192eb21ea742e` |
| `crates/core/src/loaders/compatibility_tests.rs` | `efe279cb7f8efab0fe8edc4a0def3b945e409f69ac82c25b5bc78756eee8430f` |
| `crates/core/src/version.rs` | `b80e022170052dc64febd815de7c1b1702cd63a8b154192c357a9a5b42ae8ba9` |
| `crates/core/src/rules.rs` | `aa50f61e49726a5988d66993e6f8e456661ff2cdd03e3c8f4daf9a299097d0ad` |

Only this report and `frontend-domains-review.md` are authored by this reviewer.
