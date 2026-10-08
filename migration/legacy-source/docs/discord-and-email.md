# Discord messages, live embeds and emails

## Announcements (Admin panel → Discord → Announcements)

## Set up the bot (once)

The panel posts as a Discord **bot**, so you only ever enter a **channel ID**; there are no webhooks to create.

1. In the [Discord Developer Portal](https://discord.com/developers/applications) create an application, open **Bot**, and copy its token.
2. In the panel open **Settings → Connections** and paste the application's **client ID**, the **bot token** and your **Discord server ID**. Save.
3. Under **Settings → Discord** press **Add the bot to your server** and approve it. The invite asks for exactly what is needed: view channels, send messages, embed links, read history, and manage roles (for role sync).
4. Choose the **announcement channel**: press **Pick from server** and select it, or paste its ID (Discord → Settings → Advanced → Developer Mode, then right click the channel → **Copy Channel ID**). Press **Send test announcement**.

If Discord refuses, the panel says why in plain words (missing permission, wrong channel ID, bad token). Older setups that use a webhook address keep working until you set a channel.

Three messages are posted in that channel: **Achievement unlocked**, **New player** and **New guild**. Each one is fully editable, with a live preview built the way Discord will show it:

- Message text above the box (mentions such as `<@&roleid>` work there), title and link, description, colour, a small picture on the right, a big picture at the bottom, author and footer (with icons), the time, and any number of side-by-side or full-width fields.
- The sender's name and picture (only for older webhook setups: the bot always posts under its own name).
- **Placeholders** such as `{player}`, `{avatar}`, `{achievement}`, `{achievement_description}`, `{xp}`, `{level}`, `{rank_title}`, `{guild_line}` (achievements), `{members_total}` (new players), `{guild}`, `{leader}`, `{members}`, `{claims}` (guilds) and `{server_name}`, `{time}` everywhere. Click a placeholder to insert it where you are typing. Unknown ones stay as typed so mistakes are easy to spot.
- Player names are cleaned so they can't ping anyone or break the formatting.

Switch each message on or off, and press **Save & send a sample** to see it in your channel.

## Live embeds (Discord → Live embeds)

Four boards that the panel posts once and then **edits in place**, so the channel never fills up:

| Embed | Shows | Rows are set by |
|---|---|---|
| **Server status** | Every server: online, players, TPS, version | the line template |
| **Active guilds** | Top guilds with level, members, land and leader | rows shown |
| **Player leaderboard** | Top players by playtime, level, kills or blocks | ranked by, rows shown |
| **Richest players** | Top balances of one server economy | economy of, rows shown |

Every embed has the same editor as the announcements, plus a **line for each row** (for example `{rank}. **{player}** — {value}`) that fills `{rows}` in the description, an update interval (1 minute to 1 hour), and an optional **own channel** so each board can live somewhere else. If someone deletes the message, the next update posts a fresh one; **Post as new message** forces it.

## Slash commands (`/status`, `/quests`, …)

Members can type these in your Discord server — nothing else needs to run, because Discord calls the panel each time:

| Command | Shows |
| --- | --- |
| `/status` | Every game server: online or not, players, TPS, version |
| `/players` | Who is online, per server |
| `/quests <player>` | A player's daily and weekly quests with progress bars and reset times |
| `/stats <player>` | Level, playtime, kills, achievements and guild |
| `/leaderboard [by]` | Top players by level, playtime, kills, blocks or balance |
| `/guild <name>` / `/guilds` | A guild's details, or the biggest guilds |

Setup (once), under **Admin panel → Settings → Discord slash commands**:

1. In the Discord Developer Portal open your application → **General Information** and copy the **Public Key** into the panel.
2. On that same page set **Interactions Endpoint URL** to `<your panel address>/api/v1/discord/interactions` and save (Discord sends a test request, which the panel answers once the public key is saved).
3. Make sure the bot was invited with the `applications.commands` scope (the invite link in Settings includes it), then press **Save & register commands**.

Every request is verified against the public key; anything unsigned is refused. Player and guild names autocomplete as people type.

## Emails (Admin panel → Emails)

Set up the sender and Resend key under **Settings → Connections** first. Then:

1. Write a **template**: subject and message with placeholders (`{player}`, `{level}`, `{rank_title}`, `{guild}`, `{brand}`, `{panel_url}`…). Simple formatting turns into a branded HTML email in your launcher colours: `# Heading`, `**bold**`, `- list`, `[link](https://…)`, `[button: Label](https://…)`.
2. Check the live **preview**, and **send a test** to yourself.
3. **Send** to everyone with an email, a group, or chosen players. The panel shows how many will receive it first, sends them one by one in the background, and keeps a log with failures and their reasons.

Every email has an unsubscribe link; players who use it are skipped by later emails. Mark a message as an **account notice** to reach everyone regardless, without the unsubscribe link: use that only for things about their account.

## Live map on the landing page

In **Landing page**, add a **Live map** section. Choose the server (or leave it at the first server with a map), the height, and what visitors may see: guild land and places are on by default, **live player positions are off** (they let strangers find players). Visitors can pan and zoom the same map your players see in the launcher; removing the section closes the public map again.
