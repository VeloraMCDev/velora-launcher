package net.velora.core;

import java.util.List;
import java.util.Locale;
import java.util.function.Predicate;

/** In-game Velora help. Keep entries in step with shared/commands/index.ts. */
public final class CommandGuide {
    private CommandGuide() {}
    public record Entry(String group, String usage, String description, String permission) {
        public String command() { return usage.substring(1).split("[ |]", 2)[0]; }
    }
    public static final List<Entry> ENTRIES = List.of(
        new Entry("Velora help", "/help [page|command]", "Browse your available commands or look up one command.", ""),
        new Entry("Getting around", "/spawn", "Teleport to the server spawn.", "velora.command.spawn"),
        new Entry("Getting around", "/home [name]", "Teleport to a home, or open the homes menu.", "velora.command.home"),
        new Entry("Getting around", "/sethome [name]", "Save your current spot as a home.", "velora.command.sethome"),
        new Entry("Getting around", "/delhome <name>", "Delete one of your homes.", "velora.command.delhome"),
        new Entry("Getting around", "/back", "Return to where you were, or where you died.", "velora.command.back"),
        new Entry("Getting around", "/warp [name]", "Go to a server warp, or open the warps menu.", "velora.command.warp"),
        new Entry("Getting around", "/reqwarp <name>", "Request a public warp at your current location.", "velora.command.reqwarp"),
        new Entry("Getting around", "/warprequests", "Staff: list pending warp requests.", "velora.command.warp.manage"),
        new Entry("Getting around", "/warpapprove <name> · /warpdeny <name>", "Staff: approve or deny a warp request.", "velora.command.warp.manage"),
        new Entry("Getting around", "/setwarp <name> · /delwarp <name>", "Staff: create or delete public warps.", "velora.command.warp.manage"),
        new Entry("Getting around", "/rtp", "Jump to a random safe spot in the wild.", "velora.command.rtp"),
        new Entry("Getting around", "/tpa <player>", "Ask to teleport to another player.", "velora.command.tpa"),
        new Entry("Getting around", "/tpaccept", "Accept a teleport request.", "velora.command.tpaccept"),
        new Entry("Getting around", "/tpdeny", "Decline a teleport request.", "velora.command.tpdeny"),
        new Entry("Money & trading", "/balance", "Check your balance.", "velora.command.balance"),
        new Entry("Money & trading", "/pay <player> <amount>", "Send money to a player.", "velora.command.pay"),
        new Entry("Money & trading", "/baltop", "See the richest players.", "velora.command.baltop"),
        new Entry("Money & trading", "/shop", "Open the server shop.", "velora.command.shop"),
        new Entry("Money & trading", "/sell hand", "Sell the item in your hand to the shop.", "velora.command.sell.hand"),
        new Entry("Money & trading", "/sell", "Open the selling menu.", "velora.command.sell"),
        new Entry("Money & trading", "/market", "Browse and buy from the player market.", "velora.command.market"),
        new Entry("Money & trading", "/market sell [price]", "Sell the item in your hand for a fixed price. Without a price, a window lets you pick Buy Now or Auction with buttons.", "velora.command.market.sell"),
        new Entry("Money & trading", "/market auction [starting bid] [hours]", "Start an auction for the item in your hand (1–168 hours, 24 by default). Highest bid wins; outbid players are refunded.", "velora.command.market.sell"),
        new Entry("Money & trading", "/market bid <#id> [amount]", "Bid on an auction. Leave the amount out to bid the minimum. A late bid adds two minutes.", "velora.command.market.buy"),
        new Entry("Money & trading", "/market claim", "Collect auctions you won and items that came back unsold.", "velora.command.market.buy"),
        new Entry("Money & trading", "/market cancel <#id>", "Take your own listing down (auctions only before the first bid).", "velora.command.market.sell"),
        new Entry("Money & trading", "/market point add <name> [market|shop]", "Staff: turn the block you look at into a shop that opens the market or shop when right-clicked. Also shown on the map.", "velora.command.market.point"),
        new Entry("Money & trading", "/darknet", "Browse the Darknet catalog of items you can buy with your balance.", "velora.command.darknet"),
        new Entry("Money & trading", "/darknet buy <product>", "Buy a Darknet product. It is delivered to your vault, and the price shown when you looked is the price you pay.", "velora.command.darknet"),
        new Entry("Money & trading", "/orders", "View your active market orders.", "velora.command.orders"),
        new Entry("Money & trading", "/orders request <amount> <total dollars> <item> <deadline minutes>", "Post a buy order and escrow the money. Another player accepts it and hands in the items before your deadline (5 minutes to 30 days); expired orders refund you.", "velora.command.orders"),
        new Entry("Money & trading", "/orders pickup <id> · drop <id> · cancel <id>", "Accept an order, give it back, or cancel your own.", "velora.command.orders"),
        new Entry("Money & trading", "/contracts [list|submit|abandon]", "See your contracts, hand in the items for one, or swap it for another.", "velora.command.contracts"),
        new Entry("Money & trading", "/pshop create <dollars> [quantity]", "Make a pedestal shop: hold the product and look at a polished blackstone slab, then right-click the linked chest to stock it.", "velora.command.pshop"),
        new Entry("Money & trading", "/pshop price <id> <dollars> · close <id> · reopen <id>", "Change a pedestal shop’s price, or close and reopen it.", "velora.command.pshop"),
        new Entry("Money & trading", "/pshop stock <id>", "Recover stock from one of your shops, even if its blocks are gone.", "velora.command.pshop"),
        new Entry("Money & trading", "/pshop buy", "Buy from the pedestal you looked at after reading the chat quote.", "velora.command.pshop"),
        new Entry("Money & trading", "/pshop promote <id> <1-30 days> [warp] [confirm]", "Pin your shop on the map (and optionally add a warp). Shows the fee first; add confirm to pay.", "velora.command.pshop"),
        new Entry("Money & trading", "/pshop visit <id>", "Teleport to a promoted shop that has a warp.", "velora.command.pshop"),
        new Entry("Money & trading", "/trade <player>", "Start a secure trade with another player.", "velora.command.trade"),
        new Entry("Money & trading", "/transactions", "See your recent money movements.", "velora.command.transactions"),
        new Entry("Factions & land", "/faction", "Open your faction dashboard.", "velora.command.faction"),
        new Entry("Factions & land", "/faction create <name> <tag>", "Found a new faction.", "velora.command.faction.create"),
        new Entry("Factions & land", "/faction leave", "Leave your faction.", "velora.command.faction.leave"),
        new Entry("Factions & land", "/faction members", "List your faction members.", "velora.command.faction.members"),
        new Entry("Factions & land", "/faction chat <message>", "Talk to your faction only.", "velora.command.faction.chat"),
        new Entry("Factions & land", "/faction sethome · /faction home", "Set or visit the faction home.", "velora.command.faction.sethome"),
        new Entry("Factions & land", "/faction map", "Show nearby claims on a map.", "velora.command.faction.map"),
        new Entry("Factions & land", "/faction invite <player>", "Invite a player to your faction (leaders and officers).", "velora.command.faction.invite"),
        new Entry("Factions & land", "/faction list · /faction info <name>", "Browse the factions on this server.", "velora.command.faction"),
        new Entry("Factions & land", "/faction join <name> [message]", "Ask to join a faction. Its leaders get a notification.", "velora.command.faction"),
        new Entry("Factions & land", "/faction requests · approve <player> · reject <player>", "Review join requests (leaders and officers).", "velora.command.faction"),
        new Entry("Factions & land", "/faction kick <player>", "Remove a member. Only the leader can remove officers.", "velora.command.faction"),
        new Entry("Factions & land", "/faction promote <player> · demote <player>", "Make someone an officer, or a member again (leader).", "velora.command.faction"),
        new Entry("Factions & land", "/faction roles · role <player> <role>", "See the roles and give a custom role (leader).", "velora.command.faction"),
        new Entry("Factions & land", "/faction transfer <player> confirm", "Hand the faction to someone else; you become an officer.", "velora.command.faction"),
        new Entry("Factions & land", "/faction motd <text> · /faction desc <text>", "Set the message of the day or the description.", "velora.command.faction"),
        new Entry("Factions & land", "/faction post <title> | <text> · /faction posts", "Post an announcement everyone in the faction is notified about, or read the board.", "velora.command.faction"),
        new Entry("Factions & land", "/faction accept [tag] · /faction decline [tag]", "Answer a faction invitation. You can also answer in the launcher.", "velora.command.faction.accept"),
        new Entry("Factions & land", "/faction rename <name> [tag] · /faction disband confirm", "Rename the faction, or disband it (leader).", "velora.command.faction"),
        new Entry("Factions & land", "/faction flags", "Show the protection flags for your faction land.", "velora.command.faction.flags"),
        new Entry("Factions & land", "/faction upgrade [claims|members|outposts|vault] [tier]", "See or buy faction upgrades with the faction bank: more claim capacity, members, outpost flags and vault pages. Repeating a purchase never charges twice.", "velora.command.faction"),
        new Entry("Factions & land", "/faction outpost place", "Hold a purchased Outpost Flag and place it to start a second claim area. Each flag allows twelve chunks within three chunks of it.", "velora.command.faction"),
        new Entry("Factions & land", "/faction vault [page]", "Open shared faction storage (pages 1–9). Leaders, officers and roles with storage access only.", "velora.command.faction"),
        new Entry("Factions & land", "/claim", "Claim the chunk you stand in for your faction.", "velora.command.claim"),
        new Entry("Factions & land", "/unclaim", "Give up the chunk you stand in.", "velora.command.unclaim"),
        new Entry("Factions & land", "/autoclaim on|off", "Automatically claim new chunks as you walk.", "velora.command.claim"),
        new Entry("Faction bank", "/faction bank", "Show the bank balance and recent activity.", "velora.command.faction.bank"),
        new Entry("Faction bank", "/faction bank deposit <amount>", "Put your own money into the bank.", "velora.command.faction.bank.deposit"),
        new Entry("Faction bank", "/faction bank withdraw <amount>", "Take money out (officers and leaders).", "velora.command.faction.bank.withdraw"),
        new Entry("Faction bank", "/faction sell hand", "Sell your held item and pay the bank.", "velora.command.faction.sell"),
        new Entry("Faction bank", "/faction market", "Buy and sell on the market as your faction.", "velora.command.faction.market"),
        new Entry("Faction bank", "/faction pay <tag> <amount>", "Pay another faction from your bank.", "velora.command.faction.pay"),
        new Entry("Everyday perks", "/cosmetic [list|equip <key>|unequip <type>]", "Browse and equip unlocked titles, badges, pets and cosmetics.", "velora.command.cosmetic"),
        new Entry("Everyday perks", "/fly [on|off]", "Fly freely. free.fly works anywhere; faction.fly only inside your own faction’s land (it switches off when you leave).", "free.fly or faction.fly"),
        new Entry("Everyday perks", "/heal [player]", "Restore health (the amount and wait are set by the server).", "velora.command.heal"),
        new Entry("Everyday perks", "/feed [player]", "Restore hunger (the wait is set by the server).", "velora.command.feed"),
        new Entry("Everyday perks", "/vault [number]", "Open one of your cloud vaults. You start with one free page; more are unlocked in the Market > Vaults page, the Darknet or the K menu. Your vaults are also visible in the launcher.", "velora.command.vault"),
        new Entry("Everyday perks", "/echest", "Open your ender chest from anywhere.", "velora.command.echest"),
        new Entry("Everyday perks", "/kit [name]", "List the kits you can claim, or claim one.", "velora.command.kit"),
        new Entry("Everyday perks", "/quests [player]", "See your daily and weekly quests, or anyone else’s.", "velora.command.quests"),
        new Entry("Info & admin", "/shopkeeper spawn <name> [market|shop] [mob]", "Staff: place a villager (or any mob) that opens the market when right-clicked. It never moves and is marked on the map.", "velora.command.market.point"),
        new Entry("Info & admin", "/shopkeeper link <name> · remove <name> · list", "Staff: turn the mob you look at into a shopkeeper, free it again, or list them.", "velora.command.market.point"),
        new Entry("Info & admin", "/adminclaim create <name> [description]", "Staff: protect the chunk you stand in as server land with a name and description. Also manage them in the admin panel.", "velora.admin.claims"),
        new Entry("Info & admin", "/customitem give <player> <id>", "Staff: hand out an item designed in the admin panel.", "velora.admin.customitem"),
        new Entry("Info & admin", "/playtime", "Check your total and session playtime.", "velora.command.playtime"),
        new Entry("Info & admin", "/hand", "Share your held item as an inspectable chat link.", "velora.command.hand"),
        new Entry("Info & admin", "/nick <name|off>", "Set or remove your chat nickname.", "velora.command.nick"),
        new Entry("Info & admin", "/velora help [page|command]", "Browse your available Velora commands with descriptions.", "velora.command.velora.help"),
        new Entry("Info & admin", "/velora panel", "Get the link to the player panel: market, casino, friends, factions and quests in any browser or phone. Sign in with your launcher account.", "velora.command.velora.panel"),
        new Entry("Info & admin", "/velora status", "Panel connection, features and mod integrations (admins).", "velora.command.velora.status"),
        new Entry("Info & admin", "/velora reload", "Reload the Velora config (admins).", "velora.command.velora.reload")
    );
    public static List<Entry> visible(CorePlayer player, Predicate<String> available, String query) {
        String term = query.toLowerCase(Locale.ROOT).replaceFirst("^/", "");
        return ENTRIES.stream().filter(e -> available.test(e.command()) && (e.permission().isBlank() || java.util.Arrays.stream(e.permission().split(" or ")).anyMatch(player::hasPermission)))
                .filter(e -> term.isBlank() || e.command().equals(term) || e.usage().toLowerCase(Locale.ROOT).startsWith("/" + term + " ") || e.group().toLowerCase(Locale.ROOT).contains(term)).toList();
    }
    public static void show(CorePlayer player, Predicate<String> available, String[] args) {
        int page = 1;
        String query = "";
        if (args.length > 0) {
            try { page = Integer.parseInt(args[0]); }
            catch (NumberFormatException ignored) {
                query = String.join(" ", args);
                if (args.length > 1) {
                    try { page = Integer.parseInt(args[args.length - 1]); query = String.join(" ", java.util.Arrays.copyOf(args, args.length - 1)); }
                    catch (NumberFormatException topic) { /* multiword topic */ }
                }
            }
        }
        List<Entry> entries = visible(player, available, query);
        int pages = Math.max(1, (entries.size() + 5) / 6);
        if (page < 1 || page > pages) { player.send(Format.RED + "Choose a help page from 1 to " + pages + ". Use /help [page] or /help <command>. "); return; }
        player.send(Format.GOLD + "=== Velora help " + page + "/" + pages + (query.isBlank() ? "" : " - " + query) + " ===");
        if (entries.isEmpty()) player.send(Format.GRAY + "No available commands match. Use /help to browse your commands.");
        String lastGroup = "";
        for (Entry entry : entries.subList(Math.min((page - 1) * 6, entries.size()), Math.min(page * 6, entries.size()))) {
            if (!entry.group().equals(lastGroup)) { player.send(Format.AQUA + entry.group()); lastGroup = entry.group(); }
            player.send(Format.YELLOW + entry.usage() + Format.GRAY + " - " + entry.description());
        }
        player.send(Format.GRAY + (page < pages ? "Next: /help " + (query.isBlank() ? "" : query + " ") + (page + 1) + " | " : "") + "/help <command> for details | /velora panel for the player panel");
    }
}
