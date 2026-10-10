# Velora Core — Fabric 1.20.1

Server code lives in `../fabric` and `../common`; client code lives in `../fabric-client/src/main`. The Minecraft-specific seven-row vault menu is shared from `src/minecraft/java`. Modern-version and other-loader sources remain retained independently.

Use Java 17 or a newer JDK with the configured Java 17 target. Fabric API is required on both sides. The dedicated wrapper pins Gradle 8.10 with its distribution checksum for Loom 1.8.13. The repository's modern Gradle wrapper is unchanged. The wrapper's Apache license/notice remain in `../gradle/LICENSE` and `../gradle/NOTICE`.

From `integrations` on PowerShell:

```powershell
.\toolchains\fabric-1.20.1\gradlew.bat -p . '-Ploader=fabric' '-PmcVersion=1.20.1' :fabric:build :common:test
.\toolchains\fabric-1.20.1\gradlew.bat -p . '-Ploader=fabric-client' '-PmcVersion=1.20.1' :fabric-client:build
```

On Unix use `sh toolchains/fabric-1.20.1/gradlew` with the same arguments. Do not combine server and client tasks in one invocation: settings include one loader at a time. Distribution jars are under each loader's `build/libs`; install the remapped jar, not the development jar. Existing archive names/identities are preserved despite the new display title.

Server config remains `config/scopenet.properties`. Optional local switches are `modules.map.enabled`, `modules.economy.enabled`, `modules.vaults.enabled`, `modules.casino.enabled`, `modules.analytics.enabled`, `modules.factions.enabled` and `modules.permissions_chat.enabled`. Each accepts strict `true`/`false`; local switches can only narrow panel policy. Authentication has no module switch.

Client config remains `config/scopenet-client.json`. K opens the existing hub/settings, M opens the server map and V runs `/vault 1`. Module visibility, map layers, local waypoints and minimap anchor/offset/compact/scale settings persist there. Map tiles come from the server's panel feed. Seven-row vaults register `scopenet:vault_7rows` on both sides and require the Core client.

Build success proves compilation and the common policy tests, not Minecraft gameplay acceptance. Required staging checks and the complete implemented/pending feature matrix are in [docs/VELORA_CORE.md](../../docs/VELORA_CORE.md). Preserve each loader's licenses/attribution and independent release lifecycle.

For a staging run, use the [test-build checklist](../../docs/VELORA_CORE_TESTING.md). K now opens native Casino, Darknet and Faction screens as well as Market, Map and Vault 1. Physical shops use a vanilla slab display pedestal and a linked chest opening protected stock storage; see the checklist for setup and purchase commands.

An isolated mixin audit can be run without loading an existing world:

```powershell
.\toolchains\fabric-1.20.1\gradlew.bat -p . '-Ploader=fabric' '-PmcVersion=1.20.1' '-PverifyMixins' '-PverificationRunDir=build/velora-mixin-audit' :fabric:runServer
```

The audit forces target-class transformation and exits at Minecraft's EULA gate in a new directory. It does not accept the EULA, load player data or validate real in-world interactions. Install the Fabric LuckPerms mod to exercise the panel permission manager; its API is compile-only and is not bundled into Core.
