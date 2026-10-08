# Casino

The Casino is a page in the launcher **and in the [player panel](player-panel.md) on the website** (so players can gamble from their phones) where players spend and win the in-game currency: **Slots, Wheel, Plinko, Mines, Blackjack, Crash, Dice and Coin Flip**, a **free daily spin**, **Double or Nothing** after any win, **bounties** on other players, and **bets** on what players will do. Everything is configured in the admin panel under **Casino**, takes effect immediately, and is paid from and to the SCOPENET economy, so balances stay in step with the game (and the market, kits and shops).

Players pick a server at the top of the page (servers that share an economy group share one balance). The same casino runs on every server; the odds you set apply everywhere.

## Games

| Game | How it works | What you configure |
| --- | --- | --- |
| **Slots** | Three reels, one pay line. Three matching symbols pay that symbol's multiplier; any two matching pay a smaller one. | Symbols (name, colour, chance weight, payout), pair payout, bet limits. |
| **Wheel** | One spin of a weighted wheel; the slice it lands on is the multiplier of the bet. | Slices (label, colour, multiplier, chance weight), bet limits. |
| **Plinko** | A ball falls through rows of pegs into a payout slot. Low, medium and high risk give flatter or spikier payouts. | Return to players, rows (6 to 16), risk levels on offer, bet limits. The tables are generated so the average round returns exactly the percentage you choose. |
| **Mines** | Turn over tiles on a board. Gems raise the multiplier, a mine loses the bet, and players can cash out any time. | Board size (3 to 7), mine counts, house edge, top multiplier, bet limits. |
| **Blackjack** | Beat the dealer to 21. Hit, stand or double down; blackjack pays 3 to 2. Cards come from an endless shoe, so nothing can be counted. | Blackjack payout, whether the dealer hits soft 17, bet limits. |
| **Crash** | A multiplier climbs from 1× and crashes at a random moment nobody can predict. Cash out before it does (or set an auto cash-out). | House edge, top multiplier, bet limits. Every cash-out target returns the same percentage, so there is no safe strategy to exploit. |
| **Dice** | Players choose their own win chance (roll under or over) and the payout follows: the smaller the chance, the bigger the prize. | House edge, lowest and highest win chance, bet limits. |
| **Coin Flip** | Call heads or tails on a fair 50/50 coin. | Payout (a fair coin would pay 2×), bet limits. |

Every control shows the **return to players** next to it, worked out by the server from the exact settings you typed, so you always know the house edge before you save. The defaults return about 95% (a 5% edge).

All randomness is rolled on the panel, never in the launcher, and every bet, result and payout is saved in one step: a bet is never taken without its result being recorded, and a round is never paid twice.

### Double or Nothing

After any win, the player is offered a gamble on those winnings: win and they double, lose and they're gone. Declining costs nothing, because the winnings are already in their balance, and the offer simply expires. A win can be doubled again up to a streak limit. Under **Casino → Double or Nothing** you set the chance the double comes off (default **49%**, so the house keeps a small edge), the streak limit (default 5) and how long the offer stays open (default 5 minutes). Each double is its own recorded round, and the **largest payout** and **daily loss limit** still apply.

### Chaos

Real casinos feel random because they are, not because a table says so. **Casino → Chaos** adds surprises on top of the instant games (Slots, Wheel, Plinko, Dice, Coin Flip): any winning round can be hit by a **lucky surge** (the win is multiplied by 1.5×, 2× or 3×) or a **curse** (the win is halved), and the player sees which. With the defaults (4% surge, 6% curse) the two roughly cancel, so the published return to players barely moves; the panel shows the net effect of your settings. Dice lets players pick their own odds, Crash ends at a random point, and Blackjack deals from an endless shoe, so the longer games are chance-driven too.

### Free daily spin

Each player gets the configured number of free spins per day (default **1**, reset at midnight UTC) on a prize wheel of cash amounts. The prizes are paid straight into their balance. Set the spins to 0 or switch it off in **Casino → Daily spin**.

### Safety limits (Casino → Overview)

* **Largest payout per round** caps any single win.
* **Daily loss limit** stops a player who has lost that much today until tomorrow. 0 turns it off.
* **Big-wins ticker** shows wins of at least the amount you set on the launcher's lobby.

## Bounties

A player can put money on another player's head (**Casino → Bounties** in the launcher). The money is taken immediately and held. The first player to kill the target in PvP collects **every** bounty on them. You choose:

* minimum and maximum bounty, and how many a player can have out at once,
* the **house's cut** (taken when it is placed; the rest is the reward),
* an **expiry** (uncollected bounties go back to whoever placed them),
* a **same-pair cooldown** so two friends can't farm a bounty by killing each other,
* whether bounties can be anonymous and whether placers can withdraw them.

Players cannot claim a bounty they placed themselves. Targets, placers and the killer all get a notification in the launcher's bell and in game chat. Admins can remove any bounty from the panel, which returns the reward to the placer.

Bounties are claimed from the `pvp_kill` event the **Paper** plugin sends when a player kills another player. Update the plugin on each server to turn this on. Fabric and Forge servers do not send it yet, so bounties placed on those servers are never collected (they expire and are refunded).

## Player betting

A bet asks "**will this player reach N of something before time runs out?**", for example "Steve gets 5 player kills in the next hour". Everyone puts money on **Yes** or **No**. When time is up, the winning side splits the whole pool in proportion to their stakes, minus the **house rake**. If nobody backed the other side, every stake is returned and the house takes nothing.

* **What can be bet on**: player kills, deaths, mob kills, blocks broken, blocks placed (you pick which in **Casino → Betting**).
* **Who can open a bet**: any player, or admins only.
* Results come from the stats the game servers already report (the count is read at the end of the time limit, plus up to a minute for the final stat sync). The target can't bet on themselves, nobody can bet both ways, and betting closes a few minutes before the end.
* Admins can cancel any open bet; every stake is returned.

## Leaving a game mid-round

Switching pages (or closing the tab or launcher) never costs a player their stake. In the launcher and the web panel every table stays mounted while the player looks at another tab, so an unfinished Mines board, Blackjack hand or Crash flight is exactly where they left it. If the player is gone for good, the server settles the round: Crash cashes out at the multiplier it had reached once the player has not checked in for a short while, an abandoned Mines board is cashed out at its current multiplier (or refunded when nothing was revealed), and an abandoned Blackjack hand is refunded. The casino is not part of the in-game Fabric companion; it lives in the launcher and the web panel.

## Animations and sound

Crash rises on a live graph, Blackjack deals cards one by one, the coin tumbles in the air and the dice number rolls before it settles. Double or Nothing flips its own coin.

Every game plays out visually: slot reels spin up, blur and settle one after another with a small bounce; the wheel spins up, ticks a pointer past each peg and eases onto the winning slice; the Plinko ball bounces peg by peg; Mines tiles flip and a mine shakes the board; big wins throw coins and confetti. The server has already decided the result before anything moves, so the animation only reveals it.

The animations are driven from JavaScript (not CSS transitions), so they keep running when the player's operating system has **reduce animations** switched on, which used to freeze the wheel and reels. The launcher's own **Settings → Appearance → Reduce motion** switch still shortens them. Tiny synthesised sound effects (tick, coin, boom) are available behind a speaker button on each game and stay off until the player turns them on.

## How the money moves

Casino rounds, bounties and bets appear in the economy's transaction list as "Casino: ..." entries, with the house as the other party. The panel's **Settle casino bets and bounties** scheduled task (every 30 seconds; see **Scheduled tasks**) pays out finished bets, refunds expired bounties and closes Mines games a player left open for a day.

## Limits

* Odds are shown to players on each game, and the daily loss limit is the only built-in spending guard. Pick bet limits that suit your economy.
* The casino is a launcher and web feature; there is no in-game casino GUI.
* A Crash round that is left running is lost when it crashes (the bet was taken at the start); a Blackjack hand left open for a day is a lost hand.

## On phones

The player panel behaves like a native app on touch screens: pinch and double-tap zoom are switched off, the page doesn't bounce or scroll sideways, fields never trigger the browser's focus zoom, and the main button of each game (Spin, Hit, Cash out and so on) stays pinned above the tab bar so it is always under your thumb. Install it to the home screen, or use the Android and iOS app, for the full-screen version.
