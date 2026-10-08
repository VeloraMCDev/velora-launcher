# Rewards, limits and permission nodes

## Rewards beyond XP

Quests, achievements and rank milestones (level rewards) can hand out more than XP and a title. In the admin panel, open any quest, achievement or milestone and use **Extra rewards**:

| Reward | What happens | Where it happens |
|---|---|---|
| **Money** | Credits the balance, once per economy group (or one chosen server) | Panel, instantly |
| **Claim chunks** | Raises the land limit of the player's guild while they are a member | Panel, instantly |
| **Badge** | Shows a badge on their profile | Panel, instantly |
| **Item** | Puts the item in their inventory (anything that doesn't fit drops at their feet). Vanilla ids such as `diamond` or `modid:item` | Game server, when they are online |
| **Permission** | Grants a permission node, permanently or for a number of days | Game server (LuckPerms by default) |
| **Group** | Adds them to a permission group | Game server (LuckPerms by default) |
| **Message** | Sends a chat message | Game server |
| **Command** | Runs a console command with `{player}` and `{uuid}` filled in | Game server, only if the server allows it |

Players see short summaries ("$250.00", "Diamond x3", "+4 claim chunks") next to each quest and achievement in the launcher. Commands are shown only as "A special reward".

Items, permissions, groups and messages are queued in the panel and handed to the game server once the player is online there. Anything that fails (an unknown item, LuckPerms missing) is retried a few times and then kept in the delivery log with the reason. A reward earned on one server can be delivered by whichever server the player is on, unless you pin it to a server.

### Server settings

Paper `config.yml` / Fabric `scopenet.properties`:

```yaml
rewards:
  enabled: true
  allow-commands: false      # let the panel run its own console commands. Leave off unless every panel admin is trusted.
  permission-command: "lp user {player} permission set {node} true"
  permission-temp-command: "lp user {player} permission settemp {node} {value} {duration}"
  group-command: "lp user {player} parent add {group}"
```

Using another permissions plugin? Change the command templates. Placeholders: `{player}`, `{uuid}`, `{node}`, `{value}`, `{duration}`, `{group}`.

## What you can configure

### Per server (plugin/mod config)

| Setting | Meaning |
|---|---|
| `essentials.max-homes` | Homes every player can set |
| `essentials.home-name-max-length` | Longest home name |
| `essentials.max-warps` | Most warps the server may have (0 = no limit) |
| `essentials.tpa-timeout-seconds` | How long a `/tpa` request stays open |
| `essentials.rtp-radius`, `rtp-min-radius`, `rtp-attempts` | Where `/rtp` sends players, and how hard it tries |
| `essentials.cooldowns.<command>` | Seconds between uses of `spawn`, `home`, `back`, `tpa`, `rtp`, `warp` |
| `map.enabled`, `map.show-homes` | The SCOPENET Map and whether homes appear on it |
| `economy.currency-symbol` | The symbol shown next to money |
| `rewards.*` | See above |
| Feature switches | `leveling`, `quests`, `achievements`, `guilds`, `social`, `essentials`, `economy` |

### Network-wide (admin panel → Progression → Economy & guild rules)

Starting balance, chunks a new guild may claim, extra claims per member and per guild level, guild member limit, and market listings per player. These apply on every server and take effect immediately.

## Permission nodes that change limits

Give these with LuckPerms (or any permissions plugin) to ranks:

| Node | Effect |
|---|---|
| `scopenet.homes.<number>` | Homes limit for the player, e.g. `scopenet.homes.10`. The highest one a player has wins and never lowers the base `max-homes` |
| `scopenet.homes.unlimited` | No home limit |
| `scopenet.cooldown.bypass` | Skip every command cooldown |

Operators and anyone with `*` get the top tier on Paper. On Fabric these nodes are off for everyone until a permissions system (such as the LuckPerms mod) grants them; operators still skip cooldowns.

Claim chunks are a guild limit, so they are changed with the network rules above or with **Claim chunks** rewards, not with a permission node.
