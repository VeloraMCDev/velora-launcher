# SCOPENET demo environment

A throwaway panel on a scratch data dir, filled with believable community data through the panel's real HTTP APIs.
Use it to try the player-facing pages and to take README screenshots of the admin panel, player panel and launcher.

```bash
scripts/demo/run.sh    # build if needed, wipe scripts/demo/.data, start the panel, seed it (~10 s)
scripts/demo/stop.sh   # stop the panel and the heartbeat
node scripts/demo/seed.mjs   # re-seed an already running, *fresh* panel (not idempotent on a used data dir)
```

`run.sh` builds `panel/web` (when `dist` is missing or older than `src`) and `target/debug/scopenet-panel` (when missing),
starts the panel in the background (pid in `.pid`, log in `.panel.log`) and starts a small heartbeat (`seed.mjs --heartbeat`,
pid in `.hb.pid`) that syncs the two demo game servers every 30 s so they stay "online" with 7 and 3 players.
`DEMO_PORT=19090 scripts/demo/run.sh` uses another port.

## Where things are

| What | Value |
| --- | --- |
| Panel, admin UI, player panel | `http://127.0.0.1:18080` |
| Admin | `admin` / `demo-admin-pass` |
| Players (all 16 + `Fresh_Spawn`) | password `demo-pass-1234` |
| Main screenshot player | `Alex_Miner` (level 17, ~12k balance on Survival SMP, guild leader of `IRON`, 7 friends, DMs, auctions, quests) |
| Game servers | id `1` Survival SMP, id `2` Creative Build (their tokens are in `.data/demo.json`) |
| Instances | `scopenet-survival` (Fabric 1.21.4), `creative-build` (vanilla 1.21.4) |
| Data dir | `scripts/demo/.data/` (git-ignored) |

Other players: Steve_Builds, CreeperSlayer, LunaMC, Redstone_Rae, DiamondDave, NetherNinja, PixelPaige, TNT_Tommy, EnderEmma,
FarmerFinn, ObsidianOwen, BlazeRunner, MossyMax, Cobble_Cleo, Axolotl_Ava.

## What is seeded

Branding, news and links, open sign-ups, landing page; 2 instances and 2 game servers with tps and online players; stats,
events and levels for 16 players (leaderboards fill up); balances from ~400 to ~48k and some payments; 4 guilds (members,
officer/custom role, claimed chunks, bank, posts, a pending join request and an invite); 27 + 4 market listings with
buy-now items, auctions (some ending within 1-2 hours), ~35 bids, a few completed sales; friendships (incl. pending
requests), DMs (some unread), bios, profile posts with likes; a fixed set of 5 daily + 3 weekly quests with partial,
completed and claimed progress; the casino with ~130 real rounds, daily spins, bounties and bet markets; player skins
(generated procedurally).

Quests are pinned so every player sees the same 8; most are "action" quests whose progress the seed sets explicitly.

## Using it with the dev servers

```bash
# admin panel + player panel (proxies /api and friends to the demo panel)
cd panel/web && PANEL_URL=http://127.0.0.1:18080 npx vite --port 5199

# launcher UI in a browser (Tauri calls are mocked there; the launcher vite config has no panel proxy yet,
# so PANEL_URL only matters once it grows one)
cd launcher && PANEL_URL=http://127.0.0.1:18080 npx vite --port 1420
```

Player endpoints used by the launcher live under `/api/v1` with `Authorization: Bearer <token>` (`POST /api/v1/auth/login`
returns the token). Game-server endpoints live under `/api/server/v1` with the server token.

## Notes

- The seed uses random game outcomes (casino, plinko, bids), so numbers differ slightly between runs; the structure is the same.
- Item ids use Bukkit material names (`DIAMOND_SWORD`), which is what the Paper plugin sends and what the launcher's item tiles expect.
- Optional steps (skins, landing page, groups, ...) print a `[warn]` and continue; required steps stop the seed with a message.
