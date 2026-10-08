# Proposed publication cutover

**Superseded choice:** the owner selected a new `VeloraMCDev/velora-launcher`.
The existing `VeloraMCDev/experiences` remains private with its original identity.
No repository rename or old-reference rewrite is required. See
[current publication status](PUBLICATION_STATUS.md). The original same-name
proposal below is retained as a planning record, not an instruction to rename.

The owner approved complete product source publication. The reviewed current tree
contains the full product, with obsolete installers, screenshots and operator
evidence removed. Runtime identities, storage paths and per-package terms remain.

## Why a latest-tree cleanup is insufficient

The existing private repository retains old branches, ten merged pull requests
after this preparation, and 13 Actions artifacts totaling 946,711,521 bytes at
the inventory checkpoint. It has no releases, repository Actions secrets,
environments, forks or enabled discussions at that checkpoint. Issue/PR bodies
contained no attachment links, but comments/attachments and packages were not
fully inspected. Old snapshots retain the removed files and three distinct
non-noreply commit email addresses.

[GitHub documents that force-pushing rewritten history does not remove old PR
references or cached commits](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/removing-sensitive-data-from-a-repository).
Its support process is limited to sensitive-data removals, so source cleanup
must not assume a routine history rewrite will clear every old reference.

## Recommended approach, pending owner approval

1. Freeze changes and verify a private complete-history backup of the current
   canonical repository. Retain all runtime databases and signing keys in place.
2. Preserve the current private repository as
   `VeloraMCDev/experiences-private-history-20261008`. Disable its Actions and mark
   it as the private preservation archive. Its PRs and artifacts remain private.
3. Create a fresh private `VeloraMCDev/experiences`, seed only the reviewed
   complete-product snapshot with one GitHub-noreply root commit, and configure
   the same default branch and scoped ownership guidance. Do not import old refs,
   PRs, Actions artifacts or release attachments.
4. Verify the new Git tree against the reviewed source, scan its complete history
   and checkout, confirm the new repository has no inherited attachments/secrets,
   and resolve the publication policy against this new custody.
5. Make the new repository public and enable the bounded standard-runner Source
   CI. Run it once. Keep release, packaging, registration and deployment manual.
6. Archive the private preservation repository after verifying access. Existing
   working copies must use the new source lineage; do not merge old history back.

The canonical public URL stays `VeloraMCDev/experiences`, but the GitHub repository
ID and commit lineage change. Old PR/commit URLs must point to the private
preservation repository where historical access is needed. Old stars/watchers,
repository settings and package linkage do not automatically migrate. No current
repo secrets/environments were found; public CI requires none. Inspect package
permissions before any manual image publication.

Cloudflare registration verifies immutable GitHub repository IDs, workflow paths
and SHAs. Record the new repository ID and review source-policy/registry grants
before enabling candidate registration or deployment. Do not loosen validation or
silently substitute the new ID into existing policy. The current paused live
deployment configuration stays paused throughout this cutover.

This approach adds no paid services. The private archive retains old objects and
artifacts until retention expires or separate cleanup is approved; the public
source begins with the reviewed tree only. Repository renaming/replacement and
archiving have not been performed by this preparation.

## Alternative

Keep the existing repository and finish reviewing every retained source revision,
binary, screenshot, author record, GitHub artifact and attachment before flipping
visibility. This retains its GitHub ID and PR history, but requires additional
review of material deliberately removed from the current tree.
