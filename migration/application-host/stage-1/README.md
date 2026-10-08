# Velora stage 1 recovery checkpoint

These patches preserve all eight tested foundation commits because this cloud
connection can read VeloraMCDev repositories but cannot push to them. No target
branch was published. See ../../STAGE_1.md for scope, tests and remaining work.
The original monorepo applications remain unchanged.

All eight recovery patches were applied in fresh temporary Git repositories and
reconstructed identical source trees. The recovered SDK passed its six tests with
locked offline dependencies, without importing any sibling source.

Preferred next action after enabling organization write access: push the existing
`codex/stage-1-foundation` branches from /workspace/velora/<repository>.
Do not reset those local commits to make repository declarations pass.

If the original checkout is unavailable, verify each patch against checkpoints.json,
clone its empty destination, create the foundation branch and apply its patch:

```sh
git clone https://github.com/VeloraMCDev/sdk.git
cd sdk
git switch --orphan codex/stage-1-foundation
git am /path/to/migration/stage-1/sdk.patch
```

Repeat with the matching name for other destinations. Applying a patch preserves
content/author but may create a different commit ID. If the destination already has
work, inspect it and adapt through review; do not overwrite or orphan existing work.
The seven platform foundations are MIT; experiences contains no gameplay or license
grant. Nothing here changes repository visibility or performs deployment.
