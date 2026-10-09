# Development instructions

The canonical public repository is VeloraMCDev/velora-launcher. It includes the complete product and gameplay.
Read MONOREPO.md, docs/BOUNDARIES.md, docs/VALIDATION.md and docs/DEPLOYMENT_TARGETS.md from the repository root.
Use this checkout directly; create a worktree only if the user requests one.
Keep reusable platform libraries independent of gameplay and complete application hosts.
Preserve persisted IDs, database paths, volume names, Minecraft protocol identities, credential-store and updater/signing identities unless compatibility is explicitly tested.
Keep each segment's lockfiles, licenses, attribution and independent build/deployment lifecycle.
Run relevant local checks. Source CI runs automatically; packaging and releases remain manual.
Do not introduce paid infrastructure or automatic deployments during source cleanup.
Never commit credentials, player data, operator addresses or local runtime stores. Screenshots must use synthetic data.
Retain operational SQL schema migrations; these are runtime inputs.
Describe implemented behavior and remaining gates honestly. MIT does not cover every file or Velora artwork/trademarks.
