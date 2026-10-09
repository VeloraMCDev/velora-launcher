# Security and publication

This repository publishes the reviewed complete product source. See
[publication status](PUBLICATION_STATUS.md) for source/history custody,
[build artifact provenance](BUILD_ARTIFACT_PROVENANCE.md) for wrapper review,
and the tracked publication-policy.json for the approved scope. Superseded
publication proposals and migration audit reports remain in retained history.

`node scripts/check-publication.mjs` inventories tracked sensitive filenames,
restricted paths, binaries, Git LFS pointers and commit-email exposure without
printing file contents or addresses. Add `--public` to enforce the recorded
publication policy. `--checkout` rejects tracked credential filenames, compiled releases,
legacy screenshots, operator evidence and unreviewed LFS pointers.
This inventory supplements a secret scanner; it does not replace one.

`node scripts/check-secrets.mjs` runs Gitleaks 8.30.1 over a temporary tracked-only
export. Install that pinned version on PATH or set `GITLEAKS` to its executable.
The report is redacted and deleted after checking. Reviewed synthetic fixtures
and provenance matches are accepted only at their exact rule/path/line and
normalized whole-file SHA-256. Any file edit invalidates its review; review new
matches before changing `reviewed-secret-findings.json`. Inline allow comments
are disabled. CI checks current tracked source; full history is a separate gate.

Keep live credentials in the deployment system's secret storage. Use synthetic
examples in source, tests and documentation. Keep operator databases, private
keys, enrollment output, local Wrangler state and audit raw output untracked.

If a real secret is discovered in any commit, revoke or rotate it first. Removing
it from the latest tree does not remove it from history. Coordinate a history
cleanup and scan the resulting refs before a visibility change. Review GitHub
releases, packages, Actions artifacts, issues, pull requests and discussions
separately: those are outside a source checkout scan.
