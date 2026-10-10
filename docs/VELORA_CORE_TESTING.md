# Velora Core staging test

This is a test-build handoff, not a production release. Casino rooms, tournaments and custom casino blocks are deferred. The broader remaining feature matrix is in [VELORA_CORE.md](VELORA_CORE.md).

## Install together

Use a copy of the SMP world and its corresponding panel instance database. Keep a matched backup of both, including playerdata, config and signing/authentication material. Do not test recovery against production balances or real player items.

1. Build/update the Panel backend and web from this checkout before installing the new mods. New custody/shop routes and retained SQL migrations are required. An older running panel cannot serve this build. Use the existing manual build/deployment lifecycle; no automatic deployment was performed.
2. Build the Fabric 1.20.1 server and client separately using the [integration guide](../integrations/velora-core/README.md). The remapped outputs are integrations/fabric/build/libs/scopenet-fabric-1.20.1-0.5.0.jar and integrations/fabric-client/build/libs/scopenet-client-fabric-1.20.1-0.5.0.jar. Replace the old matching-side jar; do not install two copies with the same mod ID. Archive filenames and protocol identities remain unchanged.
3. Install Fabric API on both sides. Install the Fabric LuckPerms mod on the server for panel-managed permissions. Use the existing custom authentication configuration, server token and online-mode=true. Never put the server token in client config.
4. Use the existing SMP experience/module configuration. Activation hides other experiences and must be performed deliberately; it does not stop their server processes. Review existing personal-vault counts/free pages, faction storage role grants and any old paid outpost tiers before testing.
5. Start with two synthetic accounts: a seller/faction leader and a buyer/member, plus an operator. Record initial balances and exact item quantities across inventory, vaults, stock and mailbox. Use named/enchanted items and stacks with different NBT.

## Functional pass

- Authentication: both accounts log in through custom auth; an unlinked/revoked account is denied. Reload preserves all existing UUID/protocol/config identities.
- Native hub: K opens Market, Darknet, Casino, Faction, Map and Vault 1. V/M work; module switches hide/deny the appropriate controls. Test GUI scale and small resolutions. Native casino options currently use presets for Mines, Dice, Plinko and Crash.
- Map: use Claims, right-click a chunk, then Claim or Unclaim. Test negative coordinates, another dimension, another faction's territory, ordinary-member denial and capacity/upkeep freeze. Verify refreshed borders and launcher/web pins; toggle all global/player layers.
- Vault: click whole stacks in both directions, close/reopen, move a stack from the launcher/web while closed, and verify edits are blocked while open or unresolved. Test full inventory/full vault, two viewers of one faction vault and locked personal pages. Item count, name, enchantments, damage and NBT must survive exactly. Extra personal/faction vault prices are configurable in the Experience Editor; total/free personal counts are in Utilities.
- Darknet: review/buy an item and a permanent vault unlock in game and the launcher/web. Change a price between review and purchase. Test an offline buyer, full vaults, server restart and delayed mailbox processing. Delivery can wait for a running server and sufficient room.
- Casino: exercise all ten games in game and launcher/web, switch interfaces during Blackjack/Mines/Burst/Crash, reconnect and resume, test cashout/loss and insufficient funds. Crash's native preset includes 2x automatic cashout. Disable an individual game during an active stake and test settlement; settle stakes before disabling the whole module. Check balances and history after a lost response; do not submit a new wager to guess whether an old wager completed.
- Faction: view roster/bank, deposit/withdraw with leader and member roles, open faction vaults only with an explicit storage grant, test claim upgrades and the three-day upkeep freeze/recovery. Test agreed rival cooldowns and ally protection flags.
- Permissions/chat: create a LuckPerms group from the panel, assign a contextual permission and prefix/suffix, add/remove a player, reconnect/restart and verify persistence. Test explicit denies, color permission, signed chat, filtering, narration and the installed chat/moderation mods. Automatic rank/group mapping stays off unless explicitly enabled in server config.

## Pedestal shop pass

Inside the seller's faction claim, place a polished blackstone slab and exactly one empty single chest within two Manhattan blocks. Hold a product and look at the slab, then run /pshop create 25 2 to sell two matching items for $25. The held sample must remain unchanged. Record the returned shop UUID.

Right-click the chest to put stock into its protected three-row store. Close stock before selling. The chest opens cloud stock; do not expect physical vanilla slots to contain a second copy. Have the buyer right-click the pedestal, review the native offer and confirm. Verify a $25 debit, a full $25 seller credit, exactly two stock items consumed and exactly two exact-NBT items delivered to the buyer's vault. Repeat with a visually similar item with different tags; it must not substitute.

Test /pshop price <id> 30, /pshop close <id>, /pshop reopen <id> and /pshop stock <id>. Break/remove the pedestal/chest: stock must remain recoverable and purchases must pause. Test vanilla hopper insertion/extraction, double-chest conversion and every installed automation/protection mod. A chest that receives vanilla contents from another mod must be repaired before a sale; the cloud stock is independent.

Run /pshop promote <id> 1 warp to review a listing, then repeat with confirm. The default is $50/day; change it in the Experience Editor and verify stale quotes are refused. Retry the same confirmation after losing a response: it must not charge or extend twice. Check shop pins in all maps and /pshop visit <id>; expired/closed listings must not offer a warp. Change/unclaim the seller's territory and verify claim-required purchases stop.

## Crash and outage pass

Use synthetic items and a disposable matched backup. Test both deposit and withdrawal, personal and faction vaults, and shop stock:

1. Interrupt panel connectivity before prepare: no inventory mutation; interaction remains protected until recovery.
2. Terminate the game process after prepare but before player-data persistence. On reconnect, the absent checkpoint must cancel and preserve the original inventory/vault totals.
3. Terminate after the inventory checkpoint is written but before panel commit. Reconnect must commit the prepared after-state once.
4. Lose the commit response after the panel writes it. Retry/reconnect must not repeat the inventory change, money charge or stock consumption.
5. Restart the panel with a prepared row, wait beyond the ordinary lease, and attempt a launcher/web edit. It must remain blocked. Reconnect the originating player/server to resolve it.
6. Disconnect/reconnect during a transfer, use two shared-vault viewers, and attempt drop, crafting, item-frame/armor-stand interaction, creative editing, book editing, pickup, death and commands while locked.
7. Test the actual server filesystem's atomic replacement and permissions. Ambiguous player-data writes must fail closed and require reconnect recovery. Restoring only one side of a backup or loading corrupt player data is an operator reconciliation case, not an automatic cancellation decision.

The new checkpoint protocol covers cloud-vault/shop-stock clicks. Existing market listing and contract hand-in journals and third-party inventory writes still need their own restart/conservation tests; they are not certified by the vault tests. Do not advertise general crash safety until these tests pass on the actual modpack.

Record each test's initial/final counts, balances, result and logs with synthetic data. Stop rollout on duplication, loss, authorization bypass, failed mixins or unresolved custody. Packaging/signing and production promotion remain manual.
