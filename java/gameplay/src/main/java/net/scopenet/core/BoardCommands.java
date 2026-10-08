package net.scopenet.core;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

import java.util.*;

/**
 * /orders and /contracts. A buy order is a request for items with the money held in escrow; anyone can pick one up from the
 * board and hand the items in, and the items go to the buyer's vault while the escrow is released. A contract is a randomly
 * generated task paid in money: kills count by themselves, and resource contracts need the items handed in. Handing items in
 * goes through {@link Jobs}, so a crash or an outage never loses them.
 */
final class BoardCommands {
    private final Env env;
    private final Jobs jobs;

    BoardCommands(Env env, Jobs jobs) { this.env = env; this.jobs = jobs; }

    List<CoreCommand> build() {
        List<CoreCommand> out = new ArrayList<>();
        out.add(Cmd.of("orders", List.of("buyorder", "bo"), this::orders).completing((p, a) -> a.length <= 1 ? List.of("list", "request", "pickup", "drop", "fill", "cancel") : List.of()));
        out.add(Cmd.of("contracts", List.of("contract"), this::contracts).completing((p, a) -> a.length <= 1 ? List.of("list", "submit", "abandon") : List.of()));
        return out;
    }

    private JsonObject who(CorePlayer p) {
        JsonObject o = new JsonObject();
        o.addProperty("uuid", p.uuid().toString());
        o.addProperty("name", p.name());
        return o;
    }

    private static String str(JsonObject o, String key, String fallback) { return o.has(key) && o.get(key).isJsonPrimitive() ? o.get(key).getAsString() : fallback; }
    private static double num(JsonObject o, String key) { return o.has(key) && o.get(key).isJsonPrimitive() ? o.get(key).getAsDouble() : 0; }
    private static long lng(JsonObject o, String key) { return (long) num(o, key); }

    private long id(CorePlayer p, String[] args, String usage) {
        if (args.length >= 2) {
            try { return Long.parseLong(args[1].replace("#", "")); } catch (NumberFormatException ignored) { /* fall through */ }
        }
        p.send(Format.RED + "Usage: " + usage);
        return -1;
    }

    // ---- buy orders --------------------------------------------------------------------------------------------------

    void orders(CorePlayer p, String[] args) {
        String sub = args.length == 0 ? "list" : args[0].toLowerCase(Locale.ROOT);
        switch (sub) {
            case "request", "create", "new" -> request(p, args);
            case "pickup", "claim", "take" -> simple(p, args, "economy/orders/claim", "/orders pickup <id>");
            case "drop", "release" -> simple(p, args, "economy/orders/release", "/orders drop <id>");
            case "cancel" -> simple(p, args, "economy/orders/cancel", "/orders cancel <id>");
            case "fill", "submit", "deliver" -> fill(p, args);
            default -> listOrders(p);
        }
    }

    private void listOrders(CorePlayer p) {
        p.send(Format.GRAY + "Loading buy orders...");
        env.io(() -> env.panel.call("economy/orders", who(p)).getAsJsonObject(), r -> {
            JsonArray list = r.has("orders") && r.get("orders").isJsonArray() ? r.getAsJsonArray("orders") : new JsonArray();
            p.send(Format.GOLD + "=== Buy Orders ===");
            if (list.size() == 0) p.send(Format.GRAY + "Nothing is wanted right now. Ask for something with " + Format.YELLOW + "/orders request <amount> <total price> [item]");
            int shown = 0;
            for (JsonElement e : list) {
                if (shown++ >= 15) { p.send(Format.GRAY + "...and " + (list.size() - 15) + " more in the launcher."); break; }
                JsonObject o = e.getAsJsonObject();
                boolean mine = o.has("mine") && o.get("mine").getAsBoolean(), taken = "claimed".equals(str(o, "status", ""));
                boolean mineTaken = o.has("claimed_by_me") && o.get("claimed_by_me").getAsBoolean();
                String state = mine ? Format.AQUA + "YOURS" : mineTaken ? Format.GREEN + "PICKED UP BY YOU" : taken ? Format.GRAY + "picked up by " + str(o, "claimer_name", "?") : Format.YELLOW + "open";
                p.send(Format.DARK_GRAY + "#" + lng(o, "id") + " " + Format.WHITE + lng(o, "amount") + "x " + str(o, "item_name", "?") + Format.GRAY + " for " + Format.GREEN + env.money(num(o, "total"))
                        + Format.GRAY + " (" + env.money(num(o, "each")) + " each) from " + str(o, "buyer_name", "?") + " · " + state);
            }
            p.send(Format.GRAY + "Pick one up with " + Format.YELLOW + "/orders pickup <id>" + Format.GRAY + ", hand the items in with " + Format.YELLOW + "/orders fill <id>" + Format.GRAY + ".");
        }, e -> p.send(Format.RED + "Could not load buy orders: " + e));
    }

    /** {@code /orders request <amount> <total price> [item]}: the item defaults to what is in your hand. */
    private void request(CorePlayer p, String[] args) {
        if (args.length < 3) { p.send(Format.RED + "Usage: /orders request <amount> <total price> [item]  (hold the item, or name it, e.g. DIAMOND)"); return; }
        int amount;
        try { amount = Integer.parseInt(args[1]); } catch (NumberFormatException e) { amount = -1; }
        Double total = Format.amount(args[2]);
        if (amount < 1 || total == null) { p.send(Format.RED + "Give a whole number of items and a price, like /orders request 64 150 DIAMOND."); return; }
        String item;
        if (args.length >= 4) item = args[3].toUpperCase(Locale.ROOT);
        else {
            Optional<Item> held = p.heldItem();
            if (held.isEmpty()) { p.send(Format.RED + "Hold the item you want, or name it: /orders request <amount> <total> <item>"); return; }
            item = held.get().id();
        }
        JsonObject body = who(p);
        body.addProperty("item_id", item);
        body.addProperty("amount", amount);
        body.addProperty("total", total);
        env.io(() -> { body.addProperty("operation_id", UUID.randomUUID().toString()); return env.panel.call("economy/orders/create", body).getAsJsonObject(); },
                r -> p.send(Format.GREEN + str(r, "message", "Buy order posted.")),
                e -> p.send(Format.RED + "Could not post the order: " + e));
    }

    private void simple(CorePlayer p, String[] args, String endpoint, String usage) {
        long id = id(p, args, usage);
        if (id < 0) return;
        JsonObject body = who(p);
        body.addProperty("order_id", id);
        env.io(() -> { body.addProperty("operation_id", UUID.randomUUID().toString()); return env.panel.call(endpoint, body).getAsJsonObject(); },
                r -> p.send(Format.GREEN + str(r, "message", "Done.")),
                e -> p.send(Format.RED + e));
    }

    /** Look the order up, take exactly what it needs from the inventory, and hand it in as one safe job. */
    private void fill(CorePlayer p, String[] args) {
        long id = id(p, args, "/orders fill <id>");
        if (id < 0) return;
        env.io(() -> env.panel.call("economy/orders", who(p)).getAsJsonObject(), r -> {
            JsonObject order = null;
            if (r.has("orders") && r.get("orders").isJsonArray()) for (JsonElement e : r.getAsJsonArray("orders")) if (lng(e.getAsJsonObject(), "id") == id) order = e.getAsJsonObject();
            if (order == null) { p.send(Format.RED + "Buy order #" + id + " isn't on the board any more."); return; }
            String itemId = str(order, "item_id", ""), name = str(order, "item_name", itemId);
            int need = (int) lng(order, "amount");
            List<Item> taken = p.takeItems(itemId, need);
            int have = taken.stream().mapToInt(Item::count).sum();
            if (have < need) {
                taken.forEach(p::give);
                p.send(Format.RED + "Order #" + id + " needs " + need + "x " + name + " in your inventory (plain items, no enchants or names). You have " + have + ".");
                return;
            }
            JsonObject payload = who(p);
            payload.addProperty("order_id", id);
            payload.addProperty("item_id", itemId);
            payload.addProperty("amount", have);
            if (!jobs.enqueue(p, "economy/orders/fill", payload, taken, null)) taken.forEach(p::give);
        }, e -> p.send(Format.RED + "Could not load the order: " + e));
    }

    // ---- contracts ----------------------------------------------------------------------------------------------------

    void contracts(CorePlayer p, String[] args) {
        String sub = args.length == 0 ? "list" : args[0].toLowerCase(Locale.ROOT);
        switch (sub) {
            case "submit", "deliver", "turnin" -> submit(p, args);
            case "abandon", "swap", "reroll" -> abandon(p, args);
            default -> listContracts(p);
        }
    }

    private void listContracts(CorePlayer p) {
        p.send(Format.GRAY + "Loading your contracts...");
        env.io(() -> env.panel.call("economy/contracts", who(p)).getAsJsonObject(), r -> {
            if (r.has("enabled") && !r.get("enabled").getAsBoolean()) { p.send(Format.RED + "Contracts are switched off on this server."); return; }
            p.send(Format.GOLD + "=== Your Contracts ===");
            JsonArray list = r.has("contracts") && r.get("contracts").isJsonArray() ? r.getAsJsonArray("contracts") : new JsonArray();
            if (list.size() == 0) p.send(Format.GRAY + "No contracts right now. New ones appear as you finish these.");
            for (JsonElement e : list) {
                JsonObject c = e.getAsJsonObject();
                boolean gather = "gather".equals(str(c, "kind", ""));
                p.send(Format.DARK_GRAY + "#" + lng(c, "id") + " " + (c.has("bonus") && c.get("bonus").isJsonPrimitive() && c.get("bonus").getAsBoolean() ? Format.AQUA + "HOT " : "") + Format.WHITE + str(c, "title", "?")
                        + Format.GRAY + " " + lng(c, "progress") + "/" + lng(c, "required") + " · pays " + Format.GREEN + env.money(num(c, "reward")) + Format.GRAY + (gather ? " · /contracts submit" : " · counts automatically"));
            }
            JsonObject s = r.has("stats") && r.get("stats").isJsonObject() ? r.getAsJsonObject("stats") : new JsonObject();
            p.send(Format.GRAY + "Finished today: " + lng(s, "done_today") + (lng(s, "daily_limit") > 0 ? "/" + lng(s, "daily_limit") : "") + " · swaps left: " + lng(s, "rerolls_left") + " (" + Format.YELLOW + "/contracts abandon <id>" + Format.GRAY + ")");
        }, e -> p.send(Format.RED + "Could not load contracts: " + e));
    }

    /** Hands in whatever the resource contracts still need from the inventory, one safe job per contract. */
    private void submit(CorePlayer p, String[] args) {
        long only = args.length >= 2 ? id(p, args, "/contracts submit [id]") : 0;
        if (args.length >= 2 && only < 0) return;
        env.io(() -> env.panel.call("economy/contracts", who(p)).getAsJsonObject(), r -> {
            JsonArray list = r.has("contracts") && r.get("contracts").isJsonArray() ? r.getAsJsonArray("contracts") : new JsonArray();
            int jobsQueued = 0;
            for (JsonElement e : list) {
                JsonObject c = e.getAsJsonObject();
                if (!"gather".equals(str(c, "kind", "")) || (only > 0 && lng(c, "id") != only)) continue;
                int need = (int) (lng(c, "required") - lng(c, "progress"));
                if (need <= 0) continue;
                List<Item> taken = p.takeItems(str(c, "target", ""), need);
                int have = taken.stream().mapToInt(Item::count).sum();
                if (have <= 0) continue;
                JsonObject payload = who(p);
                payload.addProperty("contract_id", lng(c, "id"));
                payload.addProperty("item_id", str(c, "target", ""));
                payload.addProperty("amount", have);
                if (!jobs.enqueue(p, "economy/contracts/submit", payload, taken, null)) taken.forEach(p::give); else jobsQueued++;
            }
            if (jobsQueued == 0) p.send(Format.RED + "Nothing in your inventory counts toward your resource contracts (plain items only). See " + Format.YELLOW + "/contracts" + Format.RED + ".");
        }, e -> p.send(Format.RED + "Could not load contracts: " + e));
    }

    private void abandon(CorePlayer p, String[] args) {
        long id = id(p, args, "/contracts abandon <id>");
        if (id < 0) return;
        JsonObject body = new JsonObject();
        body.addProperty("uuid", p.uuid().toString());
        body.addProperty("contract_id", id);
        env.io(() -> env.panel.call("economy/contracts/abandon", body).getAsJsonObject(), r -> { p.send(Format.GREEN + "Contract swapped for a new one."); listContracts(p); },
                e -> p.send(Format.RED + e));
    }
}
