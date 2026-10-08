# Example configuration

Copy the file for your setup and fill in `panel-url` and `token` (create the token under **Servers** in the admin panel).

| Folder | Goes in | For |
|---|---|---|
| `paper/config.yml` | `plugins/SCOPENET/config.yml` | The Paper plugin. Includes the `integrations:` section for LuckPerms, PlaceholderAPI, Vault, CoreProtect, WorldGuard and Spark. |
| `fabric-server/scopenet.properties` | `config/scopenet.properties` | The Fabric server mod (Fabric 1.20.1, 1.21.1). |
| `fabric-client/scopenet-client.json` | `config/scopenet-client.json` | The optional Fabric client mod: HUD widgets, notifications and keybinds. |

The client mod is display-only. The server mod or plugin stays in charge of claims, money and permissions, so a missing or edited client mod can't change anything.
