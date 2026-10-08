package net.scopenet.client;

import com.google.gson.*;
import java.util.*;

/** What the panel's map endpoints return, parsed once: dimensions, claims, pins and players (see shared/map/types.ts). */
final class MapScene {
    record Dimension(String id, String slug, String label, int tiles, int[] bounds, int[] core, int[] levels) {}
    record Claim(String id, String name, String tag, int color, String slug, double labelX, double labelZ, double[] outer, List<double[]> holes, List<String> lines,
                 double minX, double maxX, double minZ, double maxZ) {
        String title() { return (tag.isEmpty() ? "" : "[" + tag + "] ") + name; }
    }
    record Pin(String id, String kind, String label, String slug, double x, double y, double z, List<String> lines) {}
    record Player(String uuid, String name, String slug, double x, double y, double z, double yaw) {}

    static final Map<String, Integer> PIN_COLORS = Map.of("spawn", 0xfbbf24, "warp", 0x38bdf8, "home", 0xa78bfa, "guild_home", 0xfb7185, "market", 0x34d399, "shop", 0xf59e0b);
    static final Map<String, String> PIN_ITEMS = Map.of("spawn", "minecraft:compass", "warp", "minecraft:ender_pearl", "home", "minecraft:red_bed",
            "guild_home", "minecraft:white_banner", "market", "minecraft:emerald", "shop", "minecraft:chest");

    boolean ready; String message = ""; String token = ""; long serverId; int maxZoom = 6;
    final List<Dimension> dimensions = new ArrayList<>();
    final List<Claim> claims = new ArrayList<>();
    final List<Pin> pins = new ArrayList<>();
    final List<Player> players = new ArrayList<>();

    /** Same naming as the panel: `minecraft:the_nether` -> `the_nether`, `mod:dim` -> `mod__dim`. */
    static String slug(String id) {
        if (id == null) return "";
        String t = id.startsWith("minecraft:") ? id.substring(10) : id;
        if (t.equals("nether")) return "the_nether";
        if (t.equals("end")) return "the_end";
        return t.replace(":", "__");
    }

    static MapScene parse(JsonObject data) {
        MapScene s = new MapScene();
        JsonObject info = Model.object(data, "info");
        s.ready = Model.flag(info, "ready"); s.message = Model.text(info, "message"); s.token = Model.text(info, "token");
        s.serverId = (long) Model.number(info, "server_id"); s.maxZoom = info.has("max_zoom") ? (int) Model.number(info, "max_zoom") : 6;
        for (JsonElement e : Model.array(info, "dimensions")) {
            if (!e.isJsonObject()) continue;
            JsonObject d = e.getAsJsonObject();
            if (d.has("available") && !Model.flag(d, "available")) continue;
            int[] levels = null;
            JsonArray lv = Model.array(d, "levels");
            if (!lv.isEmpty()) { levels = new int[lv.size()]; for (int i = 0; i < levels.length; i++) levels[i] = lv.get(i).getAsInt(); }
            int tiles = (int) Model.number(d, "tiles");
            int[] bounds = bounds(Model.object(d, "bounds")), core = bounds(Model.object(d, "core"));
            s.dimensions.add(new Dimension(Model.text(d, "id"), Model.text(d, "slug"), Model.text(d, "label"), tiles, bounds, tiles >= 8 && core != null ? core : bounds, levels));
        }
        for (JsonElement e : Model.array(data, "claims")) {
            if (!e.isJsonObject()) continue;
            JsonObject c = e.getAsJsonObject();
            double[] outer = ring(Model.array(c, "outer"));
            if (outer.length < 6) continue;
            List<double[]> holes = new ArrayList<>();
            for (JsonElement h : Model.array(c, "holes")) if (h.isJsonArray()) holes.add(ring(h.getAsJsonArray()));
            double minX = Double.MAX_VALUE, maxX = -Double.MAX_VALUE, minZ = Double.MAX_VALUE, maxZ = -Double.MAX_VALUE;
            for (int i = 0; i < outer.length; i += 2) { minX = Math.min(minX, outer[i]); maxX = Math.max(maxX, outer[i]); minZ = Math.min(minZ, outer[i + 1]); maxZ = Math.max(maxZ, outer[i + 1]); }
            s.claims.add(new Claim(Model.text(c, "id"), Model.text(c, "name"), Model.text(c, "tag"), color(Model.text(c, "color"), 0x8b6cff), slug(Model.text(c, "dimension")),
                    Model.number(c, "label_x"), Model.number(c, "label_z"), outer, holes, lines(c), minX, maxX, minZ, maxZ));
        }
        for (JsonElement e : Model.array(data, "pins")) {
            if (!e.isJsonObject()) continue;
            JsonObject p = e.getAsJsonObject();
            s.pins.add(new Pin(Model.text(p, "id"), Model.text(p, "kind"), Model.text(p, "label"), slug(Model.text(p, "dimension")), Model.number(p, "x"), Model.number(p, "y"), Model.number(p, "z"), lines(p)));
        }
        for (JsonElement e : Model.array(data, "players")) {
            if (!e.isJsonObject()) continue;
            JsonObject p = e.getAsJsonObject();
            s.players.add(new Player(Model.text(p, "uuid"), Model.text(p, "name"), slug(Model.text(p, "dimension")), Model.number(p, "x"), Model.number(p, "y"), Model.number(p, "z"), Model.number(p, "yaw")));
        }
        return s;
    }

    private static int[] bounds(JsonObject b) {
        if (b == null || b.isEmpty() || !b.has("min_x")) return null;
        return new int[]{(int) Model.number(b, "min_x"), (int) Model.number(b, "max_x"), (int) Model.number(b, "min_y"), (int) Model.number(b, "max_y")};
    }

    private static double[] ring(JsonArray points) {
        double[] out = new double[points.size() * 2];
        int n = 0;
        for (JsonElement p : points) {
            if (!p.isJsonArray() || p.getAsJsonArray().size() < 2) continue;
            out[n++] = p.getAsJsonArray().get(0).getAsDouble(); out[n++] = p.getAsJsonArray().get(1).getAsDouble();
        }
        return Arrays.copyOf(out, n);
    }

    private static List<String> lines(JsonObject o) {
        List<String> out = new ArrayList<>();
        for (JsonElement l : Model.array(o, "lines")) if (l.isJsonPrimitive()) out.add(Words.plain(l.getAsString()));
        return out;
    }

    static int color(String hex, int fallback) {
        try { return Integer.parseInt(hex.trim().replace("#", ""), 16) & 0xffffff; } catch (Exception e) { return fallback; }
    }

    Dimension dimension(String slug) { for (Dimension d : dimensions) if (d.slug.equals(slug)) return d; return null; }

    /** Even-odd point-in-ring test over a flat x,z array. */
    static boolean inside(double[] ring, double x, double z) {
        boolean in = false;
        int n = ring.length / 2;
        for (int i = 0, j = n - 1; i < n; j = i++) {
            double ax = ring[i * 2], az = ring[i * 2 + 1], bx = ring[j * 2], bz = ring[j * 2 + 1];
            if ((az > z) != (bz > z) && x < (bx - ax) * (z - az) / (bz - az) + ax) in = !in;
        }
        return in;
    }

    static boolean contains(Claim c, double x, double z) {
        if (!inside(c.outer(), x, z)) return false;
        for (double[] h : c.holes()) if (inside(h, x, z)) return false;
        return true;
    }

    /** The x ranges of a claim along one world z line (outer ring minus holes, even-odd), as [from, to, from, to...]. */
    static double[] spans(Claim c, double z) {
        List<Double> xs = new ArrayList<>();
        crossings(c.outer(), z, xs);
        for (double[] h : c.holes()) crossings(h, z, xs);
        Collections.sort(xs);
        double[] out = new double[xs.size() / 2 * 2];
        for (int i = 0; i < out.length; i++) out[i] = xs.get(i);
        return out;
    }

    private static void crossings(double[] ring, double z, List<Double> xs) {
        int n = ring.length / 2;
        for (int i = 0, j = n - 1; i < n; j = i++) {
            double ax = ring[i * 2], az = ring[i * 2 + 1], bx = ring[j * 2], bz = ring[j * 2 + 1];
            if ((az > z) != (bz > z)) xs.add((bx - ax) * (z - az) / (bz - az) + ax);
        }
    }
}
