# Velora monorepo

`VeloraMCDev/velora-launcher` is the canonical public repository. Auth, Panel,
Launcher, SDK, integrations, gameplay, Docs and Infra belong here. The owner
approved public access to the complete product source, including gameplay.
Infra retains a separate workspace, lockfile and operational lifecycle.

See [the actual boundaries](docs/BOUNDARIES.md) and segment READMEs. Existing
directory names and separate frontend lockfiles are deliberate build boundaries.
Installed identities, SQLite paths, volume names and signing continuity stay stable.

Remaining work: Panel/gameplay extension seam, exact per-service image acceptance,
canonical candidate source policies, native deployment transport, backup/restore,
and Production acceptance. Historical migration records certify their stated
checkpoints only. GitHub Actions is enabled on the canonical repository; Source
CI runs automatically on public PRs and main pushes. Release/packaging remains manual,
and Actions stays disabled on the deprecated source repositories.

Other project repositories are preservation sources pending retirement. Keep
history custody before archiving; do not delete them as part of consolidation.
The complete source is published from one reviewed root commit. Original private
history is preserved separately; no old refs, PRs or artifacts were imported.
See [publication status](docs/security/PUBLICATION_STATUS.md). Deployment policy
must explicitly admit the new repository identity before registration resumes.
