# Final scoped local sweep and remaining gates

October 13, 2026. This is a changed-source/local regression sweep, not complete
monorepo, supported-OS, production or release acceptance. Base HEAD is
`4f639266a515c3cb05769b055a4772c4dcb89cb8`; tested source includes the uncommitted
working-tree changes. Fresh `git fetch origin` reports zero commits on either
side of `HEAD...origin/main`. No pull, stash, commit, branch, PR, signing,
packaging, store migration or deployment was performed. Logs for this batch are
in the private temporary directory `/tmp/velora-revitalization-final`.

## Implemented integrity follow-up

Java runtime ensuring now verifies a strict 40-ASCII-hex manifest SHA-1 against
the fetched bytes before decoding or install mutation. Manifest responses have
a 16 MiB advertised-length preflight and streamed-body limit. Online warm ensures
fetch the verified manifest and deep-check declared files instead of trusting
the marker alone; corrupt artifacts are repaired, and unchanged markers are not
rewritten. Public API, default Mojang index, platform/executable paths and
`.scopenet-runtime` identity remain unchanged. No dependency or path migration.

The separate reviewer independently passed 16 contracts and accepted this bounded
change. Current source hashes match `java-runtime-integrity-review.md` exactly:

| File | SHA-256 |
| --- | --- |
| `crates/core/src/java.rs` | `893bccb2c28ba4c32fa654b0d310dfd68c27b27a8c8c0af2cdbbe941f3465787` |
| `crates/core/src/java/profile_tests.rs` | `0c029ce897dc9dea98fe735586b21849272c6f5304e96e5282b409bd7ed24674` |

Warm now depends on manifest availability when the index succeeds and does extra
integrity work. Five tiny synthetic samples yield cold median 1.319 ms and warm
0.568 ms versus historical warm 0.271 ms; the 0.297 ms increase is recorded, not
hidden or claimed as a speedup. This is not native JVM/game performance evidence.

## Passing validation

Node 24.21.0, Rust 1.98.1, Gitleaks 8.30.1; retained segment lockfiles. Relevant
frontend dependencies were installed with `npm ci --no-audit --no-fund`.

| Scope | Command or checks | Result |
| --- | --- | --- |
| Launcher, core, utility, shared | `CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher -p velora-launcher-core -p velora-platform-utils@0.5.0 -p velora-shared@0.5.0` | 70 + 101 + 10 + 19 = 200 passed; doc tests pass |
| Panel library, separate run | `CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-panel --lib` | 72 passed |
| Launcher frontend | `npm test`; `npm run check`; `npm run check:runes`; `npm run build` | 98 passed; zero Svelte errors/warnings; build passes |
| Operations | Same four npm commands in `panel/operations` | 30 passed; zero Svelte errors/warnings; build passes |
| Documentation | `npm test`; `npm run check`; `npm run build` in `docs` | 4 passed; links/build pass; 42 emitted files |
| Infra source/local fixtures | `npm test` in `infra`; `npx --no-install tsc -p control-plane/tsconfig.json`; workflow source checks | 28 passed; TypeScript/workflow checks pass; no cloud deployment |
| Signing/mobile source fixtures | Existing launcher/core signing and mobile-config Node tests | 9 passed; synthetic signing only, no release credentials |
| Panel frontend | `npm run check`; `npm run check:runes`; `npm run build` in `panel/web` | Zero errors; 11 preexisting warnings in unchanged `CompanionBuilder.svelte`; build passes |
| Panel native application | `CARGO_PROFILE_DEV_DEBUG=0 cargo build --locked -j1 -p velora-panel` | Pass; 2 preexisting dead-code warnings |

Totals are **272 scoped Rust tests and 169 Node tests passed**, not including
reviewer reruns or explicit timing profiles. Core's three timing profiles are
ignored by default and separately exercised; seven preexisting online tests
remain ignored and are not acceptance evidence. Node VM Modules experimental
warnings and Panel's existing greater-than-800-kB bundle warning remain.

### Final source safety and consistency checks

After report reconciliation, boundary, identity, workflow, publication and
whitespace checks pass. Gitleaks scans 1,535 tracked files: six exact previously
reviewed findings, zero unreviewed findings. A separate private-temporary copy of
all **131 nonignored untracked additions** scans clean, without staging any files.
Local Markdown checks pass across 172 files and 101 link targets; external URLs,
HTML links and heading anchors are not certified. Checklist arithmetic validates
19 rows, 58 verified checkpoints and the unchanged denominator of 76. Commands:

```sh
node scripts/check-boundaries.mjs
node scripts/verify-identity.mjs
node infra/scripts/check-workflows.mjs .
node scripts/check-publication.mjs --public --checkout
node scripts/check-secrets.mjs
node docs/scripts/check-links.mjs .
git diff --check
```

The additions scan uses Gitleaks `dir --redact=100 --ignore-gitleaks-allow
--max-archive-depth=2` on the copied files and deletes only its own temporary copy.
After the validation-document command correction, Docs' four tests, link check
and 42-file build pass again; the emitted content manifest SHA-256 is
`e742213da7ebd274818df0572924f412423df371580e827d4e16a19c994f5a22`.
Repeated runs do not add to the distinct test totals.

### Synthetic native Panel restore

After the final incremental native build, ran
`node scripts/application-acceptance.mjs target/debug/velora-panel` with Node 24.
Final log timestamp: **2026-10-13T12:21:52.835Z**. Four check groups pass:
fresh synthetic admin/player authentication and isolation; cold byte-identical
backup of four files; restart preserving original credentials despite changed
bootstrap settings; separate restored directory preserving backup-point users,
roles and instances while excluding later writes. The script uses an ephemeral
loopback bind, isolated temporary data directories and sanitized inherited
environment, and stops its own server. This is not an exact-container/offsite
restore or acceptance against an actual user's installed store. The log's source
field records base HEAD, not a committed representation of the working-tree edits.

### Unsuccessful attempts, not passing evidence

The initial five-package command failed before tests because Panel introduces
another `velora-platform-utils` version. The version-qualified combined retry was
cancelled as feature-unified artifacts consumed scarce disk. Neither attempt is
passing evidence. The separate final runs above passed against current source
without renaming packages or deleting user artifacts.

## Credit and readiness decision

Award only **VEL-019 checkpoint 1**, the full scoped/local regression sweep:
**58/76 = 76.32%** fixed engineering checkpoints. The scope covers changed
launcher/core/shared/Panel code and adjacent frontend/source checks, not all Rust,
Java, legacy or mobile runtime toolchains. Earlier 57/76 milestone evidence stays
historical. No VEL-014 or further VEL-019 checkpoint is earned by these synthetic
profiles, one scoped reviewer or one synthetic restore. **Not release-ready.**

Required next work, in dependency order:

1. Reconcile actual provider billing and free development disk before another
   large implementation batch. The $1,000 hard cap remains; actual spend and
   balance are unavailable. Approximately 2.0 GiB is free; APFS reports 100%.
2. Implement durable interrupted-sync and cross-store transaction journals,
   restart recovery and process/I/O shutdown quiescence. These are missing
   implementation, not just unexecuted acceptance tests. Integrate durable cache
   reopen/resume only after its independent integrity/recovery contract passes.
3. Resolve/review authenticated manifest/cache fallback semantics, including
   auth revocation. Existing all-error cache fallback is not fail-closed proof.
   Java index-failure fallback still checks marker/executable presence, not
   offline integrity. Index-body bounding, manifest shape/artifact references,
   symlink/path policies, ignored permission/link errors, concurrent mutation and
   whole-runtime transaction durability also remain open.
4. Execute the supported-OS keystore/installer/upgrade, representative Java/game/
   loader, native UI/reconnection/accessibility and installed-data matrices using
   approved synthetic/backup fixtures. Establish comparable real preparation and
   startup metrics before performance claims.
5. Obtain final independent security/data/compatibility acceptance; build/test
   exact release images, offsite restore and pinned-digest rollback; perform
   approved signing/updater/release checks. Packaging/releases remain manual.

No percentage authorizes commits, publication, migrations or deployment. Further
large work is deliberately not started with nearly exhausted disk and unknown
budget. Existing operational SQL, identities and independent lifecycle boundaries
remain intact. See `completion-checklist.md` for the fixed definitions.
