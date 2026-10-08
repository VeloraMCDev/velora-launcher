# Buy orders and contracts

Two more ways for players to earn and trade, both paid in the SCOPENET economy. Players use them in game (`/orders`, `/contracts`) and on the **Market** page of the launcher and the player panel (the **Buy orders** and **Contracts** tabs). Admins configure them under **Economy → Orders & contracts**.

## Buy orders

A buy order is a request: *"I want 128× Iron Ingot and I'll pay $900 for all of it."*

1. **Ask.** The buyer posts the order (`/orders request 128 900 IRON_INGOT`, or from the launcher). The full price is taken from their balance **at once and held in escrow**.
2. **Pick up.** Anyone else checks the board (`/orders`, or the launcher) and picks an order up. That **reserves it for them** for a while (60 minutes by default) so nobody can snipe it while they gather the items. They can give the reservation up, and it lapses by itself.
3. **Hand in.** In game, `/orders fill <id>` takes exactly the items the order needs out of their inventory. The items go to the **buyer's vault** (even if the buyer is offline) and the escrow is **released to whoever filled it**, all in one step. Anything extra they carried is handed straight back.

Details that keep it safe:

* The items are taken and written to disk before the panel is asked, so a crash or an outage never loses them; if the panel refuses (the order was cancelled, someone else filled it) they are returned.
* An order can only ever be filled once and the buyer is never charged twice. Replaying the same request changes nothing.
* The buyer can cancel any time before it is filled and gets the money back. Orders nobody fills come down after 7 days and are refunded. Admins can remove any order.
* Only plain items count (no enchantments, custom names or other data), and you can't fill your own order.
* An optional house fee can be taken from the filler's payment.

Limits (all in the admin page): open orders per player, items and price per order, expiry, reservation time, orders one player can pick up at once, and the fee.

## Contracts

Every player has their own **board of randomly generated contracts**, for example *Kill 50 Skeletons* or *Submit 256× Cobblestone*. Pay scales with **how hard the task is**: every kill target and resource has a difficulty from 1 (Zombies, Cobblestone) to 5 (Wither Skeletons, Diamonds) that sets the rate, bigger asks pay a slightly better rate, and now and then a **hot contract** pays 50% more. Every payout is a flat number ending in **5 or 0** (never under $5). The board shows the difficulty, and the multiplier in the settings scales it all.

* **Kill contracts** count by themselves as the game server reports mob kills (Paper and Fabric servers both report them). They pay out the moment they complete.
* **Resource contracts need the real items.** Gather them and run `/contracts submit` (or `/contracts submit <id>`): the items are taken from your inventory and **used up**, so a contract is a genuine sink for the resource. Only the amount still needed is taken; any extra is handed back. Progress never moves without items.
* Contracts expire (24 hours by default). Finishing one makes room for a new one, up to a **daily limit** per player. A few **swaps** a day let a player throw away a contract they don't like.
* Each board never repeats a target, and every contract's size and payout is rolled at random within sensible ranges.

Admins choose the board size, expiry, payout multiplier, daily limit, swaps, and whether kill and resource contracts are on.

## Commands

| Command | What it does |
| --- | --- |
| `/orders` | Show the board. |
| `/orders request <amount> <total price> [item]` | Ask for items (hold the item, or name it like `DIAMOND`). |
| `/orders pickup <id>` · `drop <id>` | Reserve an order / give the reservation up. |
| `/orders fill <id>` | Hand the items in and get paid. |
| `/orders cancel <id>` | Take your own order down. |
| `/contracts` | Show your contracts. |
| `/contracts submit [id]` | Hand in items for resource contracts. |
| `/contracts abandon <id>` | Swap a contract for a new one. |
