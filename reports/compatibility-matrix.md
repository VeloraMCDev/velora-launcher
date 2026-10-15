# Compatibility matrix

Source support is not acceptance evidence. All persistent IDs, schemas, signing/credential identities and deployment volumes remain unchanged in this batch.

| Surface | Source / contract | Validation status |
| --- | --- | --- |
| Windows desktop | Launcher release workflow: NSIS installer; upgrade script | Native launch/upgrade pending on Windows |
| macOS desktop | Universal DMG release; current machine arm64 | Rust tests attempted; native signed installer/upgrade/game launch pending |
| Linux desktop | AppImage/deb release; platform credential fallback | Native launch/upgrade/permissions pending on Linux |
| Application identity | `net.scopenet.launcher`, legacy package/update identities | Identity checker passes; no rename |
| Sync store | Existing `.scopenet/state.json` and removed-mod backup paths | Temporary-filesystem regressions; no migration |
| Credentials/accounts | Native keyring service and encrypted secret files | Existing resilience tests retained; OS restore roundtrips pending |
| Vanilla/loaders | Core metadata, Fabric/Quilt/Forge/NeoForge modules and ignored online test matrix | No general version-support claim; actual download/launch matrix pending |
| Java | Mojang runtime selection, overrides, loader requirements | Unit coverage is not downloaded-runtime/process acceptance |
| Server integrations | Separate Fabric/Paper/Velocity/Calagopus segments and Java toolchains | Java builds/server joins pending; identities unchanged |
| Panel revisions | Launcher manifest and revision sync; native SQLite schema/migrations | Synthetic sync regressions only; old/new Panel compatibility pending |
| Deployment continuity | `panel-data`, installed runtime paths, signing/session keys | Unchanged; exact-image backup/restore/deployment gates pending |

Acceptance must name exact Minecraft/loader/runtime/OS versions tested, fixture provenance, native command, result and limitations. Do not publish an unsupported universal matrix based on module names.
