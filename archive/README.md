# Archive

Source that Velora no longer builds or ships. Nothing here is part of CI, packaging or a release, and it is not maintained. It is kept in the repository so history, licenses and attribution stay intact and so it can be restored with `git mv`.

| Path | What it was |
| --- | --- |
| `integrations/paper` | Paper/Purpur/Spigot plugin and its legacy API wrapper (`integrations/api`). |
| `integrations/forge` | Forge loader build. |
| `integrations/fabric/modern.gradle`, `integrations/fabric-client/modern.gradle` | Fabric 26.x server and client builds. |
| `integrations/fabric-client/src/26`, `integrations/minecraft/src/26` | Minecraft 26.x sources. |
| `integrations/minecraft/src/1.21.1` | Minecraft 1.21.1 adapter sources. |

Velora Core targets Fabric 1.20.1 only. The compatibility identifiers (`net.scopenet` packages, the `scopenet` mod ID, protocol channels and config names) are unchanged, so restoring an archived module only needs its files moved back and its `include` line re-added to `integrations/settings.gradle`.
