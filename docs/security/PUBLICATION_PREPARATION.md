# Public repository preparation

This document records preparation and earlier publication choices. The owner
selected a new public `VeloraMCDev/velora-launcher`, preserving the old repository
privately without renaming it. See [current publication status](PUBLICATION_STATUS.md)
for the resolved source/history custody. Earlier pending-review descriptions below
refer to the private preservation repository.

The current tree has had obsolete installers, release JARs, legacy screenshots
and recorded operator evidence removed. A verified local private Git bundle
preserves the pre-cleanup history. It is not an independently stored backup.
Runtime implementations, installed-client identities and data paths are retained.

## Approved source scope

On 2026-10-08 the owner selected **Publish complete product source**. This includes
gameplay, SMP/Frontiers presentation, storage, the complete application hosts and
all reusable platform segments. A platform-only export is not the target.

Existing `private-*` directory/crate names remain for compatibility. npm
`private: true` and Cargo `publish = false` prevent accidental package-registry
releases; they do not prohibit public Git source. Reusable platform libraries
remain independent of gameplay. Per-package licenses and attribution are retained;
approval to publish source is not a blanket MIT/trademark/binary distribution grant.
Historical provenance retains the extraction-time visibility policy, with the
current decision recorded in [publication-policy.json](publication-policy.json).

`Source CI` runs on pull requests and main pushes after the repository is public.
It uses one standard Ubuntu runner, read-only credentials, pinned actions, a
15-minute timeout and cancellation of superseded runs. It checks compatibility
identities, tracked publication filenames and credentials, workflow policy, Infra tests and
Worker types, and documentation tests/links/build. It has no deployment or
artifact-upload steps. Rust, UI, acceptance, desktop packaging and release image
workflows remain manual; this fast suite does not replace release validation.

The automatic job skips private repositories to avoid consuming the exhausted
private allowance. Repository-level Actions has been re-enabled for the canonical
repository; the deprecated repositories remain disabled.
[Standard hosted runners are free for public repositories](https://docs.github.com/en/billing/concepts/product-billing/github-actions).
Storage and larger runners have separate billing; this workflow uses neither
uploaded artifacts nor larger runners.

Remaining publication reviews:

- Retained branding metadata and Gradle wrapper provenance have been reviewed;
  see [build artifact provenance](BUILD_ARTIFACT_PROVENANCE.md). Keep their terms.
- Sanitize or replace published history containing removed screenshots and
  installers and review author metadata. Old branches, tags and pull-request refs
  need review too.
- Review releases, packages, Actions artifacts and issue/PR attachments before
  changing visibility. Those are outside a source-tree scan.
- Re-scan the final checkout and every ref intended for publication.

The [concrete cutover proposal](PUBLICATION_CUTOVER.md) preserves the old GitHub
custody privately and starts the public canonical source from the reviewed tree.
It requires approval for replacing the repository identity/lineage; source
publication approval alone does not choose this preservation strategy.

`check-publication.mjs --checkout` enforces the current-tree filename policy;
`--public` reports the pending reviews recorded in `publication-policy.json` and
remains blocked until those reviews finish. Neither option
certifies credentials by itself. See [the credential audit](PUBLICATION_AUDIT.md).
