# Velora Core setup guide

Velora Core is the Fabric 1.20.1 integration for Velora SMP. It has two jars:

| Jar | Install on | Purpose |
| --- | --- | --- |
| `velora-core-server-1.20.1-<version>.jar` (**Velora Core Server**) | The Minecraft server | Connects the server to the panel: sign-in, economy, vaults, factions, casino, map and permissions. |
| `velora-core-client-1.20.1-<version>.jar` (**Velora Core Client**) | Each player's game | In-game hub (K), map (M), seven-row vaults and HUD. Optional for basic play, required for seven-row vaults. |

Both need Fabric Loader 0.16.10 or newer, Fabric API and Java 17 or newer. Install the remapped jar from `build/libs`, not the development jar.

## Using Calagopus

If you run your servers on Calagopus, the Velora Core extension installs and updates the mod for you, writes the config and downloads authlib-injector. See [VELORA_CORE_DEPLOYMENT.md](VELORA_CORE_DEPLOYMENT.md). The steps below are what it automates.

## Upgrading from the SCOPENET names

Earlier test builds used `scopenet` names. Replace both jars with the Velora Core ones, remove the old server jar from `mods`, and start the server. It copies `config/scopenet.properties` to `config/velora-core.properties` (the old file is left in place), moves `config/scopenet` to `config/velora-core`, and keeps honouring LuckPerms nodes named `scopenet.*`. New nodes are `velora.*`. `/scopenet` still works as an alias for `/velora`. Back up `config` first, and update the server and every client together, because the network channel is now `velora_core:s2c`/`c2s`.

## Server setup

1. **Create the server in the panel.** Servers → New server. Copy the token (`sn_` followed by 40 characters). It is shown once.
2. **Install the mod.** Put the server jar and Fabric API in the server's `mods` folder.
3. **Start the server once.** Velora Core writes `config/velora-core.properties`, with every option explained. It stops with a message naming that file until the token is set.
4. **Set the two required lines:**

   ```properties
   panel-url=https://your-panel.example.com
   token=sn_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
   ```

   `panel-url` must be an HTTP(S) address with no credentials, query or fragment.
5. **Enable the panel's sign-in.** Download `authlib-injector.jar` from the panel (`/api/v1/launcher/authlib-injector.jar`) into the server folder, keep `online-mode=true` in `server.properties` and add the flag before `-jar`:

   ```sh
   java -javaagent:authlib-injector.jar=https://your-panel.example.com/api/yggdrasil -jar fabric-server-launch.jar nogui
   ```

   Players then join with their Velora account through the launcher. Without this flag the server falls back to Mojang's sign-in and Velora accounts cannot join.
6. **Restart, then verify.** Join and run `/velora status`. It shows the panel address, whether the token is set, which modules are on, the claim index and the permission source.

Never put the server token in the client config or share it. After changes to `config/velora-core.properties` you can run `/velora reload` instead of restarting.

## Player setup

1. Install the Velora launcher and sign in.
2. Optional: add the Velora Core Client jar and Fabric API to the instance's `mods` folder.
3. Join the server from the launcher.

In game: **K** opens the hub (Market, Darknet, Casino, Faction, Map, Vault 1), **M** opens the map, **V** opens Vault 1. `/velora panel` prints the web panel address.

## Configuration reference

Server: `config/velora-core.properties`. Client: `config/velora-core-client.json`, created on first launch of the client.

| Key | Default | Meaning |
| --- | --- | --- |
| `panel-url`, `token` | none | Required. Panel address and the server token. |
| `modules.<id>.enabled` | `true` | Narrows the panel policy for `map`, `economy`, `vaults`, `casino`, `analytics`, `factions`, `permissions_chat`. Never overrides a panel switch. Authentication has no switch. |
| `map.enabled` | `true` | Upload this world's map tiles, players and claims to the panel. |
| `permissions.default_level` | `all` | `all` lets everyone use Velora commands unless LuckPerms denies them; `op` restricts to operators. |
| `clientlink.enabled` | `true` | Let Velora Core Client connect to the in-game hub and map. |
| `economy.currency_symbol` | `$` | Symbol shown in chat. |
| `essentials.*` | see file | Home, warp, TPA and random-teleport limits and cooldowns. |
| `rewards.*` | see file | Reward delivery; `rewards.allow_commands` lets the panel run console commands, leave off unless every panel admin is trusted. |
| `integrations.luckperms.apply-panel-groups` | `false` | Map panel ranks to LuckPerms groups. Needs the Fabric LuckPerms mod. |

Older keys such as `leveling.*`, `quests.enabled` and `guilds.*` are still read for compatibility.

## Commands

| Command | Purpose |
| --- | --- |
| `/velora status` | Connection and module health. |
| `/velora reload` | Re-read `config/velora-core.properties`. |
| `/velora panel` | Print the player panel address. |
| `/velora map` | Map upload status. |
| `/velora help` | Commands available to you. |
| `/vault <n>`, `/darknet`, `/pshop ...` | Vaults, Darknet and pedestal shops. See [VELORA_CORE.md](VELORA_CORE.md). |

## Troubleshooting

- **"Velora cannot start: Set token in config/velora-core.properties"**: the token line is empty or malformed. Copy it again from the panel; a lost token needs a new one from the Servers page.
- **No `config/velora-core.properties`**: the server must run from the folder containing `config`, and the server jar must be in `mods`. The file appears on the first start even if the server then stops.
- **Players are rejected at login**: confirm the `-javaagent` flag, that `online-mode=true` and that the server can reach the panel.
- **Nothing happens on K**: the Client jar is missing, or `clientlink.enabled=false`, or the module is turned off in the panel.

Remaining gaps and staging checks: [VELORA_CORE.md](VELORA_CORE.md) and [VELORA_CORE_TESTING.md](VELORA_CORE_TESTING.md).
