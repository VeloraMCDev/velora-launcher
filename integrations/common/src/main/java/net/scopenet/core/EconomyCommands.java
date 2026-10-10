package net.scopenet.core;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

import java.util.*;

/**
 * /balance /pay /baltop /shop /sell /market /orders /transactions, in chat. Money lives in the panel; selling and buying
 * go through {@link Jobs} so a crash or outage never loses an item or a payment. Shop, sell-chest and market GUIs
 * are the client mod's job; the commands here are what it (and plain chat) use.
 */
final class EconomyCommands {
    private final Env env;
    private final Jobs jobs;
    private final ShopPoints shops;

    EconomyCommands(Env env, Jobs jobs, ShopPoints shops) { this.env = env; this.jobs = jobs; this.shops = shops; }

    List<CoreCommand> build() {
        List<CoreCommand> out = new ArrayList<>();
        out.add(Cmd.of("balance", List.of("bal", "money"), (p, a) -> balance(p)));
        out.add(Cmd.of("pay", List.of(), this::pay).completing((p, a) -> a.length <= 1 ? EssentialsCommands.names(env, p) : List.of()));
        out.add(Cmd.of("baltop", List.of("richest"), (p, a) -> baltop(p)));
        out.add(Cmd.of("shop", List.of(), (p, a) -> shop(p)));
        out.add(Cmd.of("darknet", List.of(), this::darknet));
        out.add(Cmd.of("sell", List.of(), (p, a) -> sell(p, a, false)).completing((p, a) -> a.length <= 1 ? List.of("hand") : List.of()));
        out.add(Cmd.of("market", List.of("ah", "auction"), (p, a) -> market(p, a, false)).completing((p, a) -> a.length <= 1 ? List.of("sell", "auction", "bid", "buy", "cancel", "claim", "point") : a.length == 2 && a[0].equalsIgnoreCase("point") ? List.of("add", "remove", "list") : List.of()));
        out.add(Cmd.of("trade", List.of(), (p, a) -> p.send(Format.RED + "/trade needs the chest window, which isn't available on this platform yet. Use /market or /pay.")).completing((p, a) -> a.length == 1 ? EssentialsCommands.names(env, p) : List.of()));
        out.add(Cmd.of("transactions", List.of(), (p, a) -> transactions(p)));
        return out;
    }

    private void darknet(CorePlayer p, String[] args) {
        JsonObject body = who(p);
        if (args.length == 0) {
            env.io(() -> env.panel.call("economy/darknet", body).getAsJsonObject(), catalog -> {
                p.send(Format.GOLD + "=== Darknet ===");
                JsonArray products = catalog.getAsJsonArray("products");
                if (products == null || products.isEmpty()) { p.send(Format.GRAY + "The catalog is empty."); return; }
                for (JsonElement value : products) {
                    JsonObject product = value.getAsJsonObject();
                    p.send(Format.YELLOW + product.get("id").getAsString() + Format.WHITE + " · " + product.get("amount").getAsInt() + "x "
                            + product.get("item_name").getAsString() + " · " + env.money(product.get("price_cents").getAsLong() / 100.0));
                }
                p.send(Format.GRAY + "Purchase with /darknet buy <product>. Items go to your vault.");
            }, error -> p.send(Format.RED + error));
            return;
        }
        if (args.length != 2 || !args[0].equalsIgnoreCase("buy")) { p.send(Format.RED + "Usage: /darknet [buy <product>]"); return; }
        body.addProperty("product_id", args[1]); body.addProperty("operation_id", UUID.randomUUID().toString());
        env.io(() -> env.panel.call("economy/darknet/buy", body), result -> p.send(Format.GREEN + "Purchased. Delivery is queued for your /vault."), error -> p.send(Format.RED + error));
    }

    private JsonObject who(CorePlayer p) {
        JsonObject o = new JsonObject();
        o.addProperty("uuid", p.uuid().toString());
        o.addProperty("username", p.name());
        return o;
    }

    void balance(CorePlayer p) {
        p.send(Format.GRAY + "Fetching your balance...");
        env.io(() -> env.panel.call("economy/balance", who(p)).getAsJsonObject(), r -> {
            double bal = r.has("balance") ? r.get("balance").getAsDouble() : 0.0;
            p.send(Format.GOLD + "=== Velora Economy ===");
            p.send(Format.YELLOW + "Your Balance: " + Format.GREEN + env.money(bal));
        }, e -> p.send(Format.RED + "Could not reach economy server: " + e));
    }

    void pay(CorePlayer p, String[] args) {
        if (args.length < 2) { p.send(Format.RED + "Usage: /pay <player> <amount>"); return; }
        Optional<CorePlayer> found = env.platform.playerByName(args[0]);
        if (found.isEmpty()) { p.send(Format.RED + "Player '" + args[0] + "' is not online."); return; }
        CorePlayer target = found.get();
        if (target.uuid().equals(p.uuid())) { p.send(Format.RED + "You cannot pay yourself."); return; }
        Double amount;
        try { amount = Double.parseDouble(args[1]); } catch (NumberFormatException e) { amount = null; }
        if (amount == null || amount <= 0 || Double.isNaN(amount) || Double.isInfinite(amount)) {
            p.send(Format.RED + "Invalid amount specified. Must be positive number.");
            return;
        }
        final double value = amount;
        p.send(Format.GRAY + "Processing payment...");
        JsonObject body = new JsonObject();
        body.addProperty("from_uuid", p.uuid().toString());
        body.addProperty("from_name", p.name());
        body.addProperty("to_uuid", target.uuid().toString());
        body.addProperty("to_name", target.name());
        body.addProperty("amount", value);
        body.addProperty("description", "Player payment to " + target.name());
        env.io(() -> env.panel.call("economy/transfer", body).getAsJsonObject(), r -> {
            double left = r.has("from_balance") ? r.get("from_balance").getAsDouble() : 0.0;
            p.send(Format.GREEN + "Sent " + Format.YELLOW + env.money(value) + Format.GREEN + " to " + Format.YELLOW + target.name()
                    + Format.GREEN + ". New Balance: " + Format.YELLOW + env.money(left));
            target.send(Format.GREEN + "Received " + Format.YELLOW + env.money(value) + Format.GREEN + " from " + Format.YELLOW + p.name() + Format.GREEN + "!");
        }, e -> p.send(Format.RED + "Transfer failed: " + e));
    }

    void baltop(CorePlayer p) {
        p.send(Format.GRAY + "Loading leaderboard...");
        env.io(() -> env.panel.call("economy/baltop", new JsonObject()), el -> {
            JsonArray list = el.isJsonArray() ? el.getAsJsonArray() : new JsonArray();
            p.send(Format.GOLD + "================= " + Format.YELLOW + "BALTOP (Richest Players)" + Format.GOLD + " =================");
            if (list.size() == 0) { p.send(Format.GRAY + "No players found on the leaderboard."); return; }
            for (int i = 0; i < Math.min(10, list.size()); i++) {
                JsonObject e = list.get(i).getAsJsonObject();
                int rank = e.has("rank") ? e.get("rank").getAsInt() : i + 1;
                String name = e.has("username") ? e.get("username").getAsString() : "Unknown";
                double bal = e.has("balance") ? e.get("balance").getAsDouble() : 0.0;
                String colour = rank == 1 ? Format.GOLD : rank == 2 ? Format.WHITE : rank == 3 ? Format.YELLOW : Format.GRAY;
                p.send(colour + "#" + rank + " " + Format.WHITE + name + ": " + Format.GREEN + env.money(bal));
            }
        }, e -> p.send(Format.RED + "Could not fetch baltop: " + e));
    }

    void shop(CorePlayer p) {
        p.send(Format.GOLD + "=== Server Shop (sell prices) ===");
        ItemValues.all().forEach((id, price) -> p.send(Format.GRAY + " " + id.toLowerCase(Locale.ROOT).replace('_', ' ') + Format.DARK_GRAY + " - " + Format.GREEN + env.money(price)));
        p.send(Format.GRAY + "Hold items and use " + Format.YELLOW + "/sell hand" + Format.GRAY + " to sell them.");
    }

    /** {@code guild}: the proceeds go to the player's guild bank. */
    void sell(CorePlayer p, String[] args, boolean guild) {
        if (args.length == 0 || !args[0].equalsIgnoreCase("hand")) {
            p.send(Format.YELLOW + "Hold the items you want to sell and use /" + (guild ? "guild sell" : "sell") + " hand." + Format.GRAY + " See prices with /shop.");
            return;
        }
        if (!guild && !p.hasPermission("scopenet.command.sell.hand")) { p.send(Format.RED + "You do not have permission to use /sell hand."); return; }
        Optional<Item> held = p.heldItem();
        if (held.isEmpty()) { p.send(Format.RED + "You are not holding any item to sell."); return; }
        Item item = held.get();
        Double unit = ItemValues.unitPrice(item.id());
        if (unit == null) { p.send(Format.RED + "This item (" + item.id() + ") cannot be sold to the server shop."); return; }
        JsonObject payload = who(p);
        payload.addProperty(guild ? "amount" : "delta", unit * item.count());
        payload.addProperty("description", "Sell hand: " + item.id());
        p.clearHeld();
        if (!jobs.enqueue(p, guild ? "guilds/bank/credit" : "economy/adjust", payload, List.of(item), null)) p.give(item);
    }

    /** {@code asGuild}: list on behalf of the player's guild, or buy with its bank. */
    void market(CorePlayer p, String[] args, boolean asGuild) {
        if (!asGuild && args.length >= 1 && args[0].equalsIgnoreCase("point")) {
            // The block under the player's feet: the platform-neutral stand-in for "the block you look at".
            Pos at = p.pos();
            shops.command(p, Arrays.copyOfRange(args, 1, args.length), new Pos(at.world(), at.x(), at.y() - 1, at.z(), 0, 0));
            return;
        }
        if (args.length >= 2 && args[0].equalsIgnoreCase("buy")) {
            if (!asGuild && !p.hasPermission("scopenet.command.market.buy")) { p.send(Format.RED + "You do not have permission to buy from the market."); return; }
            long listing;
            try { listing = Long.parseLong(args[1].replace("#", "")); } catch (NumberFormatException e) {
                p.send(Format.RED + "Usage: /" + (asGuild ? "guild market" : "market") + " buy <listing number> (shown as #n in /market)");
                return;
            }
            JsonObject payload = new JsonObject();
            payload.addProperty("listing_id", listing);
            payload.addProperty("buyer_uuid", p.uuid().toString());
            payload.addProperty("buyer_name", p.name());
            if (asGuild) payload.addProperty("as_guild", true);
            jobs.enqueue(p, "economy/market/buy", payload, List.of(), null);
            return;
        }
        if (args.length >= 2 && (args[0].equalsIgnoreCase("sell") || args[0].equalsIgnoreCase("auction"))) {
            sellOrAuction(p, args, asGuild, args[0].equalsIgnoreCase("auction"));
            return;
        }
        if (args.length >= 2 && args[0].equalsIgnoreCase("bid")) {
            long listing = listingNumber(p, args[1], "/market bid <listing number> [amount]");
            if (listing < 0) return;
            JsonObject payload = new JsonObject();
            payload.addProperty("listing_id", listing);
            payload.addProperty("bidder_uuid", p.uuid().toString());
            payload.addProperty("bidder_name", p.name());
            if (args.length > 2) {
                try { payload.addProperty("amount", Double.parseDouble(args[2])); } catch (NumberFormatException e) { p.send(Format.RED + "Bid amount must be a number."); return; }
            }
            jobs.enqueue(p, "economy/market/bid", payload, List.of(), null);
            return;
        }
        if (args.length >= 2 && args[0].equalsIgnoreCase("cancel")) {
            long listing = listingNumber(p, args[1], "/market cancel <listing number>");
            if (listing < 0) return;
            JsonObject payload = new JsonObject();
            payload.addProperty("listing_id", listing);
            payload.addProperty("uuid", p.uuid().toString());
            jobs.enqueue(p, "economy/market/cancel", payload, List.of(), null);
            return;
        }
        if (args.length >= 1 && (args[0].equalsIgnoreCase("claim") || args[0].equalsIgnoreCase("mailbox"))) {
            JsonObject payload = new JsonObject();
            payload.addProperty("uuid", p.uuid().toString());
            jobs.enqueue(p, "economy/market/mailbox/claim", payload, List.of(), null);
            return;
        }
        listMarket(p);
    }

    private long listingNumber(CorePlayer p, String text, String usage) {
        try { return Long.parseLong(text.replace("#", "")); } catch (NumberFormatException e) {
            p.send(Format.RED + "Usage: " + usage + " (listing numbers are shown as #n in /market)");
            return -1;
        }
    }

    /** {@code /market sell <price>} lists the held stack for an instant sale; {@code /market auction <start> [hours]} starts an auction. */
    private void sellOrAuction(CorePlayer p, String[] args, boolean asGuild, boolean auction) {
        String word = auction ? "auction" : "sell";
        if (!asGuild && !p.hasPermission("scopenet.command.market.sell")) { p.send(Format.RED + "You do not have permission to list items on the market."); return; }
        Optional<Item> held = p.heldItem();
        if (held.isEmpty()) { p.send(Format.RED + "Hold the item you want to " + word + " in your main hand."); return; }
        Double price;
        try { price = Double.parseDouble(args[1]); } catch (NumberFormatException e) { price = null; }
        if (price == null || price <= 0 || price.isNaN() || price.isInfinite()) {
            p.send(Format.RED + "Invalid price. Usage: /market " + word + " <" + (auction ? "starting bid" : "price") + ">" + (auction ? " [hours]" : ""));
            return;
        }
        int hours = 24;
        if (auction && args.length > 2) {
            try { hours = Integer.parseInt(args[2]); } catch (NumberFormatException e) { hours = -1; }
            if (hours < 1 || hours > 168) { p.send(Format.RED + "Auctions run for 1 to 168 hours."); return; }
        }
        Item item = held.get();
        JsonObject payload = new JsonObject();
        payload.addProperty("seller_uuid", p.uuid().toString());
        payload.addProperty("seller_name", p.name());
        payload.addProperty("item_id", item.id());
        payload.addProperty("item_name", item.name());
        payload.addProperty("amount", item.count());
        payload.addProperty("price", price);
        payload.addProperty("item_data", item.data());
        payload.addProperty("kind", auction ? "auction" : "buy_now");
        if (auction) payload.addProperty("duration_hours", hours);
        if (asGuild) payload.addProperty("as_guild", true);
        p.clearHeld(); // escrow first, then a retryable operation
        if (!jobs.enqueue(p, "economy/market/list", payload, List.of(item), null)) p.give(item);
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

    void listMarket(CorePlayer p) {
        p.send(Format.GRAY + "Loading market listings...");
        env.io(() -> env.panel.call("economy/market", new JsonObject()), el -> {
            JsonArray list = el.isJsonArray() ? el.getAsJsonArray() : new JsonArray();
            p.send(Format.GOLD + "=== Player Market ===");
            if (list.size() == 0) { p.send(Format.GRAY + "No listings. Be the first: hold an item and use /market sell <price> or /market auction <starting bid> [hours]."); return; }
            for (JsonElement e : list) {
                JsonObject l = e.getAsJsonObject();
                String guild = l.has("seller_guild") && l.get("seller_guild").isJsonPrimitive() ? l.get("seller_guild").getAsString() : null;
                String seller = guild != null ? Format.GOLD + "[" + guild + "] guild" : Format.WHITE + l.get("seller_name").getAsString();
                boolean auction = l.has("kind") && "auction".equals(l.get("kind").getAsString());
                String head = Format.DARK_GRAY + "#" + l.get("id").getAsLong() + " " + Format.WHITE + l.get("item_name").getAsString() + Format.GRAY + " x" + l.get("amount").getAsInt() + " ";
                if (auction) {
                    boolean bids = l.has("current_bid") && l.get("current_bid").isJsonPrimitive();
                    double next = l.has("min_next_bid") ? l.get("min_next_bid").getAsDouble() : l.get("price").getAsDouble();
                    p.send(head + Format.AQUA + "AUCTION " + Format.GREEN + (bids ? env.money(l.get("current_bid").getAsDouble()) + Format.GRAY + " (" + l.get("bid_count").getAsInt() + " bid" + (l.get("bid_count").getAsInt() == 1 ? "" : "s") + ")"
                            : env.money(l.get("price").getAsDouble()) + Format.GRAY + " start")
                            + Format.GRAY + " · ends in " + timeLeft(l.has("ends_at") ? l.get("ends_at").getAsString() : "") + " · next bid " + env.money(next) + " by " + seller);
                } else {
                    p.send(head + Format.GREEN + env.money(l.get("price").getAsDouble()) + Format.GRAY + " by " + seller);
                }
            }
            p.send(Format.GRAY + "Buy with " + Format.YELLOW + "/market buy <#id>" + Format.GRAY + " · bid with " + Format.YELLOW + "/market bid <#id> [amount]" + Format.GRAY + " · collect wins with " + Format.YELLOW + "/market claim");
        }, e -> p.send(Format.RED + "Could not load market listings: " + e));
    }

    void orders(CorePlayer p) {
        p.send(Format.GOLD + "=== Your Active Market Orders ===");
        p.send(Format.GRAY + "Use " + Format.YELLOW + "/market sell <price>" + Format.GRAY + " to list items, or browse with " + Format.YELLOW + "/market" + Format.GRAY + ".");
    }

    void transactions(CorePlayer p) {
        p.send(Format.GOLD + "=== Recent Transactions ===");
        env.io(() -> env.panel.call("economy/balance", who(p)).getAsJsonObject(), r -> {
            double bal = r.has("balance") ? r.get("balance").getAsDouble() : 0.0;
            p.send(Format.GREEN + "Current Balance: " + Format.YELLOW + env.money(bal));
            p.send(Format.GRAY + "For full transaction logs and charts, check the Velora Launcher Stats page!");
        }, e -> p.send(Format.RED + "Error: " + e));
    }
}
