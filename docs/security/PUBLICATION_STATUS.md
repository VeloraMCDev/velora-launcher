# Complete product source publication

The owner selected `VeloraMCDev/velora-launcher` as the new public source repository.
It contains the complete product, including gameplay, SMP/Frontiers presentation,
Auth, Panel, Launcher, integrations, SDK and deployment tooling. The name does not
limit the source scope to the desktop application.

The public lineage begins with one reviewed root commit using a GitHub-noreply
author/committer. It imports the cleaned source tree only. Original history,
merged PRs, historical installers/screenshots, release artifacts and operator
evidence remain in the private `VeloraMCDev/experiences` preservation repository.
No original branch, tag, PR, Actions artifact, secret or environment was imported.
Historical provenance links and hashes retain their original attribution.

Current-tree filename checks and the tracked-source credential scan pass. The
nine exact reviewed matches are synthetic fixtures, the generated-key-header
assertion and provenance hashes. Wrapper binaries match official Gradle checksums;
branding metadata and source were reviewed. Source publication changes no package
license, signing identity, protocol namespace, database path or persistent volume.

The publication policy applies to this new Git/GitHub custody. The old repository's
history/artifact review remains private preservation work and is not imported as
part of public source. Future branches, attachments and artifacts need their own
review. This audit is not a promise that every possible secret can be detected.

## GitHub Actions and deployment

Source CI uses one bounded standard Ubuntu runner with read-only permissions,
pinned actions, no artifact upload and no deployment credentials. Desktop/image
packaging, application acceptance and releases remain manual. No paid plan,
runner or infrastructure was added.

The new GitHub repository ID is `1411074604`; the old preservation repository ID
is `1403902382`. Active artifact generators identify `VeloraMCDev/velora-launcher`.
Cloudflare candidate registration and deployment remain paused. Before resuming,
review immutable repository ID/workflow/SHA source policies and GHCR grants against
the new source. Existing policies must fail closed; public CI does not grant
deployment authority or silently migrate live state.
