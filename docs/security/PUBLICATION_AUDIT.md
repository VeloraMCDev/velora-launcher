# Publication audit — 2026-10-08

**Historical decision: keep `VeloraMCDev/experiences` private.** That repository
retains the original history. The owner subsequently selected publication of the
cleaned complete source as a fresh `VeloraMCDev/velora-launcher`; see
[current publication status](PUBLICATION_STATUS.md). The sections below record
the original custody review and do not block the separately reviewed new lineage.

## Current-tree cleanup follow-up

Public-release preparation removed 61 tracked files: 17 obsolete installers/game
JARs, 33 legacy screenshots and 11 operator evidence reports. Three legacy
documents now describe removed screenshots rather than embedding them. Original
source inventory hashes describe the private preserved originals, not this
cleaned tree. A verified private Git bundle retains the pre-cleanup history.
No shared history or repository visibility has been changed.

The cleaned tree passes the tracked-filename policy. A fresh tracked-only
Gitleaks scan has nine reviewed matches and no unreviewed match: synthetic test
values, the PEM-header assertion and provenance revisions. Exact whole-file
hashes constrain their acceptance in CI. Retained PNG/JPEG images were visually
reviewed as logos/banners/icons, with no visible player records or credentials;
this is not a license grant. Follow-up checked all 30 PNGs for text/EXIF chunks
and found none. All four Gradle wrapper JARs match an official upstream checksum;
maintained wrappers pin the official distribution checksum and retain notices.
See [build artifact provenance](BUILD_ARTIFACT_PROVENANCE.md).

The owner subsequently approved publication of the complete product source,
including gameplay and SMP/Frontiers. Extraction-time private visibility records
are historical; source publication is approved without a blanket license change.
Removed material still exists in earlier commits and GitHub PR refs. Author
metadata and GitHub attachment/history review remain open.
See [the preparation record](PUBLICATION_PREPARATION.md).

## Credentials

Gitleaks 8.30.1, downloaded from its official GitHub release and checked against
the release SHA-256, scanned tracked-file exports and all fetched Git refs of
experiences, authentication, panel, launcher, sdk, minecraft-integrations, infra
and docs. Archive traversal was enabled to depth two; secret values were fully
redacted. Untracked operator state and dependency/build directories were excluded.

| Repository | Tracked files at initial scan | History matches | Checkout matches |
|---|---:|---:|---:|
| experiences | 1,878 | 8 | 6 |
| authentication | 97 | 1 | 1 |
| panel | 122 | 2 | 2 |
| launcher | 60 | 0 | 0 |
| sdk | 130 | 0 | 0 |
| minecraft-integrations | 48 | 0 | 0 |
| infra | 132 | 2 | 2 |
| docs | 33 | 0 | 0 |

All scanner matches were reviewed. No live credential was confirmed in those
matches. They were a runtime-generated key's PEM-header assertion, synthetic
Discord IDs/JWT acceptance values, source revision identifiers and artifact
checksums. Historical matches in `application/scripts/migration/record-panel-gateway.mjs`
are provenance revisions, not API credentials. Raw reports remain in the ignored
`.runtime-checks/publication-audit/` directory, not in this repository's history.
This result is not proof that every possible secret or personal record is absent.

The initial consolidated checkout inventory contained 858 restricted-path files,
79 binaries/images requiring review, no tracked database/private-key filenames,
no Git LFS pointers, and three distinct non-GitHub-noreply commit email addresses.
Counts describe the initial 1,878-file tree and must be rerun after consolidation.

The Docker build context previously omitted exclusions for ignored operator state
and environment credentials. `.dockerignore` now excludes those paths, private
database/key files, audit/history backups and Infra. This protects builder inputs;
Git ignore rules alone do not control Docker's build context.

The expanded 2,188-file consolidated checkout was scanned again: ten matches,
all the same reviewed fixtures, provenance revisions and checksums imported from
the source repositories. Its inventory has 933 restricted-path files, 84
binary/image files, no tracked sensitive database/key filenames or LFS pointers,
and three distinct non-noreply commit email addresses. Subsequent audit/validation
documentation changes contain only reviewed descriptions and aggregate counts.

The retained `scopeddlol/SCOPENET-MC` source history was also scanned separately:
250 matches reduce to six rule/path/line groups across repeated extraction
commits. They are the same reviewed test values and source revision records,
including an older generated-key header assertion. That history was not merged
into the canonical repository's ancestry; it remains private preservation custody.

## Publication blockers

| Area | Evidence | Required resolution |
|---|---|---|
| Gameplay scope | Extraction-time provenance used private visibility; current owner decision approves the complete product source | Resolved for source publication; retain per-package license boundaries and source attribution |
| Historical exposure | Gameplay and legacy material already exist in earlier commits | A latest-tree deletion is insufficient; review all refs or create a clean, approved publication repository |
| Player identity in images | `migration/legacy-source/docs/images/companion-overview.png` displays a player handle | Replace with synthetic screenshots in a publication export; review image history as well |
| Binary releases | Frozen `migration/legacy-source/release-artifacts/` includes Windows installers and Java archives | Inspect embedded resources/configuration and release rights; source scanners do not certify compiled binaries |
| Retained assets and third-party terms | Branding is included in the complete-product decision; wrapper hashes match official Gradle binaries and notices are retained | Current retained build/branding source reviewed; preserve existing terms and do not extend MIT to unrelated assets |
| Commit metadata | Author/committer identities and email addresses are retained by Git | Obtain publication consent or sanitize a publication export; inventory counts are available from the check script |
| Operational material | Infra evidence, recipes and migration records describe deployment and repository structure | Review retained hostnames, routes, infrastructure IDs and operational evidence for intended disclosure |

The visually reviewed legacy screenshots included launcher updates/admin, launcher
about, companion overview and companion quests. Remaining images and historic
image revisions need review; this is not a blanket image clearance.

## Reproduce before publication

From a fresh clone containing every branch/tag intended for publication:

```sh
git fetch --all --tags
node scripts/check-publication.mjs --public
gitleaks git --log-opts="--all" --redact=100 --ignore-gitleaks-allow --report-format=json --report-path=/private/audit/history.json .
```

Export only tracked files into a separate temporary directory, then run
`gitleaks dir --redact=100 --ignore-gitleaks-allow --max-archive-depth=2` against
that export, with a private report path. Do not scan an operator's live data as a
substitute for reviewing what Git actually publishes. Review redacted findings;
do not use broad allowlists to silence unknown matches.

Scope excludes credentials held only in GitHub/Cloudflare secret stores, live
databases, unreachable Git objects, unfetched refs, GitHub release attachments,
packages, Actions artifacts, issues/PR discussions, LFS payloads not downloaded,
and a complete reverse-engineering review of installers. Audit again after source
changes or additional history imports. Preserve the private source archive;
publication cleanup should be a separately reviewed operation.

Scanner reference: [official Gitleaks documentation](https://github.com/gitleaks/gitleaks).
