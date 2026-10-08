# Migration instructions

The canonical public repository is VeloraMCDev/velora-launcher.
Read MONOREPO.md, docs/BOUNDARIES.md and docs/VALIDATION.md before changing code.
Historical migration tracker/stage reports live in docs/migration (paths from repo root).
Review docs/security/PUBLICATION_AUDIT.md before any proposed public release.
Run local checks while publication is prepared. Source CI runs automatically only
in public repositories; expensive packaging and release workflows remain manual.
Add no paid infrastructure or automatic deployment as part of source organization.
Use this checkout directly; do not create a worktree unless the user requests one.
Do not erase existing behavior or rename persisted IDs, database paths, volume names,
Minecraft protocol identities or updater/signing identities without compatibility tests.
The owner approved publication of the complete product source, including gameplay
and SMP/Frontiers. Retain gameplay/package names and the reusable library boundaries.
Public builds must require no inaccessible repository. Do not import mixed legacy files
wholesale. Preserve source revision, checksums and attribution for each extraction.
Do not claim a placeholder is runnable. Record validation and incomplete tasks honestly.
Use generic, synthetic examples; never commit credentials, player data or operator URLs.
MIT applies to platform code; artwork/trademark/private distribution terms are separate.
