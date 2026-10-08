# Migration merge checkpoint (2026-10-07)

The operator prioritized migration and repository merges before further deployment
implementation. Reviewed component extractions are landing on the owner repositories'
default branches using merge commits, retaining source provenance and snapshot pins.
No branch history is rewritten and no repository visibility is changed.

## Verified owner merges

Each PR was conflict-free and its current required checks passed before merging.

| Owner PR | Reviewed head | Merge commit |
| --- | --- | --- |
| [SDK #1](https://github.com/VeloraMCDev/sdk/pull/1) | `9d132b1fa09e651229fd4fb52dc2fcf0f782b36c` | `de3b8fd7539c3b9d20fbc8a3b58317f61a321faa` |
| [Authentication #1](https://github.com/VeloraMCDev/authentication/pull/1) | `2e2c3ec89ea5dd0791c6575686e13449b5ac1db3` | `5162abd28300300667ea2b029fc63551cb966977` |
| [Minecraft integrations #1](https://github.com/VeloraMCDev/minecraft-integrations/pull/1) | `4a7cbb14ce163b2f1ce50854ae364b97825a9450` | `410bad40189c3af73d24f7cf37be57a4a86f14b9` |
| [Launcher #1](https://github.com/VeloraMCDev/launcher/pull/1) | `1f3652101f47e6029ea917eae6f12048dc3ae8fa` | `404d3b6a007f7b795e2d3efbabd3cb2308d9e362` |
| [Experiences #1](https://github.com/VeloraMCDev/experiences/pull/1) | `64dfcffeabfbc626913b6557f59516b83b13b853` | `86c4cabdf3de021d0a402a613226ef34ce382a47` |
| [Panel #1](https://github.com/VeloraMCDev/panel/pull/1) | `750b886cc89a56e664babfea226e53a4fb14a036` | `5b902a65ca788a1c1caefc8fcec328aeaf469fff` |
| [Infra #1](https://github.com/VeloraMCDev/infra/pull/1) | `1c7f00999b85cecc7c9462675bbefc65426d7781` | `ecf17d815897f9036f1480857858c4bd1c3e561a` |

[Docs #1](https://github.com/VeloraMCDev/docs/pull/1) contains this checkpoint and is
the final owner merge pending its updated documentation checks. The compatibility
[source PR #41](https://github.com/scopeddlol/SCOPENET-MC/pull/41) merged as
`2db2426648574dd374f6a5a26ca16e4f92f0864c` after all seven fresh CI jobs passed
at `e2afb33688ef85a0f045ba1579c004b312a1fa9b`
([run 37721352880](https://github.com/scopeddlol/SCOPENET-MC/actions/runs/37721352880)).

Seven public-source preflights pass, including tracked files and reachable history.
Experiences remains private. All 796 preserved archive files were checked again
against the SHA-256 inventory of frozen source
`57daa92cb6cb4027982d06bfa1c692ab9edb88ff`. Public owner builds do not resolve a
private Experiences dependency. The inventory still records incomplete final
mixed-file disposition; byte preservation alone is not application migration.

## Remaining migration gates

- Authentication: real policy, audit, presence, transaction and email adapters;
  admin/provider and gateway composition; credential-writer and data cutover.
- Panel: complete admin/player/landing backend and UI, composing public components
  with optional private pages while preserving existing routes and authorization.
- Launcher: independent Tauri desktop/UI and updater composition, signing custody
  and native upgrade acceptance; its owned installation/launch engine already builds.
- Minecraft integrations: full loader/admission/companion composition around the
  owned neutral producer and shared Java API, preserving private gameplay boundaries.
- Experiences: remaining route, schema/migration and service composition around
  maintained private Rust/Java/UI packages, retaining data and gameplay behavior.
- SDK and cross-repository integration: complete generated/cross-language API
  coverage, final ownership/disposition, and independent consumer builds.
- Whole product: fresh install, existing-data upgrades, outages, offline restore,
  rollback and release parity before replacing the compatibility application host.

The protected Development panel and Hermes heartbeat agent have passed deployment
guide Milestones 3 and 4. Milestone 5 execution/rollback and broader infrastructure
onboarding remain paused until migration work resumes its requested priority.
There is no production cutover, package publication or visibility change in these merges.
