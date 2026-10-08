# Ranks, guild roles, chat and warps

## Level ranks

In the panel's **Leveling** page, create a **Player Title / Prefix** reward at each global level milestone. A title can name a LuckPerms group and a Discord role ID. The highest title the player has earned is shown in Paper chat and, when LuckPerms is installed, its configured group is assigned automatically. SCOPENET only changes LuckPerms groups explicitly named on title rewards. Existing panel group to LuckPerms mappings still work independently.

In **Settings → Discord community**, enable role sync and optionally set a base role ID. Linked players receive the Discord role for their highest earned title; players without a mapped title receive the base role. When a higher title replaces a lower one, the old managed title role is removed. Players without a linked Discord account cannot receive a Discord role; in-game chat shows the base `Novice` title until they earn a title.

Paper chat shows `[Guild tag] [Level title] [LuckPerms group display] nickname: message`. LuckPerms prefixes and suffixes decorate the name. Messages support `**bold**`, `*italic*`, `__underline__`, `~~strikethrough~~`, backtick code, and Bukkit `&` formatting. `&#RRGGBB` colors and `&` codes require `scopenet.chat.color`, which is granted to operators by default. Players set or clear a persistent nickname with `/nick <name|off>`; chat also respects display names set by other plugins.

## Guilds

Players can request to join a guild from the launcher directory. Leaders and officers review those requests from the guild roster. Leaders can edit the guild description, message of the day, icon and banner from **Guild settings**. They can create and assign custom roles with permissions for invites, kicks, claims, posts and guild details. Deleting a role returns its members to `member`.

**Land rules.** Leaders, officers and roles that can manage the guild decide what happens on the guild's land: whether visitors can build, use doors and buttons, open chests or walk in, and whether PVP, mob spawning, mob griefing, explosions, fire and flowing fluids are allowed. Members are never held back by the visitor rules, and everything defaults to how guild land has always behaved. Change them in the **Land** tab (player panel) or **Territory** tab (launcher), or in game with `/guild flags` and `/guild flags <rule> <on|off>`. Server admins pick which rules guilds may change on the panel's Admin claims page. The game enforces them from the same claim index it already uses.

The in-app territory view draws on the server's SCOPENET Map tiles (see [map.md](map.md)). It centers on the player's last position while they are in game. Left click or drag to claim; right click or drag to unclaim. On Paper, `/autoclaim on` claims each new unclaimed chunk entered while walking; `/autoclaim off` stops it. Auto-claim stops on a failed claim so it does not repeatedly send requests at the claim limit or in another guild's territory.

### Managing a guild in game

Everything in the launcher's Guild page works from `/guild` too, with the same rules: `list`, `info <name>`, `join <name>`, `requests`, `approve`/`reject <player>`, `kick`, `promote`/`demote`, `roles`, `role <player> <role>`, `transfer <player> confirm`, `motd`, `desc`, `post <title> | <text>` and `posts`. Changes show up in the launcher straight away and the other way round.

### Auctions and the market

`/market sell` (with no price) opens a window: choose **Buy Now** or **Auction**, set the price with the +/- buttons, pick how long an auction runs, and confirm. Players bid by clicking a listing (left click = minimum bid, right click = +10%) or with `/market bid <#id> [amount]`. A bid is held from the bidder's balance and refunded when they are outbid; a bid in the last two minutes extends the clock. Winners and unsold items wait in the **mailbox** (`/market claim`) and players are told on join. **In the launcher** the Market & Auctions page shows everything for sale (search, filter, sort, live countdowns), lets you buy and bid, shows your own listings and auctions you're winning, and lists what sold. Anything you buy, win or cancel is delivered to your **vault** by the game server within about 30 seconds — the player doesn't need to be online (if the vault is full the item waits for `/market claim` and you get a notification). Admins can place a **shopkeeper** — a villager or any mob with its AI removed — with `/shopkeeper spawn <name>`; right-clicking opens the market and the map shows a pin.

## Warps and item sharing on Paper

Players use `/reqwarp <name>` at the requested location. An operator or player with `scopenet.command.warp.manage` uses `/warprequests`, then `/warpapprove <name>` or `/warpdeny <name>`. Admins can create and remove warps directly with `/setwarp <name>` and `/delwarp <name>`. Pending requests are stored in `plugins/SCOPENET/warp_requests.yml`; approved warps are stored in `warps.yml`.

`/hand` sends the held item as a clickable chat entry. Other players can inspect a read-only snapshot for five minutes. The launcher **Command Guide** lists these commands and their permission nodes.
