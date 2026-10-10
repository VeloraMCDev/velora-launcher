package net.velora.client;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;

import java.util.ArrayList;
import java.util.List;

/** What the server last told this client. Everything is optional: before the first message the HUD shows nothing. */
public final class ClientState {
    public record Quest(int done, int total) {}
    public record Guild(String name, String tag, String role, long claims) {}
    public record Owner(String name, String tag, boolean mine) {}
    public record Listing(long id, String seller, String guildTag, String itemId, String itemName, int amount, double price) {}

    public volatile boolean connected;
    public volatile String server = "";
    public volatile String currency = "$";
    public volatile boolean featEconomy, featGuilds, featClaims, featEssentials;
    public volatile JsonObject modules;
    public boolean module(String id) { return modules == null || bool(modules, id); }

    public volatile boolean hasState;
    public volatile String name = "";
    public volatile long level = 1, xp, xpNow, xpNext, serverLevel = 1;
    public volatile double xpPct, serverXpPct, balance;
    public volatile String title = "", rank = "";
    public volatile Guild guild;
    public volatile Quest daily = new Quest(0, 0), weekly = new Quest(0, 0);
    public volatile long playtime, kills, deaths;

    public volatile String claimDim = "";
    public volatile int claimCx, claimCz, claimRadius;
    public volatile String claimCells = "";
    public volatile Owner claimOwner;

    public volatile List<Listing> market = List.of();
    public volatile long marketAt;
    /** A window the server asked us to open ("market"), taken once by the screen code. */
    public volatile String pendingOpen;

    /** Called on the client thread with a server message; returns a toast (kind, title, text) or null. */
    public String[] handle(String json) {
        JsonObject m;
        try { m = JsonParser.parseString(json).getAsJsonObject(); } catch (RuntimeException e) { return null; }
        if (m.has("modules") && m.get("modules").isJsonObject()) {
            modules = m.getAsJsonObject("modules").deepCopy();
            featEconomy = module("economy"); featGuilds = module("factions"); featClaims = module("factions");
        }
        switch (str(m, "t")) {
            case "hello" -> {
                connected = true;
                server = str(m, "server");
                currency = m.has("currency") ? str(m, "currency") : "$";
                JsonObject f = obj(m, "features");
                featEconomy = bool(f, "economy"); featGuilds = bool(f, "guilds"); featClaims = bool(f, "claims"); featEssentials = bool(f, "essentials");
            }
            case "state" -> {
                name = str(m, "name");
                level = lng(m, "level", 1); xp = lng(m, "xp", 0); xpNow = lng(m, "xpNow", 0); xpNext = lng(m, "xpNext", 0);
                xpPct = num(m, "xpPct"); title = str(m, "title"); serverLevel = lng(m, "serverLevel", 1); serverXpPct = num(m, "serverXpPct");
                balance = num(m, "balance"); playtime = lng(m, "playtime", 0); kills = lng(m, "kills", 0); deaths = lng(m, "deaths", 0);
                rank = str(m, "rank");
                JsonObject g = obj(m, "guild");
                guild = g == null ? null : new Guild(str(g, "name"), str(g, "tag"), str(g, "role"), lng(g, "claims", 0));
                daily = quest(obj(m, "daily"));
                weekly = quest(obj(m, "weekly"));
                hasState = true;
            }
            case "claims" -> {
                claimDim = str(m, "dim"); claimCx = (int) lng(m, "cx", 0); claimCz = (int) lng(m, "cz", 0);
                claimRadius = (int) lng(m, "r", 0); claimCells = str(m, "cells");
                JsonObject o = obj(m, "owner");
                claimOwner = o == null ? null : new Owner(str(o, "name"), str(o, "tag"), bool(o, "mine"));
            }
            case "market" -> {
                List<Listing> list = new ArrayList<>();
                for (JsonElement e : arr(m, "listings")) {
                    JsonObject l = e.getAsJsonObject();
                    list.add(new Listing(lng(l, "id", 0), str(l, "seller_name"), l.has("seller_guild") && l.get("seller_guild").isJsonPrimitive() ? str(l, "seller_guild") : null,
                            str(l, "item_id"), str(l, "item_name"), (int) lng(l, "amount", 1), num(l, "price")));
                }
                market = list;
                if (m.has("balance")) balance = num(m, "balance");
                marketAt = System.currentTimeMillis();
            }
            case "notify" -> { return new String[]{str(m, "kind"), str(m, "title"), str(m, "text")}; }
            case "open" -> pendingOpen = str(m, "screen");
            default -> { }
        }
        return null;
    }

    /** Sample values so the layout editor can show every widget even with no server data. */
    public static ClientState preview() {
        ClientState s = new ClientState();
        s.connected = true; s.hasState = true; s.featEconomy = true; s.featGuilds = true; s.featClaims = true; s.featEssentials = true;
        s.level = 27; s.xpPct = 62; s.title = "Miner"; s.serverLevel = 9; s.balance = 12840.5;
        s.guild = new Guild("Iron Wolves", "IRON", "officer", 14);
        s.daily = new Quest(2, 3); s.weekly = new Quest(1, 4);
        s.claimCells = "x"; s.claimOwner = new Owner("Iron Wolves", "IRON", true);
        return s;
    }

    public void reset() {
        connected = false; hasState = false; guild = null; claimOwner = null; claimCells = ""; market = List.of(); modules = null;
    }

    private static Quest quest(JsonObject o) { return o == null ? new Quest(0, 0) : new Quest((int) lng(o, "done", 0), (int) lng(o, "total", 0)); }
    private static String str(JsonObject o, String k) { JsonElement e = o == null ? null : o.get(k); return e != null && e.isJsonPrimitive() ? e.getAsString() : ""; }
    private static double num(JsonObject o, String k) { JsonElement e = o == null ? null : o.get(k); return e != null && e.isJsonPrimitive() && e.getAsJsonPrimitive().isNumber() ? e.getAsDouble() : 0; }
    private static long lng(JsonObject o, String k, long d) { JsonElement e = o == null ? null : o.get(k); return e != null && e.isJsonPrimitive() && e.getAsJsonPrimitive().isNumber() ? e.getAsLong() : d; }
    private static boolean bool(JsonObject o, String k) { JsonElement e = o == null ? null : o.get(k); return e != null && e.isJsonPrimitive() && e.getAsBoolean(); }
    private static JsonObject obj(JsonObject o, String k) { JsonElement e = o == null ? null : o.get(k); return e != null && e.isJsonObject() ? e.getAsJsonObject() : null; }
    private static JsonArray arr(JsonObject o, String k) { JsonElement e = o == null ? null : o.get(k); return e != null && e.isJsonArray() ? e.getAsJsonArray() : new JsonArray(); }
}
