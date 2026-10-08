# Command guide

Every command players and staff can type in game. This page is generated from `shared/commands/index.ts`, the same list the launcher's **Command guide**, the player panel's **Commands** page and the **Ctrl+K** palettes use, so they never disagree. Regenerate it with `node --experimental-strip-types scripts/gen-commands-doc.mjs`.

Permission nodes are LuckPerms-style. Servers using LuckPerms can grant each node to any group; without it, the defaults in the Paper `plugin.yml` (and the Fabric/Forge defaults) apply.

## Getting around

Teleports, homes and warps.

| Command | What it does | Permission |
|---|---|---|
| `/spawn` <br><sub>aliases: `/hub`, `/lobby`</sub> | Teleport to the server spawn. | `scopenet.command.spawn` |
| `/home [name]` <br><sub>aliases: `/homes`</sub> | Teleport to a home, or open the homes menu. | `scopenet.command.home` |
| `/sethome [name]` | Save your current spot as a home. | `scopenet.command.sethome` |
| `/delhome <name>` <br><sub>aliases: `/rmhome`</sub> | Delete one of your homes. | `scopenet.command.delhome` |
| `/back` <br><sub>aliases: `/return`</sub> | Return to where you were, or where you died. | `scopenet.command.back` |
| `/warp [name]` <br><sub>aliases: `/warps`</sub> | Go to a server warp, or open the warps menu. | `scopenet.command.warp` |
| `/reqwarp <name>` | Request a public warp at your current location. | `scopenet.command.reqwarp` |
| `/warprequests` | Staff: list pending warp requests. | `scopenet.command.warp.manage` |
| `/warpapprove <name> · /warpdeny <name>` | Staff: approve or deny a warp request. | `scopenet.command.warp.manage` |
| `/setwarp <name> · /delwarp <name>` | Staff: create or delete public warps. | `scopenet.command.warp.manage` |
| `/rtp` <br><sub>aliases: `/wild`, `/randomtp`</sub> | Jump to a random safe spot in the wild. | `scopenet.command.rtp` |
| `/tpa <player>` | Ask to teleport to another player. | `scopenet.command.tpa` |
| `/tpaccept` <br><sub>aliases: `/tpyes`</sub> | Accept a teleport request. | `scopenet.command.tpaccept` |
| `/tpdeny` <br><sub>aliases: `/tpno`</sub> | Decline a teleport request. | `scopenet.command.tpdeny` |

## Money & trading

Your balance, the shop, the market and player trades.

| Command | What it does | Permission |
|---|---|---|
| `/balance` <br><sub>aliases: `/bal`, `/money`</sub> | Check your balance. | `scopenet.command.balance` |
| `/pay <player> <amount>` | Send money to a player. | `scopenet.command.pay` |
| `/baltop` <br><sub>aliases: `/richest`</sub> | See the richest players. | `scopenet.command.baltop` |
| `/shop` | Open the server shop. | `scopenet.command.shop` |
| `/sell hand` | Sell the item in your hand to the shop. | `scopenet.command.sell.hand` |
| `/sell` | Open the selling menu. | `scopenet.command.sell` |
| `/market` <br><sub>aliases: `/ah`, `/auction`</sub> | Browse and buy from the player market. | `scopenet.command.market` |
| `/market sell [price]` | Sell the item in your hand for a fixed price. Without a price, a window lets you pick Buy Now or Auction with buttons. | `scopenet.command.market.sell` |
| `/market auction [starting bid] [hours]` | Start an auction for the item in your hand (1–168 hours, 24 by default). Highest bid wins; outbid players are refunded. | `scopenet.command.market.sell` |
| `/market bid <#id> [amount]` | Bid on an auction. Leave the amount out to bid the minimum. A late bid adds two minutes. | `scopenet.command.market.buy` |
| `/market claim` <br><sub>aliases: `/market mailbox`</sub> | Collect auctions you won and items that came back unsold. | `scopenet.command.market.buy` |
| `/market cancel <#id>` | Take your own listing down (auctions only before the first bid). | `scopenet.command.market.sell` |
| `/market point add <name> [market\|shop]` | Staff: turn the block you look at into a shop that opens the market or shop when right-clicked. Also shown on the map. | `scopenet.command.market.point` |
| `/orders` (`/buyorder`, `/bo`) | The buy order board. `/orders request <amount> <total price> [item]` asks for items (the money is held in escrow), `/orders pickup <id>` reserves one, `/orders fill <id>` hands the items in from your inventory, `/orders drop <id>` gives a reservation up and `/orders cancel <id>` takes your own order down. See [Buy orders and contracts](orders-and-contracts.md). | `scopenet.command.orders` |
| `/contracts` (`/contract`) | Your generated contracts. `/contracts submit [id]` hands in the items resource contracts need, `/contracts abandon <id>` swaps one for a new one. | `scopenet.command.contracts` |
| `/trade <player>` | Start a secure trade with another player. | `scopenet.command.trade` |
| `/transactions` | See your recent money movements. | `scopenet.command.transactions` |

## Guilds & land

Team up, claim land and protect it.

| Command | What it does | Permission |
|---|---|---|
| `/guild` <br><sub>aliases: `/g`, `/clan`</sub> | Open your guild dashboard. | `scopenet.command.guild` |
| `/guild create <name> <tag>` | Found a new guild. | `scopenet.command.guild.create` |
| `/guild leave` | Leave your guild. | `scopenet.command.guild.leave` |
| `/guild members` | List your guild members. | `scopenet.command.guild.members` |
| `/guild chat <message>` <br><sub>aliases: `/guild c`</sub> | Talk to your guild only. | `scopenet.command.guild.chat` |
| `/guild sethome · /guild home` | Set or visit the guild home. | `scopenet.command.guild.sethome` |
| `/guild map` | Show nearby claims on a map. | `scopenet.command.guild.map` |
| `/guild flags [<rule> <on\|off>]` | See your guild's land rules, or switch one (leaders, officers and roles that can manage the guild). Same rules as the Land tab in the player panel and launcher. | `scopenet.command.guild.flags` |
| `/guild invite <player>` | Invite a player to your guild (leaders and officers). | `scopenet.command.guild.invite` |
| `/guild list · /guild info <name>` | Browse the guilds on this server. | `scopenet.command.guild` |
| `/guild join <name> [message]` | Ask to join a guild. Its leaders get a notification. | `scopenet.command.guild` |
| `/guild requests · approve <player> · reject <player>` | Review join requests (leaders and officers). | `scopenet.command.guild` |
| `/guild kick <player>` | Remove a member. Only the leader can remove officers. | `scopenet.command.guild` |
| `/guild promote <player> · demote <player>` | Make someone an officer, or a member again (leader). | `scopenet.command.guild` |
| `/guild roles · role <player> <role>` | See the roles and give a custom role (leader). | `scopenet.command.guild` |
| `/guild transfer <player> confirm` | Hand the guild to someone else; you become an officer. | `scopenet.command.guild` |
| `/guild motd <text> · /guild desc <text>` | Set the message of the day or the description. | `scopenet.command.guild` |
| `/guild post <title> \| <text> · /guild posts` | Post an announcement everyone in the guild is notified about, or read the board. | `scopenet.command.guild` |
| `/guild accept [tag] · /guild decline [tag]` | Answer a guild invitation. You can also answer in the launcher. | `scopenet.command.guild.accept` |
| `/claim` | Claim the chunk you stand in for your guild. | `scopenet.command.claim` |
| `/unclaim` | Give up the chunk you stand in. | `scopenet.command.unclaim` |
| `/autoclaim on\|off` | Automatically claim new chunks as you walk. | `scopenet.command.claim` |

## Guild bank

Shared guild money. Officers and leaders can spend it.

| Command | What it does | Permission |
|---|---|---|
| `/guild bank` | Show the bank balance and recent activity. | `scopenet.command.guild.bank` |
| `/guild bank deposit <amount>` | Put your own money into the bank. | `scopenet.command.guild.bank.deposit` |
| `/guild bank withdraw <amount>` | Take money out (officers and leaders). | `scopenet.command.guild.bank.withdraw` |
| `/guild sell hand` | Sell your held item and pay the bank. | `scopenet.command.guild.sell` |
| `/guild market` | Buy and sell on the market as your guild. | `scopenet.command.guild.market` |
| `/guild pay <tag> <amount>` | Pay another guild from your bank. | `scopenet.command.guild.pay` |

## Everyday perks

Handy commands you can configure per server. Ask an admin if one is off.

| Command | What it does | Permission |
|---|---|---|
| `/fly [on\|off]` | Fly freely. free.fly works anywhere; guild.fly only inside your own guild’s land (it switches off when you leave). | `free.fly or guild.fly` |
| `/heal [player]` | Restore health (the amount and wait are set by the server). | `scopenet.command.heal` |
| `/feed [player]` | Restore hunger (the wait is set by the server). | `scopenet.command.feed` |
| `/vault [number]` <br><sub>aliases: `/pv`</sub> | Open a portable chest. Some vaults need a higher rank. | `scopenet.command.vault` |
| `/echest` <br><sub>aliases: `/ec`, `/enderchest`</sub> | Open your ender chest from anywhere. | `scopenet.command.echest` |
| `/kit [name]` <br><sub>aliases: `/kits`</sub> | List the kits you can claim, or claim one. | `scopenet.command.kit` |
| `/quests [player]` <br><sub>aliases: `/quest`, `/dailies`</sub> | See your daily and weekly quests, or anyone else’s. | `scopenet.command.quests` |
| `/cosmetic [list [type]] · equip <name> · unequip <name\|type\|all>` <br><sub>aliases: `/cosmetics`, `/wardrobe`</sub> | See your unlocked cosmetics, and put them on or take them off. | `scopenet.command.cosmetic` |

## Info & admin

Help, the player panel link and staff tools.

| Command | What it does | Permission |
|---|---|---|
| `/shopkeeper spawn <name> [market\|shop] [mob]` | Staff: place a villager (or any mob) that opens the market when right-clicked. It never moves and is marked on the map. | `scopenet.command.market.point` |
| `/shopkeeper link <name> · remove <name> · list` | Staff: turn the mob you look at into a shopkeeper, free it again, or list them. | `scopenet.command.market.point` |
| `/adminclaim create <name> [description]` <br><sub>aliases: `/aclaim`</sub> | Staff: protect the chunk you stand in as server land with a name and description. Also manage them in the admin panel. | `scopenet.admin.claims` |
| `/customitem give <player> <id>` | Staff: hand out an item designed in the admin panel. | `scopenet.admin.customitem` |
| `/playtime` <br><sub>aliases: `/ontime`</sub> | Check your total and session playtime. | `scopenet.command.playtime` |
| `/hand` | Share your held item as an inspectable chat link. | `scopenet.command.hand` |
| `/nick <name\|off>` | Set or remove your chat nickname. | `scopenet.command.nick` |
| `/scopenet help` <br><sub>aliases: `/sn`</sub> | List the SCOPENET commands you can use. | `scopenet.command.scopenet.help` |
| `/scopenet panel` <br><sub>aliases: `/scopenet web`, `/scopenet app`</sub> | Get the link to the player panel: market, casino, friends, guilds and quests in any browser or phone. Sign in with your launcher account. | `scopenet.command.scopenet.panel` |
| `/scopenet status` | Panel connection, features and plugin integrations (admins). | `scopenet.command.scopenet.status` |
| `/scopenet reload` | Reload the plugin config (admins). | `scopenet.command.scopenet.reload` |

## Good to know

- Press Ctrl+K in the launcher, the player panel or the admin panel to jump anywhere or find a command instantly.
- The player panel works on your phone: market, casino, friends, guilds, quests, leaderboards and the live map. Type /scopenet panel in game for the link, then use “Add to Home Screen”.
- Placeholders: use %scopenet_level%, %scopenet_guild%, %scopenet_balance% and more in scoreboards, tab and chat.
- Walking into claimed land shows the guild (or the server area and its description) above your hotbar. Your bell in the title bar collects guild, auction and quest news.
- Right-click a shop point (a marked block) to open the market or shop. On a server with the SCOPENET client mod it opens the shop window.
- Some commands may be switched off or limited by the server. If one says you lack permission, ask an admin.
- Everything here also works through your rank: servers using LuckPerms can grant each permission node to any group.
