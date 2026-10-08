# Plugin integrations, developer API and network features

## Optional plugins (Paper)

SCOPENET finds these plugins by itself at startup. Nothing is hard-wired into `ScopenetPlugin`; each integration is a module in `net.scopenet.paper.compat` that is only loaded when its plugin is present. Switch any off in `config.yml` under `integrations:`. `/scopenet status` lists what is active.

| Plugin | What SCOPENET does |
|---|---|
| LuckPerms | Reports each online player's rank to the panel (shown read-only on profiles and the server page). Panel groups can be mapped to LuckPerms groups (Players → Groups). LuckPerms always decides real permissions. Sync modes: off, game→panel, panel→game, both (adds only, never removes; panel→game also needs `apply-panel-groups: true`). |
| PlaceholderAPI | `%scopenet_level%`, `%scopenet_guild%`, `%scopenet_balance%`, `%scopenet_playtime%`, `%scopenet_quest_progress%` and more (see the expansion class for the full list). |
| Vault | SCOPENET's economy becomes Vault's economy, so other plugins pay into the same balances. |
| CoreProtect | Logged-activity rate, top actors and backup folder status appear on the server page. |
| WorldGuard | Regions (area, owners, flags) appear on the server page, including server-owned territory. |
| Spark | TPS, MSPT, CPU and memory appear on the server page. |

## Permissions

Every command has a LuckPerms-friendly node, `scopenet.command.<command>[.<subcommand>]` (for example `scopenet.command.guild.bank.withdraw`). Wildcards `scopenet.command.*` and `scopenet.command.guild.*` work. `scopenet.claims.bypass` lets staff build in other guilds' claims. All nodes are declared in `plugin.yml`.

## Developer API

Depend on the `integrations/api` module (`net.scopenet.api`). Get the API from Bukkit's services manager or `ScopenetApiProvider.get()`:

```java
ScopenetApi api = ScopenetApiProvider.get();
api.getPlayer(uuid).thenAccept(p -> ...);
api.getGuild(uuid);  api.getBalance(uuid);  api.getFriends(uuid);
api.addXP(uuid, 50, "my-plugin");
api.completeQuestObjective(uuid, "my_action", 1);
```

Bukkit events: `ScopenetLevelUpEvent`, `GuildJoinEvent`, `AchievementUnlockEvent`. They are raised on the main thread when the panel reports them.

## Discord

Players link Discord in the launcher (Settings → Connections). In panel Settings → Discord community, admins can:
- map panel groups to Discord role IDs (Players → Groups) and pick a sync direction (adds only; also runs every 15 minutes),
- set an invite link shown in the launcher,
- post announcements (achievements, new guilds, new players) and live boards with the Discord bot, in channels chosen by ID.

## Networks

Servers already track a global level plus a level per server; friends, guilds and achievements are account-wide. To share money, give servers the same **Shared economy group** name in the server's settings: balances and guild banks are then one pool. Existing per-server balances are not merged when a server joins a group.

## Deleting accounts

Deleting a player in the panel removes their levels, quests, achievements, stats, friends, messages, posts, profile, economy, guild membership (leadership passes on, empty guilds are dissolved), sessions and skin file. Other players' records that mention them (transactions, bank logs) are anonymised. The name stays reserved so nobody can impersonate them. Plugin-local data such as homes lives on the game server and is not reachable from the panel.
