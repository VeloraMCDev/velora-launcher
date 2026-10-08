# The player panel

The player panel is the website version of the launcher's everyday features, built for phones first. It is the **on-the-go grind**: check the market, bid on an auction, spin the casino, chat with friends, claim quests or move guild money from the bus, without opening the game or a PC.

It lives on the same address as the admin panel, at `https://your-panel/#/play`. Players sign in with the **same account** they use in the launcher; admins can use it too (the admin sidebar has a **Player panel** link, and the player panel has an **Admin panel** link for admins).

## What is in it

| Page | What you can do |
|---|---|
| **Home** | Balance, level progress, today's quests, the free daily spin, live server status, friends online, news |
| **Market** | Browse buy-now listings and auctions, search, filter and sort, buy, bid (with quick-bid buttons) and cancel your own listings, see what sold and what is waiting for you |
| **Casino** | Slots, Wheel, Plinko, Mines, Blackjack, Crash, Dice, Coin Flip, Double or Nothing, the free daily spin, bounties and player betting, all animated |
| **Friends** | Friends and requests, direct messages, game invites, player profiles and posts |
| **Wallet** | Balance on every server, a filterable transaction feed, the richest-players board, your guild bank |
| **Guilds** | Your guild, members and roles, join requests and invites, the guild bank, the announcement board, land claims, discover and found guilds |
| **Quests** | Daily and weekly quests with claim buttons, achievements, level rewards and your rank title |
| **Leaderboards** | Your stats, global and per-server rankings (playtime, kills, blocks, ...), the level board |
| **Live map** | The server map with claimed land, shops and who is online (when the server's map is switched on) |
| **Launcher** | Download links, the server address and the instances you can launch |
| **Commands** | Every in-game command, searchable; tap one to copy it |
| **Account** | Skin and arm style, cape, username, Discord link, install-as-app, sign out |

Pick the server in the top bar. Servers that share an **economy group** share one balance, so the number in the top bar follows you between them.

## Getting around

* **Phones** get a bottom tab bar (Home, Market, Casino, Friends, More). **More** opens the remaining pages.
* **Desktops** get a left rail with every page.
* **Ctrl+K** (or **Cmd+K**) opens a quick-search palette to jump to any page, switch server, or find a command (picking a command opens the Commands page on it).
* The bell collects guild, auction and quest news, exactly like the launcher's.

## Install it like an app

The panel is installable. In Chrome and Edge choose **Install app**; in Safari on iPhone tap **Share → Add to Home Screen**. The **Account** page walks players through it. It then opens full screen, with its own icon.

There is also a real **Android app (.apk)** and an **iOS build (.ipa)**, produced by the release workflow; see [phone apps](mobile-apps.md).

Serve the panel over **HTTPS** (see the [deployment notes](deployment.md#https-and-phones)): browsers only offer installation on secure origins, and your players' tokens should never travel in the clear.

## Sign-up and access

Players need an account on your panel. Whether people can create their own is the panel's existing **Settings → Sign-ups** switch (closed, approval or open); with sign-ups open or on approval the sign-in page shows **Create an account**. Disabled and pending accounts cannot sign in, exactly as in the launcher.

The player panel only ever calls the player-facing `/api/v1` endpoints with the player's own token. Nothing in it can reach admin functions; those stay behind the admin role on `/api/admin`.

## Branding

The player panel follows the colours and name you set in **Launcher design**, so the website and the launcher feel like one product. It reads the same branding the launcher reads.

## The casino and animations

The casino is one shared module (`shared/casino`) used by both the launcher and the player panel, so odds, limits and behaviour are identical. Its animations (spinning reels, the wheel, the Plinko ball, Mines tiles, win bursts) are driven from JavaScript rather than CSS transitions, so they keep moving even when your computer's **"reduce animations"** setting is on. They respect the app's own **Reduce motion** switch in launcher settings, which shortens them. Sound effects are off until a player taps the speaker icon on a game.
