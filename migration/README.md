# Source preservation checkpoint

`legacy-source/` retains the historical source used for compatibility and migration
attribution. Publication preparation removed old installers, release JARs and
screenshots and replaced their documentation embeds. It is no longer a complete
byte-for-byte snapshot. The original private source repository and verified private
Git bundle preserve the complete original.

`legacy-source.json` records original source byte counts and hashes, not a manifest
of the cleaned checkout. See [the cleanup record](legacy-source/PUBLICATION_CLEANUP.md).
Do not grant a blanket license to mixed legacy code or publish retained history
before the [publication review](../docs/security/PUBLICATION_PREPARATION.md).
