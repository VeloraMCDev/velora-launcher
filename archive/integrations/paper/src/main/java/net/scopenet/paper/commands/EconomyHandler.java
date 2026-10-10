package net.scopenet.paper.commands;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.scopenet.integration.Integration;
import net.scopenet.paper.ScopenetPlugin;
import net.scopenet.paper.gui.GuiHelper;
import org.bukkit.Bukkit;
import org.bukkit.ChatColor;
import org.bukkit.Material;
import org.bukkit.command.Command;
import org.bukkit.command.CommandExecutor;
import org.bukkit.command.CommandSender;
import org.bukkit.entity.Player;
import org.bukkit.event.EventHandler;
import org.bukkit.event.Listener;
import org.bukkit.event.inventory.InventoryClickEvent;
import org.bukkit.event.inventory.InventoryCloseEvent;
import org.bukkit.inventory.Inventory;
import org.bukkit.inventory.ItemStack;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;

public final class EconomyHandler implements CommandExecutor, Listener {
    private final ScopenetPlugin plugin;
    private final Integration integration;
    private final EconomyOperations operations;
    private final MarketWizard wizard = new MarketWizard(this::listHeld);
    /** One market window slot: which listing it shows and what a bid has to be. */
    private record Slot(long id, boolean auction, double minBid) {}

    private static final String GUI_SHOP_TITLE = ChatColor.DARK_GREEN + "Velora Server Shop";
    private static final String GUI_SELL_TITLE = ChatColor.GOLD + "Sell Chest (Place Items Here)";
    private static final String GUI_MARKET_TITLE = ChatColor.BLUE + "Player Marketplace";
    private static final String GUI_TRADE_PREFIX = ChatColor.DARK_PURPLE + "Trade: ";

    // Trade sessions: maps player UUID to active trade session
    private final Map<UUID, TradeSession> activeTrades = new ConcurrentHashMap<>();

    // Sell chests opened with /guild sell pay the guild bank.
    private final Set<Inventory> guildSellChests = Collections.newSetFromMap(new IdentityHashMap<>());

    /** Durable, idempotent panel requests; also used by the guild bank. */
    EconomyOperations operations() { return operations; }

    // Each open inventory must retain its own snapshot of listing IDs.
    private final Map<Inventory, Map<Integer, Slot>> activeMarketListings = new IdentityHashMap<>();

    // Standard sell values for common items
    private static final Map<Material, Double> ITEM_SELL_VALUES = new HashMap<>();
    static {
        ITEM_SELL_VALUES.put(Material.COBBLESTONE, 0.1);
        ITEM_SELL_VALUES.put(Material.STONE, 0.2);
        ITEM_SELL_VALUES.put(Material.OAK_LOG, 0.5);
        ITEM_SELL_VALUES.put(Material.BIRCH_LOG, 0.5);
        ITEM_SELL_VALUES.put(Material.SPRUCE_LOG, 0.5);
        ITEM_SELL_VALUES.put(Material.COAL, 1.0);
        ITEM_SELL_VALUES.put(Material.RAW_COPPER, 1.5);
        ITEM_SELL_VALUES.put(Material.COPPER_INGOT, 2.0);
        ITEM_SELL_VALUES.put(Material.RAW_IRON, 3.0);
        ITEM_SELL_VALUES.put(Material.IRON_INGOT, 4.0);
        ITEM_SELL_VALUES.put(Material.RAW_GOLD, 6.0);
        ITEM_SELL_VALUES.put(Material.GOLD_INGOT, 8.0);
        ITEM_SELL_VALUES.put(Material.REDSTONE, 1.5);
        ITEM_SELL_VALUES.put(Material.LAPIS_LAZULI, 2.0);
        ITEM_SELL_VALUES.put(Material.DIAMOND, 35.0);
        ITEM_SELL_VALUES.put(Material.EMERALD, 20.0);
        ITEM_SELL_VALUES.put(Material.NETHERITE_SCRAP, 150.0);
        ITEM_SELL_VALUES.put(Material.NETHERITE_INGOT, 650.0);
        ITEM_SELL_VALUES.put(Material.WHEAT, 0.5);
        ITEM_SELL_VALUES.put(Material.CARROT, 0.5);
        ITEM_SELL_VALUES.put(Material.POTATO, 0.5);
        ITEM_SELL_VALUES.put(Material.BEEF, 1.0);
        ITEM_SELL_VALUES.put(Material.COOKED_BEEF, 2.0);
        ITEM_SELL_VALUES.put(Material.PORKCHOP, 1.0);
        ITEM_SELL_VALUES.put(Material.COOKED_PORKCHOP, 2.0);
    }

    public EconomyHandler(ScopenetPlugin plugin, Integration integration) {
        this.plugin = plugin;
        this.integration = integration;
        this.operations = new EconomyOperations(plugin, integration);
    }

    private void giveOrDrop(Player player, ItemStack item) {
        for (ItemStack remaining : player.getInventory().addItem(item).values()) {
            player.getWorld().dropItemNaturally(player.getLocation(), remaining);
        }
    }

    @Override
    public boolean onCommand(CommandSender sender, Command command, String label, String[] args) {
        if (!(sender instanceof Player player)) {
            sender.sendMessage(ChatColor.RED + "Only in-game players can execute economy commands.");
            return true;
        }

        String cmd = command.getName().toLowerCase();
        switch (cmd) {
            case "balance" -> handleBalance(player);
            case "pay" -> handlePay(player, args);
            case "baltop" -> handleBaltop(player);
            case "shop" -> handleShop(player);
            case "sell" -> handleSell(player, args);
            case "market" -> handleMarket(player, args);
            case "orders" -> handleOrders(player, args);
            case "contracts" -> handleContracts(player, args);
            case "trade" -> handleTrade(player, args);
            case "transactions" -> handleTransactions(player);
            default -> { return false; }
        }
        return true;
    }

    /** Open the player market window (used by physical shop points). */
    public void openMarket(Player player) { handleMarket(player, new String[0], false); }

    /** Open the server shop window (used by physical shop points). */
    public void openShop(Player player) { handleShop(player); }

    private void handleBalance(Player player) {
        player.sendMessage(ChatColor.GRAY + "Fetching your balance...");
        Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
            try {
                double bal = integration.client().getBalance(player.getUniqueId(), player.getName());
                player.sendMessage(ChatColor.GOLD + "=== Velora Economy ===");
                player.sendMessage(ChatColor.YELLOW + "Your Balance: " + ChatColor.GREEN + "$" + String.format(Locale.US, "%,.2f", bal));
            } catch (Exception e) {
                player.sendMessage(ChatColor.RED + "Could not reach economy server: " + e.getMessage());
            }
        });
    }

    private void handlePay(Player player, String[] args) {
        if (args.length < 2) {
            player.sendMessage(ChatColor.RED + "Usage: /pay <player> <amount>");
            return;
        }

        Player target = Bukkit.getPlayer(args[0]);
        if (target == null || !target.isOnline()) {
            player.sendMessage(ChatColor.RED + "Player '" + args[0] + "' is not online.");
            return;
        }

        if (target.getUniqueId().equals(player.getUniqueId())) {
            player.sendMessage(ChatColor.RED + "You cannot pay yourself.");
            return;
        }

        double amount;
        try {
            amount = Double.parseDouble(args[1]);
            if (amount <= 0 || Double.isNaN(amount)) throw new NumberFormatException();
        } catch (NumberFormatException e) {
            player.sendMessage(ChatColor.RED + "Invalid amount specified. Must be positive number.");
            return;
        }

        player.sendMessage(ChatColor.GRAY + "Processing payment...");
        Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
            try {
                double newBal = integration.client().transfer(
                        player.getUniqueId(), player.getName(),
                        target.getUniqueId(), target.getName(),
                        amount, "Player payment to " + target.getName()
                );
                player.sendMessage(ChatColor.GREEN + "Sent " + ChatColor.YELLOW + "$" + String.format(Locale.US, "%,.2f", amount)
                        + ChatColor.GREEN + " to " + ChatColor.YELLOW + target.getName() + ChatColor.GREEN + ". New Balance: "
                        + ChatColor.YELLOW + "$" + String.format(Locale.US, "%,.2f", newBal));
                target.sendMessage(ChatColor.GREEN + "Received " + ChatColor.YELLOW + "$" + String.format(Locale.US, "%,.2f", amount)
                        + ChatColor.GREEN + " from " + ChatColor.YELLOW + player.getName() + ChatColor.GREEN + "!");
            } catch (Exception e) {
                player.sendMessage(ChatColor.RED + "Transfer failed: " + e.getMessage());
            }
        });
    }

    private void handleBaltop(Player player) {
        player.sendMessage(ChatColor.GRAY + "Loading leaderboard...");
        Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
            try {
                JsonArray baltop = integration.client().getBaltop();
                player.sendMessage(ChatColor.GOLD + "================= " + ChatColor.YELLOW + "BALTOP (Richest Players)" + ChatColor.GOLD + " =================");
                if (baltop.size() == 0) {
                    player.sendMessage(ChatColor.GRAY + "No players found on the leaderboard.");
                    return;
                }
                for (int i = 0; i < Math.min(10, baltop.size()); i++) {
                    JsonObject entry = baltop.get(i).getAsJsonObject();
                    int rank = entry.has("rank") ? entry.get("rank").getAsInt() : (i + 1);
                    String name = entry.has("username") ? entry.get("username").getAsString() : "Unknown";
                    double bal = entry.has("balance") ? entry.get("balance").getAsDouble() : 0.0;
                    ChatColor rankColor = rank == 1 ? ChatColor.GOLD : (rank == 2 ? ChatColor.WHITE : (rank == 3 ? ChatColor.YELLOW : ChatColor.GRAY));
                    player.sendMessage(rankColor + "#" + rank + " " + ChatColor.WHITE + name + ": " + ChatColor.GREEN + "$" + String.format(Locale.US, "%,.2f", bal));
                }
            } catch (Exception e) {
                player.sendMessage(ChatColor.RED + "Could not fetch baltop: " + e.getMessage());
            }
        });
    }

    private void handleShop(Player player) {
        Inventory inv = Bukkit.createInventory(null, 54, GUI_SHOP_TITLE);

        // Fill border
        ItemStack border = GuiHelper.createBorder(Material.BLACK_STAINED_GLASS_PANE);
        for (int i = 0; i < 9; i++) inv.setItem(i, border);
        for (int i = 45; i < 54; i++) inv.setItem(i, border);

        // Category / Popular items in shop
        inv.setItem(10, GuiHelper.createItem(Material.DIAMOND, "&bDiamond &7(x1)", "&7Price: &a$100.00", "", "&eClick to Buy!"));
        inv.setItem(11, GuiHelper.createItem(Material.NETHERITE_INGOT, "&5Netherite Ingot &7(x1)", "&7Price: &a$1,500.00", "", "&eClick to Buy!"));
        inv.setItem(12, GuiHelper.createItem(Material.IRON_INGOT, 16, "&fIron Ingot &7(x16)", "&7Price: &a$80.00", "", "&eClick to Buy!"));
        inv.setItem(13, GuiHelper.createItem(Material.GOLD_INGOT, 16, "&6Gold Ingot &7(x16)", "&7Price: &a$150.00", "", "&eClick to Buy!"));
        inv.setItem(14, GuiHelper.createItem(Material.EMERALD, 16, "&aEmerald &7(x16)", "&7Price: &a$250.00", "", "&eClick to Buy!"));
        inv.setItem(15, GuiHelper.createItem(Material.OAK_LOG, 32, "&6Oak Wood &7(x32)", "&7Price: &a$20.00", "", "&eClick to Buy!"));
        inv.setItem(16, GuiHelper.createItem(Material.STONE, 64, "&7Stone &7(x64)", "&7Price: &a$25.00", "", "&eClick to Buy!"));

        inv.setItem(19, GuiHelper.createItem(Material.COOKED_BEEF, 16, "&cCooked Steak &7(x16)", "&7Price: &a$40.00", "", "&eClick to Buy!"));
        inv.setItem(20, GuiHelper.createItem(Material.GOLDEN_APPLE, 4, "&eGolden Apple &7(x4)", "&7Price: &a$300.00", "", "&eClick to Buy!"));
        inv.setItem(21, GuiHelper.createItem(Material.EXPERIENCE_BOTTLE, 16, "&aBottle o' Enchanting &7(x16)", "&7Price: &a$200.00", "", "&eClick to Buy!"));
        inv.setItem(22, GuiHelper.createItem(Material.ENDER_PEARL, 8, "&3Ender Pearl &7(x8)", "&7Price: &a$120.00", "", "&eClick to Buy!"));
        inv.setItem(23, GuiHelper.createItem(Material.OBSIDIAN, 14, "&8Obsidian &7(x14)", "&7Price: &a$150.00", "", "&eClick to Buy!"));
        inv.setItem(24, GuiHelper.createItem(Material.TORCH, 64, "&eTorches &7(x64)", "&7Price: &a$10.00", "", "&eClick to Buy!"));
        inv.setItem(25, GuiHelper.createItem(Material.BOW, "&bPower Bow", "&7Price: &a$100.00", "", "&eClick to Buy!"));

        inv.setItem(49, GuiHelper.createItem(Material.GOLD_INGOT, "&eCheck Balance / Sell Items", "&7Type &a/sell &7to sell your loot!", "&7Type &a/market &7for player listings."));

        player.openInventory(inv);
    }

    private void handleSell(Player player, String[] args) {
        handleSell(player, args, false);
    }

    /** {@code guild}: the proceeds go to the player's guild bank instead of their balance. */
    void handleSell(Player player, String[] args, boolean guild) {
        if (args.length > 0 && args[0].equalsIgnoreCase("hand")) {
            if (!guild && !player.hasPermission("scopenet.command.sell.hand")) {
                player.sendMessage(ChatColor.RED + "You do not have permission to use /sell hand.");
                return;
            }
            ItemStack hand = player.getInventory().getItemInMainHand();
            if (hand.getType().isAir()) {
                player.sendMessage(ChatColor.RED + "You are not holding any item to sell.");
                return;
            }
            Double unitPrice = ITEM_SELL_VALUES.get(hand.getType());
            if (unitPrice == null) {
                player.sendMessage(ChatColor.RED + "This item (" + hand.getType().name() + ") cannot be sold to the server shop.");
                return;
            }
            ItemStack sold = hand.clone();
            int amount = sold.getAmount();
            double total = unitPrice * amount;
            JsonObject payload = new JsonObject();
            payload.addProperty("uuid", player.getUniqueId().toString());
            payload.addProperty("username", player.getName());
            payload.addProperty(guild ? "amount" : "delta", total);
            payload.addProperty("description", "Sell hand: " + sold.getType().name());
            player.getInventory().setItemInMainHand(null);
            if (!operations.enqueue(player, guild ? "guilds/bank/credit" : "economy/adjust", payload, List.of(sold), null)) giveOrDrop(player, sold);
            return;
        }

        // Open sell chest GUI
        Inventory sellInv = Bukkit.createInventory(null, 54, GUI_SELL_TITLE);
        if (guild) guildSellChests.add(sellInv);
        player.openInventory(sellInv);
        player.sendMessage(ChatColor.YELLOW + "Place items to sell in the chest. Close the chest to confirm your sale!"
                + (guild ? ChatColor.GOLD + " The money goes to your guild bank." : ""));
    }

    private void handleMarket(Player player, String[] args) {
        handleMarket(player, args, false);
    }

    /** {@code asGuild}: list on behalf of the player's guild, or buy with its bank (see /guild market). */
    void handleMarket(Player player, String[] args, boolean asGuild) {
        if (!asGuild && args.length >= 1 && args[0].equalsIgnoreCase("point")) {
            plugin.shopCommand(player, java.util.Arrays.copyOfRange(args, 1, args.length));
            return;
        }
        if (args.length >= 2 && args[0].equalsIgnoreCase("buy")) {
            if (!asGuild && !player.hasPermission("scopenet.command.market.buy")) {
                player.sendMessage(ChatColor.RED + "You do not have permission to buy from the market.");
                return;
            }
            long listing;
            try { listing = Long.parseLong(args[1].replace("#", "")); } catch (NumberFormatException e) {
                player.sendMessage(ChatColor.RED + "Usage: /" + (asGuild ? "guild market" : "market") + " buy <listing number> (shown as Listing #n in /market)");
                return;
            }
            JsonObject payload = new JsonObject();
            payload.addProperty("listing_id", listing);
            payload.addProperty("buyer_uuid", player.getUniqueId().toString());
            payload.addProperty("buyer_name", player.getName());
            if (asGuild) payload.addProperty("as_guild", true);
            operations.enqueue(player, "economy/market/buy", payload, Collections.emptyList(), null);
            return;
        }
        if (args.length >= 1 && (args[0].equalsIgnoreCase("sell") || args[0].equalsIgnoreCase("auction"))) {
            boolean auction = args[0].equalsIgnoreCase("auction");
            if (!asGuild && !player.hasPermission("scopenet.command.market.sell")) {
                player.sendMessage(ChatColor.RED + "You do not have permission to list items on the market.");
                return;
            }
            ItemStack hand = player.getInventory().getItemInMainHand();
            if (hand.getType().isAir()) {
                player.sendMessage(ChatColor.RED + "Hold the item you want to " + (auction ? "auction" : "sell") + " in your main hand.");
                return;
            }
            if (args.length == 1) {
                // No price typed: the point-and-click window.
                wizard.open(player, hand, auction, 10, asGuild);
                return;
            }
            double price;
            try {
                price = Double.parseDouble(args[1]);
                if (price <= 0 || Double.isNaN(price) || Double.isInfinite(price)) throw new NumberFormatException();
            } catch (NumberFormatException e) {
                player.sendMessage(ChatColor.RED + "Invalid price. Usage: /market " + (auction ? "auction <starting bid> [hours]" : "sell <price>") + " - or just /market " + (auction ? "auction" : "sell") + " for the window.");
                return;
            }
            int hours = 24;
            if (auction && args.length > 2) {
                try { hours = Integer.parseInt(args[2]); } catch (NumberFormatException e) { hours = -1; }
                if (hours < 1 || hours > 168) { player.sendMessage(ChatColor.RED + "Auctions run for 1 to 168 hours."); return; }
            }
            listHeld(player, hand.clone(), price, auction, hours, asGuild);
            return;
        }
        if (args.length >= 2 && args[0].equalsIgnoreCase("bid")) {
            long listing;
            try { listing = Long.parseLong(args[1].replace("#", "")); } catch (NumberFormatException e) {
                player.sendMessage(ChatColor.RED + "Usage: /market bid <listing number> [amount]");
                return;
            }
            JsonObject payload = new JsonObject();
            payload.addProperty("listing_id", listing);
            payload.addProperty("bidder_uuid", player.getUniqueId().toString());
            payload.addProperty("bidder_name", player.getName());
            if (args.length > 2) {
                try { payload.addProperty("amount", Double.parseDouble(args[2])); } catch (NumberFormatException e) { player.sendMessage(ChatColor.RED + "Bid amount must be a number."); return; }
            }
            operations.enqueue(player, "economy/market/bid", payload, Collections.emptyList(), null);
            return;
        }
        if (args.length >= 2 && args[0].equalsIgnoreCase("cancel")) {
            long listing;
            try { listing = Long.parseLong(args[1].replace("#", "")); } catch (NumberFormatException e) {
                player.sendMessage(ChatColor.RED + "Usage: /market cancel <listing number>");
                return;
            }
            JsonObject payload = new JsonObject();
            payload.addProperty("listing_id", listing);
            payload.addProperty("uuid", player.getUniqueId().toString());
            operations.enqueue(player, "economy/market/cancel", payload, Collections.emptyList(), null);
            return;
        }
        if (args.length >= 1 && (args[0].equalsIgnoreCase("claim") || args[0].equalsIgnoreCase("mailbox"))) {
            claimMailbox(player);
            return;
        }

        // FIX #2: Open Market GUI populated from the real API. All click slots are cancelled.
        Inventory inv = Bukkit.createInventory(null, 54, GUI_MARKET_TITLE);
        ItemStack border = GuiHelper.createBorder(Material.BLUE_STAINED_GLASS_PANE);
        for (int i = 45; i < 54; i++) inv.setItem(i, border);
        inv.setItem(49, GuiHelper.createItem(Material.EMERALD, "&aSell an Item", "&7Hold an item, then click here", "&7for Buy Now or Auction."));
        inv.setItem(47, GuiHelper.createItem(Material.CHEST, "&bMailbox", "&7Won auctions and returned items", "&eClick to collect"));
        inv.setItem(51, GuiHelper.createItem(Material.BOOK, "&fHow it works", "&aBuy Now&7: click to buy instantly.", "&bAuction&7: left-click bids the minimum,", "&7right-click adds 10%. Outbid players", "&7are refunded automatically."));
        ItemStack loading = GuiHelper.createItem(Material.GRAY_STAINED_GLASS_PANE, "&7Loading listings...", "&8Please wait");
        for (int i = 0; i < 45; i++) inv.setItem(i, loading);

        player.openInventory(inv);
        player.sendMessage(ChatColor.GRAY + "Loading market listings...");

        Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
            try {
                JsonArray listings = integration.client().getMarketListings();
                Bukkit.getScheduler().runTask(plugin, () -> {
                    if (player.getOpenInventory().getTopInventory() != inv) return;
                    // Clear loading placeholders
                    for (int i = 0; i < 45; i++) inv.setItem(i, null);
                    Map<Integer, Slot> inventoryListings = new HashMap<>();
                    activeMarketListings.put(inv, inventoryListings);
                    if (listings.size() == 0) {
                        inv.setItem(22, GuiHelper.createItem(Material.BARRIER, "&cNo listings", "&7Be the first to sell something!"));
                        return;
                    }
                    int slot = 0;
                    for (JsonElement el : listings) {
                        if (slot >= 45) break;
                        JsonObject listing = el.getAsJsonObject();
                        long id = listing.has("id") ? listing.get("id").getAsLong() : -1;
                        String sellerName = listing.has("seller_name") ? listing.get("seller_name").getAsString() : "Unknown";
                        // Guild listings show the guild, and are paid into its bank.
                        String guildTag = listing.has("seller_guild") && listing.get("seller_guild").isJsonPrimitive()
                                ? listing.get("seller_guild").getAsString() : null;
                        String listItemId = listing.has("item_id") ? listing.get("item_id").getAsString() : "STONE";
                        String listItemName = listing.has("item_name") ? listing.get("item_name").getAsString() : listItemId;
                        int listAmt = listing.has("amount") ? listing.get("amount").getAsInt() : 1;
                        double listPrice = listing.has("price") ? listing.get("price").getAsDouble() : 0;
                        boolean isAuction = listing.has("kind") && "auction".equals(listing.get("kind").getAsString());
                        double minBid = listing.has("min_next_bid") ? listing.get("min_next_bid").getAsDouble() : listPrice;
                        Material mat;
                        try { mat = Material.valueOf(listItemId); } catch (Exception ex) { mat = Material.PAPER; }
                        inventoryListings.put(slot, new Slot(id, isAuction, minBid));
                        String seller = guildTag != null ? "&7Seller: &6[" + guildTag + "] &fGuild" : "&7Seller: &f" + sellerName;
                        if (isAuction) {
                            boolean hasBids = listing.has("current_bid") && listing.get("current_bid").isJsonPrimitive();
                            int bids = listing.has("bid_count") ? listing.get("bid_count").getAsInt() : 0;
                            inv.setItem(slot, GuiHelper.createItem(mat, listAmt,
                                    "&b[Auction] &f" + listItemName + " &7(x" + listAmt + ")", seller,
                                    hasBids ? "&7Top bid: &a$" + String.format(Locale.US, "%,.2f", listing.get("current_bid").getAsDouble())
                                            + " &7by &f" + (listing.has("bidder_name") && listing.get("bidder_name").isJsonPrimitive() ? listing.get("bidder_name").getAsString() : "?")
                                            + " &8(" + bids + ")" : "&7Starting bid: &a$" + String.format(Locale.US, "%,.2f", listPrice),
                                    "&7Ends in: &e" + timeLeft(listing.has("ends_at") && listing.get("ends_at").isJsonPrimitive() ? listing.get("ends_at").getAsString() : ""),
                                    "&8Listing #" + id, "",
                                    "&eLeft-click: bid &a$" + String.format(Locale.US, "%,.2f", minBid),
                                    "&eRight-click: bid &a$" + String.format(Locale.US, "%,.2f", Math.round(minBid * 1.1 * 100) / 100.0)));
                        } else {
                            inv.setItem(slot, GuiHelper.createItem(mat, listAmt,
                                    "&f" + listItemName + " &7(x" + listAmt + ")", seller,
                                    "&7Price: &a$" + String.format(Locale.US, "%,.2f", listPrice),
                                    "&8Listing #" + id, "", "&eClick to Buy!"));
                        }
                        slot++;
                    }
                });
            } catch (Exception e) {
                player.sendMessage(ChatColor.RED + "Could not load market listings: " + e.getMessage());
            }
        });
    }

    /** Lists the stack the player is holding. Called by the commands and by the sell window. */
    void listHeld(Player player, ItemStack hand, double price, boolean auction, int hours, boolean asGuild) {
        ItemStack listed = hand.clone();
        int amount = listed.getAmount();
        String itemId = listed.getType().name();
        String itemName = listed.hasItemMeta() && listed.getItemMeta().hasDisplayName()
                ? listed.getItemMeta().getDisplayName() : listed.getType().name().replace('_', ' ');
        JsonObject payload = new JsonObject();
        payload.addProperty("seller_uuid", player.getUniqueId().toString());
        payload.addProperty("seller_name", player.getName());
        payload.addProperty("item_id", itemId);
        payload.addProperty("item_name", itemName);
        payload.addProperty("amount", amount);
        payload.addProperty("price", price);
        payload.addProperty("item_data", EconomyOperations.encode(listed));
        payload.addProperty("kind", auction ? "auction" : "buy_now");
        if (auction) payload.addProperty("duration_hours", hours);
        if (asGuild) payload.addProperty("as_guild", true);

        // Escrow the exact stack before persisting a retryable operation.
        player.getInventory().setItemInMainHand(null);
        if (!operations.enqueue(player, "economy/market/list", payload, List.of(listed), null)) {
            giveOrDrop(player, listed);
        }
    }

    void claimMailbox(Player player) {
        JsonObject payload = new JsonObject();
        payload.addProperty("uuid", player.getUniqueId().toString());
        operations.enqueue(player, "economy/market/mailbox/claim", payload, Collections.emptyList(), null);
    }

    /** Tell players with won or returned items waiting that they can collect them. */
    @EventHandler
    public void onJoin(org.bukkit.event.player.PlayerJoinEvent event) {
        Player player = event.getPlayer();
        Bukkit.getScheduler().runTaskLaterAsynchronously(plugin, () -> {
            try {
                JsonObject body = new JsonObject();
                body.addProperty("uuid", player.getUniqueId().toString());
                JsonObject answer = integration.client().post("economy/market/mailbox", body);
                int waiting = answer.has("waiting") ? answer.get("waiting").getAsInt() : 0;
                if (waiting > 0) {
                    player.sendMessage(ChatColor.GOLD + "[Market] " + ChatColor.YELLOW + waiting + " item stack" + (waiting == 1 ? " is" : "s are")
                            + " waiting for you. Collect with " + ChatColor.GREEN + "/market claim" + ChatColor.YELLOW + ".");
                }
            } catch (Exception ignored) { /* the market is optional on join */ }
        }, 60L);
    }

    private static String timeLeft(String iso) {
        try {
            long s = java.time.Duration.between(java.time.Instant.now(), java.time.Instant.parse(iso)).getSeconds();
            if (s <= 0) return "ending";
            if (s >= 86_400) return (s / 86_400) + "d " + ((s % 86_400) / 3600) + "h";
            if (s >= 3600) return (s / 3600) + "h " + ((s % 3600) / 60) + "m";
            return Math.max(1, s / 60) + "m";
        } catch (RuntimeException e) { return "?"; }
    }

    // ---- buy orders and contracts ------------------------------------------------------------------------------------

    private static String text(JsonObject o, String key, String fallback) { return o.has(key) && o.get(key).isJsonPrimitive() ? o.get(key).getAsString() : fallback; }
    private static double number(JsonObject o, String key) { return o.has(key) && o.get(key).isJsonPrimitive() ? o.get(key).getAsDouble() : 0; }
    private static String cash(double v) { return "$" + String.format(Locale.US, "%,.2f", v); }

    /** Run a panel call off the main thread, then answer on it. */
    private void panel(String endpoint, JsonObject body, java.util.function.Consumer<JsonObject> ok, java.util.function.Consumer<String> fail) {
        Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
            try {
                JsonObject answer = integration.client().post(endpoint, body);
                if (plugin.isEnabled()) Bukkit.getScheduler().runTask(plugin, () -> ok.accept(answer));
            } catch (Exception e) {
                if (plugin.isEnabled()) Bukkit.getScheduler().runTask(plugin, () -> fail.accept(e.getMessage() == null ? "the panel is unreachable" : e.getMessage()));
            }
        });
    }

    private JsonObject who(Player player) {
        JsonObject body = new JsonObject();
        body.addProperty("uuid", player.getUniqueId().toString());
        body.addProperty("name", player.getName());
        return body;
    }

    /** Take up to {@code max} plain items (no enchantments, names or other data) of this type from anywhere in the inventory. */
    private List<ItemStack> takeItems(Player player, String panelId, int max) {
        List<ItemStack> taken = new ArrayList<>();
        Material type = Material.matchMaterial(panelId);
        if (type == null || max <= 0) return taken;
        ItemStack[] slots = player.getInventory().getStorageContents();
        int left = max;
        for (int i = 0; i < slots.length && left > 0; i++) {
            ItemStack stack = slots[i];
            if (stack == null || stack.getType() != type || !stack.isSimilar(new ItemStack(type))) continue;
            int n = Math.min(left, stack.getAmount());
            taken.add(new ItemStack(type, n));
            if (n >= stack.getAmount()) player.getInventory().setItem(i, null); else stack.setAmount(stack.getAmount() - n);
            left -= n;
        }
        return taken;
    }

    private void handleOrders(Player player, String[] args) {
        String sub = args.length == 0 ? "list" : args[0].toLowerCase(Locale.ROOT);
        switch (sub) {
            case "request", "create", "new" -> requestOrder(player, args);
            case "pickup", "claim", "take" -> orderAction(player, args, "economy/orders/claim", "/orders pickup <id>");
            case "drop", "release" -> orderAction(player, args, "economy/orders/release", "/orders drop <id>");
            case "cancel" -> orderAction(player, args, "economy/orders/cancel", "/orders cancel <id>");
            case "fill", "submit", "deliver" -> fillOrder(player, args);
            default -> listOrders(player);
        }
    }

    private long idArg(Player player, String[] args, String usage) {
        if (args.length >= 2) {
            try { return Long.parseLong(args[1].replace("#", "")); } catch (NumberFormatException ignored) { /* usage below */ }
        }
        player.sendMessage(ChatColor.RED + "Usage: " + usage);
        return -1;
    }

    private void listOrders(Player player) {
        player.sendMessage(ChatColor.GRAY + "Loading buy orders...");
        panel("economy/orders", who(player), r -> {
            JsonArray list = r.has("orders") && r.get("orders").isJsonArray() ? r.getAsJsonArray("orders") : new JsonArray();
            player.sendMessage(ChatColor.GOLD + "=== Buy Orders ===");
            if (list.size() == 0) player.sendMessage(ChatColor.GRAY + "Nothing is wanted right now. Ask for something with " + ChatColor.YELLOW + "/orders request <amount> <total price> [item]");
            int shown = 0;
            for (JsonElement e : list) {
                if (shown++ >= 15) { player.sendMessage(ChatColor.GRAY + "...and " + (list.size() - 15) + " more in the launcher."); break; }
                JsonObject o = e.getAsJsonObject();
                boolean mine = o.has("mine") && o.get("mine").getAsBoolean(), mineTaken = o.has("claimed_by_me") && o.get("claimed_by_me").getAsBoolean();
                boolean taken = "claimed".equals(text(o, "status", ""));
                String state = mine ? ChatColor.AQUA + "YOURS" : mineTaken ? ChatColor.GREEN + "PICKED UP BY YOU" : taken ? ChatColor.GRAY + "picked up by " + text(o, "claimer_name", "?") : ChatColor.YELLOW + "open";
                player.sendMessage(ChatColor.DARK_GRAY + "#" + (long) number(o, "id") + " " + ChatColor.WHITE + (long) number(o, "amount") + "x " + text(o, "item_name", "?") + ChatColor.GRAY + " for "
                        + ChatColor.GREEN + cash(number(o, "total")) + ChatColor.GRAY + " (" + cash(number(o, "each")) + " each) from " + text(o, "buyer_name", "?") + " · " + state);
            }
            player.sendMessage(ChatColor.GRAY + "Pick one up with " + ChatColor.YELLOW + "/orders pickup <id>" + ChatColor.GRAY + ", hand the items in with " + ChatColor.YELLOW + "/orders fill <id>" + ChatColor.GRAY + ".");
        }, e -> player.sendMessage(ChatColor.RED + "Could not load buy orders: " + e));
    }

    /** {@code /orders request <amount> <total price> [item]}: the item defaults to the one in your hand. */
    private void requestOrder(Player player, String[] args) {
        if (args.length < 3) { player.sendMessage(ChatColor.RED + "Usage: /orders request <amount> <total price> [item]  (hold the item, or name it, e.g. DIAMOND)"); return; }
        int amount;
        double total;
        try { amount = Integer.parseInt(args[1]); total = Double.parseDouble(args[2].replace(",", "")); } catch (NumberFormatException e) { amount = -1; total = -1; }
        if (amount < 1 || !(total >= 0.01) || Double.isInfinite(total)) { player.sendMessage(ChatColor.RED + "Give a whole number of items and a price, like /orders request 64 150 DIAMOND."); return; }
        String item;
        if (args.length >= 4) item = args[3].toUpperCase(Locale.ROOT);
        else {
            ItemStack held = player.getInventory().getItemInMainHand();
            if (held.getType() == Material.AIR) { player.sendMessage(ChatColor.RED + "Hold the item you want, or name it: /orders request <amount> <total> <item>"); return; }
            item = held.getType().name();
        }
        JsonObject body = who(player);
        body.addProperty("item_id", item);
        body.addProperty("amount", amount);
        body.addProperty("total", total);
        body.addProperty("operation_id", UUID.randomUUID().toString());
        panel("economy/orders/create", body, r -> player.sendMessage(ChatColor.GREEN + text(r, "message", "Buy order posted.")), e -> player.sendMessage(ChatColor.RED + "Could not post the order: " + e));
    }

    private void orderAction(Player player, String[] args, String endpoint, String usage) {
        long id = idArg(player, args, usage);
        if (id < 0) return;
        JsonObject body = who(player);
        body.addProperty("order_id", id);
        body.addProperty("operation_id", UUID.randomUUID().toString());
        panel(endpoint, body, r -> player.sendMessage(ChatColor.GREEN + text(r, "message", "Done.")), e -> player.sendMessage(ChatColor.RED + e));
    }

    /** Takes exactly what the order needs from the inventory and hands it in as one safe, retryable job. */
    private void fillOrder(Player player, String[] args) {
        long id = idArg(player, args, "/orders fill <id>");
        if (id < 0) return;
        panel("economy/orders", who(player), r -> {
            JsonObject order = null;
            if (r.has("orders") && r.get("orders").isJsonArray()) for (JsonElement e : r.getAsJsonArray("orders")) if ((long) number(e.getAsJsonObject(), "id") == id) order = e.getAsJsonObject();
            if (order == null) { player.sendMessage(ChatColor.RED + "Buy order #" + id + " isn't on the board any more."); return; }
            String itemId = text(order, "item_id", ""), name = text(order, "item_name", itemId);
            int need = (int) number(order, "amount");
            List<ItemStack> taken = takeItems(player, itemId, need);
            int have = taken.stream().mapToInt(ItemStack::getAmount).sum();
            if (have < need) {
                taken.forEach(item -> giveOrDrop(player, item));
                player.sendMessage(ChatColor.RED + "Order #" + id + " needs " + need + "x " + name + " in your inventory (plain items, no enchants or names). You have " + have + ".");
                return;
            }
            JsonObject payload = who(player);
            payload.addProperty("order_id", id);
            payload.addProperty("item_id", itemId);
            payload.addProperty("amount", have);
            if (!operations.enqueue(player, "economy/orders/fill", payload, taken, null)) taken.forEach(item -> giveOrDrop(player, item));
        }, e -> player.sendMessage(ChatColor.RED + "Could not load the order: " + e));
    }

    private void handleContracts(Player player, String[] args) {
        String sub = args.length == 0 ? "list" : args[0].toLowerCase(Locale.ROOT);
        switch (sub) {
            case "submit", "deliver", "turnin" -> submitContracts(player, args);
            case "abandon", "swap", "reroll" -> {
                long id = idArg(player, args, "/contracts abandon <id>");
                if (id < 0) return;
                JsonObject body = new JsonObject();
                body.addProperty("uuid", player.getUniqueId().toString());
                body.addProperty("contract_id", id);
                panel("economy/contracts/abandon", body, r -> { player.sendMessage(ChatColor.GREEN + "Contract swapped for a new one."); listContracts(player); }, e -> player.sendMessage(ChatColor.RED + e));
            }
            default -> listContracts(player);
        }
    }

    private void listContracts(Player player) {
        player.sendMessage(ChatColor.GRAY + "Loading your contracts...");
        panel("economy/contracts", who(player), r -> {
            if (r.has("enabled") && !r.get("enabled").getAsBoolean()) { player.sendMessage(ChatColor.RED + "Contracts are switched off on this server."); return; }
            player.sendMessage(ChatColor.GOLD + "=== Your Contracts ===");
            JsonArray list = r.has("contracts") && r.get("contracts").isJsonArray() ? r.getAsJsonArray("contracts") : new JsonArray();
            if (list.size() == 0) player.sendMessage(ChatColor.GRAY + "No contracts right now. New ones appear as you finish these.");
            for (JsonElement e : list) {
                JsonObject c = e.getAsJsonObject();
                boolean gather = "gather".equals(text(c, "kind", "")), hot = c.has("bonus") && c.get("bonus").getAsBoolean();
                player.sendMessage(ChatColor.DARK_GRAY + "#" + (long) number(c, "id") + " " + (hot ? ChatColor.AQUA + "HOT " : "") + ChatColor.WHITE + text(c, "title", "?") + ChatColor.GRAY + " " + (long) number(c, "progress") + "/" + (long) number(c, "required")
                        + " · pays " + ChatColor.GREEN + cash(number(c, "reward")) + ChatColor.GRAY + (gather ? " · /contracts submit" : " · counts automatically"));
            }
            JsonObject s = r.has("stats") && r.get("stats").isJsonObject() ? r.getAsJsonObject("stats") : new JsonObject();
            player.sendMessage(ChatColor.GRAY + "Finished today: " + (long) number(s, "done_today") + (number(s, "daily_limit") > 0 ? "/" + (long) number(s, "daily_limit") : "") + " · swaps left: " + (long) number(s, "rerolls_left") + " (" + ChatColor.YELLOW + "/contracts abandon <id>" + ChatColor.GRAY + ")");
        }, e -> player.sendMessage(ChatColor.RED + "Could not load contracts: " + e));
    }

    /** Hands in whatever the resource contracts still need from the inventory, one safe job per contract. */
    private void submitContracts(Player player, String[] args) {
        long only = args.length >= 2 ? idArg(player, args, "/contracts submit [id]") : 0;
        if (args.length >= 2 && only < 0) return;
        panel("economy/contracts", who(player), r -> {
            JsonArray list = r.has("contracts") && r.get("contracts").isJsonArray() ? r.getAsJsonArray("contracts") : new JsonArray();
            int queued = 0;
            for (JsonElement e : list) {
                JsonObject c = e.getAsJsonObject();
                if (!"gather".equals(text(c, "kind", "")) || (only > 0 && (long) number(c, "id") != only)) continue;
                int need = (int) (number(c, "required") - number(c, "progress"));
                if (need <= 0) continue;
                List<ItemStack> taken = takeItems(player, text(c, "target", ""), need);
                int have = taken.stream().mapToInt(ItemStack::getAmount).sum();
                if (have <= 0) continue;
                JsonObject payload = who(player);
                payload.addProperty("contract_id", (long) number(c, "id"));
                payload.addProperty("item_id", text(c, "target", ""));
                payload.addProperty("amount", have);
                if (operations.enqueue(player, "economy/contracts/submit", payload, taken, null)) queued++; else taken.forEach(item -> giveOrDrop(player, item));
            }
            if (queued == 0) player.sendMessage(ChatColor.RED + "Nothing in your inventory counts toward your resource contracts (plain items only). See " + ChatColor.YELLOW + "/contracts" + ChatColor.RED + ".");
        }, e -> player.sendMessage(ChatColor.RED + "Could not load contracts: " + e));
    }

    private void handleTransactions(Player player) {
        player.sendMessage(ChatColor.GOLD + "=== Recent Transactions ===");
        player.sendMessage(ChatColor.GRAY + "Loading transaction history...");
        Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
            try {
                double bal = integration.client().getBalance(player.getUniqueId(), player.getName());
                player.sendMessage(ChatColor.GREEN + "Current Balance: " + ChatColor.YELLOW + "$" + String.format(Locale.US, "%,.2f", bal));
                player.sendMessage(ChatColor.GRAY + "For full transaction logs and charts, check the Velora Launcher Stats page!");
            } catch (Exception e) {
                player.sendMessage(ChatColor.RED + "Error: " + e.getMessage());
            }
        });
    }

    private void handleTrade(Player player, String[] args) {
        if (args.length < 1) {
            player.sendMessage(ChatColor.RED + "Usage: /trade <player>");
            return;
        }

        Player target = Bukkit.getPlayer(args[0]);
        if (target == null || !target.isOnline()) {
            player.sendMessage(ChatColor.RED + "Player '" + args[0] + "' is not online.");
            return;
        }

        if (target.getUniqueId().equals(player.getUniqueId())) {
            player.sendMessage(ChatColor.RED + "You cannot trade with yourself.");
            return;
        }

        if (activeTrades.containsKey(player.getUniqueId()) || activeTrades.containsKey(target.getUniqueId())) {
            player.sendMessage(ChatColor.RED + "Either you or the other player is already in a trade.");
            return;
        }

        // FIX #3: Use the fixed TradeSession that restricts each side and transfers items.
        TradeSession session = new TradeSession(plugin, integration, player, target);
        activeTrades.put(player.getUniqueId(), session);
        activeTrades.put(target.getUniqueId(), session);
        session.open();
    }

    @EventHandler
    public void onInventoryClick(InventoryClickEvent event) {
        if (!(event.getWhoClicked() instanceof Player player)) return;
        if (event.getView().getTopInventory().getHolder() instanceof MarketWizard.Holder wizardState) {
            event.setCancelled(true);
            if (event.getClickedInventory() == event.getView().getTopInventory()) wizard.click(player, wizardState, event.getRawSlot());
            return;
        }
        String title = event.getView().getTitle();

        if (title.equals(GUI_SHOP_TITLE)) {
            // FIX #2/#4: Always cancel – items cannot be taken from the shop GUI.
            event.setCancelled(true);
            ItemStack clicked = event.getCurrentItem();
            if (clicked == null || clicked.getType().isAir()
                    || !clicked.hasItemMeta() || clicked.getItemMeta().getLore() == null) return;

            // Extract price from lore
            double price = -1;
            for (String loreLine : clicked.getItemMeta().getLore()) {
                String raw = ChatColor.stripColor(loreLine);
                if (raw.startsWith("Price: $")) {
                    try {
                        price = Double.parseDouble(raw.replace("Price: $", "").replace(",", "").trim());
                    } catch (Exception ignored) {}
                }
            }

            if (price <= 0) return;
            ItemStack purchase = new ItemStack(clicked.getType(), clicked.getAmount());
            JsonObject payload = new JsonObject();
            payload.addProperty("uuid", player.getUniqueId().toString());
            payload.addProperty("username", player.getName());
            payload.addProperty("delta", -price);
            payload.addProperty("description", "Shop purchase: " + purchase.getType().name());
            operations.enqueue(player, "economy/adjust", payload, Collections.emptyList(), purchase);
        } else if (title.equals(GUI_MARKET_TITLE)) {
            // FIX #2: Always cancel market clicks – purchase only via the API.
            event.setCancelled(true);
            int slot = event.getRawSlot();
            if (slot == 47) { player.closeInventory(); claimMailbox(player); return; }
            if (slot == 49) {
                ItemStack hand = player.getInventory().getItemInMainHand();
                player.closeInventory();
                if (hand.getType().isAir()) player.sendMessage(ChatColor.RED + "Hold the item you want to sell in your main hand first.");
                else if (!player.hasPermission("scopenet.command.market.sell")) player.sendMessage(ChatColor.RED + "You do not have permission to list items on the market.");
                else wizard.open(player, hand, false, 10, false);
                return;
            }
            Slot listing = activeMarketListings.getOrDefault(event.getInventory(), Collections.emptyMap()).get(slot);
            if (listing == null || listing.id() < 0) return;
            if (!player.hasPermission("scopenet.command.market.buy")) {
                player.sendMessage(ChatColor.RED + "You do not have permission to buy from the market.");
                return;
            }

            JsonObject payload = new JsonObject();
            payload.addProperty("listing_id", listing.id());
            if (listing.auction()) {
                payload.addProperty("bidder_uuid", player.getUniqueId().toString());
                payload.addProperty("bidder_name", player.getName());
                double amount = event.isRightClick() ? Math.round(listing.minBid() * 1.1 * 100) / 100.0 : listing.minBid();
                payload.addProperty("amount", amount);
                operations.enqueue(player, "economy/market/bid", payload, Collections.emptyList(), null);
            } else {
                payload.addProperty("buyer_uuid", player.getUniqueId().toString());
                payload.addProperty("buyer_name", player.getName());
                operations.enqueue(player, "economy/market/buy", payload, Collections.emptyList(), null);
            }
        } else if (title.startsWith(GUI_TRADE_PREFIX)) {
            TradeSession session = activeTrades.get(player.getUniqueId());
            if (session != null) {
                session.handleClick(player, event);
            }
        }
    }

    @EventHandler
    public void onInventoryClose(InventoryCloseEvent event) {
        if (!(event.getPlayer() instanceof Player player)) return;
        String title = event.getView().getTitle();

        if (title.equals(GUI_MARKET_TITLE)) {
            activeMarketListings.remove(event.getInventory());
        }

        if (title.equals(GUI_SELL_TITLE)) {
            // Move sellable items to durable escrow before crediting the balance.
            Inventory inv = event.getInventory();
            boolean toGuild = guildSellChests.remove(inv);
            List<ItemStack> soldItems = new ArrayList<>();
            List<ItemStack> unsellable = new ArrayList<>();
            double totalEarned = 0;
            int countSold = 0;

            for (ItemStack item : inv.getContents()) {
                if (item == null || item.getType().isAir()) continue;
                Double price = ITEM_SELL_VALUES.get(item.getType());
                if (price != null) {
                    totalEarned += price * item.getAmount();
                    countSold += item.getAmount();
                    soldItems.add(item.clone());
                } else {
                    unsellable.add(item.clone());
                }
            }
            inv.clear();

            // Return unsellable items immediately
            for (ItemStack it : unsellable) giveOrDrop(player, it);

            if (totalEarned > 0) {
                JsonObject payload = new JsonObject();
                payload.addProperty("uuid", player.getUniqueId().toString());
                payload.addProperty("username", player.getName());
                payload.addProperty(toGuild ? "amount" : "delta", totalEarned);
                payload.addProperty("description", "Sell chest: " + countSold + " items");
                if (!operations.enqueue(player, toGuild ? "guilds/bank/credit" : "economy/adjust", payload, soldItems, null)) {
                    for (ItemStack item : soldItems) giveOrDrop(player, item);
                }
            }
        } else if (title.startsWith(GUI_TRADE_PREFIX)) {
            TradeSession session = activeTrades.get(player.getUniqueId());
            if (session != null) {
                // Only cancel if both players have closed
                session.handleClose(player, activeTrades);
            }
        }
    }

    /**
     * FIX #3: Trade session that:
     * - Restricts each player to their own side of the shared inventory.
     * - Transfers items between inventories on completion.
     * - Returns items if the trade is cancelled.
     *
     * Layout: slots 0-3,9-12,18-21,27-30,36-39 = P1's offer side
     *         slots 5-8,14-17,23-26,32-35,41-44 = P2's offer side
     *         slot 4,13,22,31,40,49 = dividers (not interactable)
     *         slot 0 = P1 ready toggle, slot 8 = P2 ready toggle
     */
    private static class TradeSession {
        private final ScopenetPlugin plugin;
        private final Integration integration;
        final Player p1;
        final Player p2;
        final Inventory inv;
        boolean p1Ready = false;
        boolean p2Ready = false;
        boolean completed = false;
        boolean cancelled = false;

        // Slots belonging to each side (excluding ready buttons at 0 and 8)
        private static final Set<Integer> P1_SLOTS = new HashSet<>(Arrays.asList(
                1, 2, 3, 9, 10, 11, 12, 18, 19, 20, 21, 27, 28, 29, 30, 36, 37, 38, 39));
        private static final Set<Integer> P2_SLOTS = new HashSet<>(Arrays.asList(
                5, 6, 7, 14, 15, 16, 17, 23, 24, 25, 26, 32, 33, 34, 35, 41, 42, 43, 44));
        private static final Set<Integer> DIVIDERS = new HashSet<>(Arrays.asList(4, 13, 22, 31, 40, 49));

        TradeSession(ScopenetPlugin plugin, Integration integration, Player p1, Player p2) {
            this.plugin = plugin;
            this.integration = integration;
            this.p1 = p1;
            this.p2 = p2;
            this.inv = Bukkit.createInventory(null, 54, GUI_TRADE_PREFIX + p1.getName() + " & " + p2.getName());
            buildDividers();
        }

        void buildDividers() {
            ItemStack divider = GuiHelper.createBorder(Material.GRAY_STAINED_GLASS_PANE);
            for (int s : DIVIDERS) inv.setItem(s, divider);
            updateReadyButtons();
        }

        void updateReadyButtons() {
            inv.setItem(0, GuiHelper.createItem(p1Ready ? Material.LIME_CONCRETE : Material.RED_CONCRETE,
                    p1.getName() + ": " + (p1Ready ? "&aREADY" : "&cNOT READY"), "&7Click to toggle ready status."));
            inv.setItem(8, GuiHelper.createItem(p2Ready ? Material.LIME_CONCRETE : Material.RED_CONCRETE,
                    p2.getName() + ": " + (p2Ready ? "&aREADY" : "&cNOT READY"), "&7Click to toggle ready status."));
        }

        void open() {
            p1.openInventory(inv);
            p2.openInventory(inv);
        }

        void handleClick(Player clicker, InventoryClickEvent event) {
            if (completed || cancelled) { event.setCancelled(true); return; }

            int slot = event.getRawSlot();
            boolean isP1 = clicker.getUniqueId().equals(p1.getUniqueId());

            // Ready toggle buttons
            if (slot == 0 && isP1) {
                event.setCancelled(true);
                p1Ready = !p1Ready;
                updateReadyButtons();
                checkCompletion();
                return;
            }
            if (slot == 8 && !isP1) {
                event.setCancelled(true);
                p2Ready = !p2Ready;
                updateReadyButtons();
                checkCompletion();
                return;
            }

            // Dividers and ready buttons on the wrong side are always blocked
            if (DIVIDERS.contains(slot) || slot == 0 || slot == 8) {
                event.setCancelled(true);
                return;
            }

            // FIX #3: Each player can only interact with their own offer side.
            if (isP1 && P2_SLOTS.contains(slot)) { event.setCancelled(true); return; }
            if (!isP1 && P1_SLOTS.contains(slot)) { event.setCancelled(true); return; }

            // If a player touches their side after being ready, reset ready state.
            if (isP1 && p1Ready) { p1Ready = false; updateReadyButtons(); }
            if (!isP1 && p2Ready) { p2Ready = false; updateReadyButtons(); }
        }

        void checkCompletion() {
            if (!p1Ready || !p2Ready || completed || cancelled) return;
            completed = true;

            // FIX #3: Actually transfer items between players.
            // Collect each side's offered items.
            List<ItemStack> p1Offer = new ArrayList<>();
            List<ItemStack> p2Offer = new ArrayList<>();
            for (int s : P1_SLOTS) {
                ItemStack it = inv.getItem(s);
                if (it != null && !it.getType().isAir()) p1Offer.add(it.clone());
            }
            for (int s : P2_SLOTS) {
                ItemStack it = inv.getItem(s);
                if (it != null && !it.getType().isAir()) p2Offer.add(it.clone());
            }

            p1.closeInventory();
            p2.closeInventory();

            // Give each player the other's offered items on the main thread.
            Bukkit.getScheduler().runTask(plugin, () -> {
                for (ItemStack it : p2Offer) p1.getInventory().addItem(it);
                for (ItemStack it : p1Offer) p2.getInventory().addItem(it);
                p1.sendMessage(ChatColor.GREEN + "Trade completed successfully!");
                p2.sendMessage(ChatColor.GREEN + "Trade completed successfully!");
            });
        }

        void handleClose(Player closer, Map<UUID, TradeSession> activeTrades) {
            if (completed) {
                activeTrades.remove(p1.getUniqueId());
                activeTrades.remove(p2.getUniqueId());
                return;
            }
            if (cancelled) return;
            cancelled = true;
            activeTrades.remove(p1.getUniqueId());
            activeTrades.remove(p2.getUniqueId());

            // FIX #3: Return items to their respective owners.
            Bukkit.getScheduler().runTask(plugin, () -> {
                for (int s : P1_SLOTS) {
                    ItemStack it = inv.getItem(s);
                    if (it != null && !it.getType().isAir()) p1.getInventory().addItem(it.clone());
                }
                for (int s : P2_SLOTS) {
                    ItemStack it = inv.getItem(s);
                    if (it != null && !it.getType().isAir()) p2.getInventory().addItem(it.clone());
                }
                if (p1.isOnline()) {
                    p1.closeInventory();
                    p1.sendMessage(ChatColor.RED + "Trade cancelled. Your items have been returned.");
                }
                if (p2.isOnline()) {
                    p2.closeInventory();
                    p2.sendMessage(ChatColor.RED + "Trade cancelled. Your items have been returned.");
                }
            });
        }
    }
}
