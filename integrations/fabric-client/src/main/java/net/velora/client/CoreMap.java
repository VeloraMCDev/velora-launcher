package net.velora.client;

import com.google.gson.*;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphics;
import java.util.List;

/** Draws the same panel tiles as desktop/web. Never renders a separate client-owned world. */
public final class CoreMap {
    private Atlas atlas = new Atlas();
    private JsonObject data = new JsonObject();
    private String base = "";
    public String message = "Waiting for the server map";
    public void reset() { atlas.close(); atlas = new Atlas(); data = new JsonObject(); base = ""; message = "Waiting for the server map"; }
    public void receive(String panel, JsonObject snapshot) {
        try {
            var uri = java.net.URI.create(panel);
            if (!List.of("http", "https").contains(uri.getScheme()) || uri.getHost() == null || uri.getUserInfo() != null || uri.getQuery() != null || uri.getFragment() != null) return;
            if (!base.equals(panel)) { atlas.close(); atlas = new Atlas(); }
            base = panel; data = snapshot;
            JsonObject info = object(data, "info"); message = text(info, "message");
        } catch (RuntimeException ignored) { message = "Invalid server map response"; }
    }
    public String scope() { return base + "|" + text(object(data, "info"), "server_id"); }
    public static String dimension() {
        var level = Minecraft.getInstance().level; return level == null ? "minecraft:overworld" : level.dimension().location().toString();
    }
    public void render(GuiGraphics g, int left, int top, int width, int height, double cx, double cz, double scale, String dimension, boolean labels) {
        g.fill(left - 2, top - 2, left + width + 2, top + height + 2, 0xff7760b8);
        g.fill(left, top, left + width, top + height, 0xff171421);
        JsonObject info = object(data, "info");
        String token = text(info, "token"); long server = (long) number(info, "server_id");
        int zoom = Math.max(0, Math.min(6, (int) Math.floor(Math.log(1 / scale) / Math.log(2))));
        double span = 256 * (1 << zoom), drawn = span * scale;
        int minX = (int) Math.floor((cx - width / 2.0 / scale) / span), minZ = (int) Math.floor((cz - height / 2.0 / scale) / span);
        int cols = Math.min(12, (int) Math.ceil(width / drawn) + 1), rows = Math.min(12, (int) Math.ceil(height / drawn) + 1);
        g.enableScissor(left, top, left + width, top + height);
        try {
            if (!token.isEmpty() && server > 0) for (int iz = 0; iz < rows; iz++) for (int ix = 0; ix < cols; ix++) {
                int tx = minX + ix, tz = minZ + iz;
                var texture = atlas.tile(base, server, token, dimension, zoom, tx, tz);
                if (texture == null) continue;
                int x = left + (int) Math.floor((tx * span - cx) * scale + width / 2.0), z = top + (int) Math.floor((tz * span - cz) * scale + height / 2.0);
                int size = (int) Math.ceil(drawn) + 1;
                g.blit(texture, x, z, size, size, 0f, 0f, 256, 256, 256, 256);
            }
            var config = VeloraClient.config();
            if (Boolean.TRUE.equals(config.mapLayers.get("claims"))) for (JsonElement value : array(data, "claims")) {
                if (!value.isJsonObject()) continue; JsonObject claim = value.getAsJsonObject();
                if (!sameDimension(text(claim, "dimension"), dimension) && !sameDimension(text(claim, "slug"), dimension)) continue;
                JsonArray ring = array(claim, "outer");
                for (int at = 0; at < ring.size(); at++) {
                    JsonArray a = ring.get(at).isJsonArray() ? ring.get(at).getAsJsonArray() : new JsonArray();
                    JsonArray b = ring.get((at + 1) % ring.size()).isJsonArray() ? ring.get((at + 1) % ring.size()).getAsJsonArray() : new JsonArray();
                    if (a.size() < 2 || b.size() < 2) continue;
                    int x1 = screen(a.get(0).getAsDouble(), cx, scale, left, width), z1 = screen(a.get(1).getAsDouble(), cz, scale, top, height);
                    int x2 = screen(b.get(0).getAsDouble(), cx, scale, left, width), z2 = screen(b.get(1).getAsDouble(), cz, scale, top, height);
                    if (x1 == x2 || z1 == z2) g.fill(Math.min(x1, x2), Math.min(z1, z2), Math.max(x1, x2) + 1, Math.max(z1, z2) + 1, 0xaa9b7fe6);
                }
            }
            for (String group : new String[]{"pins", "players"}) for (JsonElement value : array(data, group)) {
                if (!value.isJsonObject()) continue; JsonObject pin = value.getAsJsonObject();
                if (!sameDimension(text(pin, "dimension"), dimension) && !sameDimension(text(pin, "slug"), dimension)) continue;
                String kind = text(pin, "kind"); String layer = group.equals("players") ? "players" : switch(kind) { case "spawn" -> "spawn"; case "warp" -> "warps"; case "home", "guild_home" -> "homes"; default -> "shops"; };
                if (!Boolean.TRUE.equals(config.mapLayers.get(layer))) continue;
                marker(g, left, top, width, height, cx, cz, scale, number(pin, "x"), number(pin, "z"), group.equals("players") ? text(pin, "name") : text(pin, "label"), 0xffffcb72, labels);
            }
            if (Boolean.TRUE.equals(config.mapLayers.get("waypoints"))) for (var waypoint : config.waypoints) {
                if (waypoint != null && waypoint.scope().equals(scope()) && waypoint.dimension().equals(dimension))
                    marker(g, left, top, width, height, cx, cz, scale, waypoint.x(), waypoint.z(), waypoint.name(), 0xffc99bff, labels);
            }
            if (Minecraft.getInstance().player != null && dimension.equals(dimension()))
                marker(g, left, top, width, height, cx, cz, scale, Minecraft.getInstance().player.getX(), Minecraft.getInstance().player.getZ(), "You", 0xff8bffd4, labels);
            if (token.isEmpty()) g.drawString(Minecraft.getInstance().font, message.isBlank() ? "Map not ready" : "Map loading…", left + 6, top + 6, 0xffb8a8ce, false);
        } finally { g.disableScissor(); }
    }
    private static int screen(double at, double center, double scale, int start, int size) { return start + (int) Math.round((at - center) * scale + size / 2.0); }
    private static void marker(GuiGraphics g, int left, int top, int width, int height, double cx, double cz, double scale, double x, double z, String name, int color, boolean label) {
        int sx = screen(x, cx, scale, left, width), sy = screen(z, cz, scale, top, height);
        if (sx < left || sx > left + width || sy < top || sy > top + height) return;
        g.fill(sx - 2, sy - 2, sx + 3, sy + 3, color);
        if (label) g.drawString(Minecraft.getInstance().font, name.substring(0, Math.min(32, name.length())), sx + 5, sy - 4, color, true);
    }
    private static boolean sameDimension(String value, String dimension) { return value.equals(dimension) || value.equals(dimension.replace("minecraft:", "").replace(":", "__")); }
    private static JsonObject object(JsonObject root, String key) { return root.has(key) && root.get(key).isJsonObject() ? root.getAsJsonObject(key) : new JsonObject(); }
    private static JsonArray array(JsonObject root, String key) { return root.has(key) && root.get(key).isJsonArray() ? root.getAsJsonArray(key) : new JsonArray(); }
    private static String text(JsonObject root, String key) { return root.has(key) && root.get(key).isJsonPrimitive() ? root.get(key).getAsString() : ""; }
    private static double number(JsonObject root, String key) { try { return root.has(key) ? root.get(key).getAsDouble() : 0; } catch(RuntimeException e) { return 0; } }
}
