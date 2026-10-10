package net.scopenet.core.map;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.scopenet.core.Pos;
import net.scopenet.core.ShopPoints;
import net.scopenet.integration.ClaimIndex;

import java.util.*;

/** Builds {@link MapData} from what the server already knows: claims, warps, spawn, shop points, listings and players. */
public final class MapModel {
    private MapModel() {}

    /** Everything the map is built from. All of it must be safe to read from another thread. */
    public record Sources(Map<String, List<ClaimIndex.GuildClaims>> claims, Map<String, Pos> warps, Collection<Pos> spawns,
                          Map<UUID, Map<String, Pos>> homes, Map<String, NamedPos> guildHomes, Collection<ShopPoints.Shop> shops,
                          JsonArray market, List<MapData.OnlinePlayer> players, String panelBase, boolean showHomes,
                          Map<UUID, String> homeOwners) {}

    public record NamedPos(String label, Pos pos) {}

    public static String escape(String text) {
        if (text == null) return "";
        StringBuilder out = new StringBuilder(text.length() + 8);
        for (char c : text.toCharArray()) {
            switch (c) {
                case '&' -> out.append("&amp;");
                case '<' -> out.append("&lt;");
                case '>' -> out.append("&gt;");
                case '"' -> out.append("&quot;");
                case '\'' -> out.append("&#39;");
                default -> { if (c >= 32 || c == '\n') out.append(c); }
            }
        }
        return out.toString();
    }

    /** A stable, pleasant colour per faction: hue from the id, fixed saturation and lightness. */
    public static int guildColor(String guildId) {
        int h = guildId.hashCode();
        double hue = Math.floorMod(h, 360);
        return hsl(hue, 0.65, 0.55);
    }

    static int hsl(double h, double s, double l) {
        double c = (1 - Math.abs(2 * l - 1)) * s, x = c * (1 - Math.abs((h / 60) % 2 - 1)), m = l - c / 2;
        double r, g, b;
        if (h < 60) { r = c; g = x; b = 0; } else if (h < 120) { r = x; g = c; b = 0; } else if (h < 180) { r = 0; g = c; b = x; }
        else if (h < 240) { r = 0; g = x; b = c; } else if (h < 300) { r = x; g = 0; b = c; } else { r = c; g = 0; b = x; }
        return ((int) Math.round((r + m) * 255) << 16) | ((int) Math.round((g + m) * 255) << 8) | (int) Math.round((b + m) * 255);
    }

    /** Which Minecraft dimension a world id or name means ("world_nether" -> the nether). */
    public static String dimensionOf(String id, String name) {
        List<String> words = Arrays.asList(((id == null ? "" : id) + " " + (name == null ? "" : name)).toLowerCase(Locale.ROOT).split("[^a-z]+"));
        if (words.contains("nether")) return "minecraft:the_nether";
        if (words.contains("end")) return "minecraft:the_end";
        return "minecraft:overworld";
    }

    /** The icon address: a full web address as is, a path on the panel made absolute, empty if none. */
    public static String iconAddress(String icon, String panelBase) {
        if (icon == null || icon.isBlank()) return "";
        String i = icon.trim();
        if (i.startsWith("https://") || i.startsWith("http://")) return i;
        if (i.startsWith("/") && panelBase != null && !panelBase.isBlank()) return panelBase.replaceAll("/+$", "") + i;
        return "";
    }

    public static MapData build(Sources s) {
        List<MapData.ClaimRegion> claims = new ArrayList<>();
        s.claims().forEach((dimension, guilds) -> {
            for (ClaimIndex.GuildClaims g : guilds) {
                int n = 0;
                for (ChunkRegions.Region r : ChunkRegions.regions(g.chunks())) {
                    String detail = "<div class=\"scopenet-detail\"><strong>" + escape(g.name()) + "</strong> <span>[" + escape(g.tag()) + "]</span><br>"
                            + (g.description() == null || g.description().isBlank() ? "" : escape(g.description()) + "<br>")
                            + r.chunks() + (g.admin() ? " protected chunk" : " claimed chunk") + (r.chunks() == 1 ? "" : "s") + " in this area</div>";
                    claims.add(new MapData.ClaimRegion(g.id() + ":" + dimension + ":" + n++, g.id(), g.name(), g.tag(), iconAddress(g.icon(), s.panelBase()),
                            g.color() >= 0 ? g.color() : guildColor(g.id()), dimension, r.outer(), r.holes(), r.chunks(), r.labelX(), r.labelZ(), detail));
                }
            }
        });

        List<MapData.Pin> pins = new ArrayList<>();
        int i = 0;
        for (Pos spawn : s.spawns()) pins.add(new MapData.Pin("spawn-" + i++, "spawn", "Spawn", spawn.world(), spawn.x(), spawn.y(), spawn.z(),
                "<div class=\"scopenet-detail\"><strong>World spawn</strong></div>"));
        s.warps().forEach((name, p) -> pins.add(new MapData.Pin("warp-" + name, "warp", "Warp: " + name, p.world(), p.x(), p.y(), p.z(),
                "<div class=\"scopenet-detail\"><strong>Warp</strong> " + escape(name) + "<br><code>/warp " + escape(name) + "</code></div>")));
        s.guildHomes().forEach((guildId, home) -> pins.add(new MapData.Pin("guild-home-" + guildId, "guild_home", home.label(), home.pos().world(),
                home.pos().x(), home.pos().y(), home.pos().z(), "<div class=\"scopenet-detail\"><strong>" + escape(home.label()) + "</strong><br>Faction home</div>")));
        if (s.showHomes()) {
            s.homes().forEach((owner, byName) -> byName.forEach((name, p) -> {
                String who = s.homeOwners().getOrDefault(owner, "a player");
                pins.add(new MapData.Pin("home-" + owner + "-" + name, "home", who + "'s home", p.world(), p.x(), p.y(), p.z(),
                        "<div class=\"scopenet-detail\"><strong>" + escape(who) + "</strong><br>Home: " + escape(name) + "</div>"));
            }));
        }
        String marketDetail = marketSummary(s.market());
        for (ShopPoints.Shop shop : s.shops()) {
            boolean market = shop.kind().equals("market");
            String who = shop.isNpc() ? "Shopkeeper " : "";
            String detail = "<div class=\"scopenet-detail\"><strong>" + escape(shop.name().replace('_', ' ')) + "</strong>" + (shop.isNpc() ? " <span>shopkeeper</span>" : "") + "<br>"
                    + (market ? "Player market. Right-click to browse, bid and trade." + marketDetail : "Server shop. Right-click to buy and sell.") + "</div>";
            Pos p = shop.pos();
            pins.add(new MapData.Pin("shop-" + shop.name(), market ? "market" : "shop", who + shop.name().replace('_', ' '), p.world(), p.x(), p.y(), p.z(), detail));
        }
        pins.sort(Comparator.comparing(MapData.Pin::id));
        return new MapData(claims, pins, s.players());
    }

    /** "12 listings" and the newest few, as HTML. The market is per server, so every market point shows the same. */
    static String marketSummary(JsonArray listings) {
        if (listings == null || listings.size() == 0) return "<br><em>No listings right now.</em>";
        StringBuilder b = new StringBuilder("<br><strong>" + listings.size() + " listing" + (listings.size() == 1 ? "" : "s") + "</strong><ul class=\"scopenet-list\">");
        int shown = 0;
        for (JsonElement e : listings) {
            if (shown++ >= 5) break;
            JsonObject l = e.getAsJsonObject();
            String guild = l.has("seller_guild") && l.get("seller_guild").isJsonPrimitive() ? "[" + l.get("seller_guild").getAsString() + "] faction" : text(l, "seller_name");
            b.append("<li>").append(escape(text(l, "item_name"))).append(" x").append(l.has("amount") ? l.get("amount").getAsInt() : 1)
                    .append(" &middot; ").append(String.format(Locale.US, "%,.2f", l.has("price") ? l.get("price").getAsDouble() : 0))
                    .append(" <span>by ").append(escape(guild)).append("</span></li>");
        }
        return b.append("</ul>").toString();
    }

    private static String text(JsonObject o, String key) { return o.has(key) && o.get(key).isJsonPrimitive() ? o.get(key).getAsString() : ""; }

    /** The file the map page reads to describe players who are online (public facts only: never balances). */
    public static JsonObject playersJson(String server, List<MapData.OnlinePlayer> players, long now) {
        JsonObject root = new JsonObject();
        root.addProperty("updated", now);
        root.addProperty("server", server);
        JsonObject byId = new JsonObject();
        for (MapData.OnlinePlayer p : players) {
            JsonObject o = new JsonObject();
            o.addProperty("name", p.name());
            o.addProperty("level", p.level());
            o.addProperty("title", p.title());
            o.addProperty("serverLevel", p.serverLevel());
            o.addProperty("guild", p.guild());
            o.addProperty("guildTag", p.guildTag());
            o.addProperty("guildRole", p.guildRole());
            o.addProperty("rank", p.rank());
            o.addProperty("playtime", p.playtimeSecs());
            byId.add(p.uuid(), o);
        }
        root.add("players", byId);
        return root;
    }
}
