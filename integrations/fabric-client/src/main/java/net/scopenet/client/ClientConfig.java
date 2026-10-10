package net.scopenet.client;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import net.fabricmc.loader.api.FabricLoader;

import java.io.IOException;
import java.io.Reader;
import java.io.Writer;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.LinkedHashMap;
import java.util.Map;

/** config/scopenet-client.json: what to show and where. Edited in game (K, then Settings) or by hand. */
public final class ClientConfig {
    private static final Gson GSON = new GsonBuilder().setPrettyPrinting().create();
    private static final Path FILE = FabricLoader.getInstance().getConfigDir().resolve("scopenet-client.json");

    /** Where a widget sits. {@code x}/{@code y} are offsets from the chosen screen anchor. */
    public static final class WidgetConfig {
        public boolean enabled = true;
        public String anchor = "top_left";
        public int x = 8;
        public int y = 8;
        public boolean compact = false;

        WidgetConfig(boolean enabled, String anchor, int x, int y) { this.enabled = enabled; this.anchor = anchor; this.x = x; this.y = y; }
        public WidgetConfig() { }
    }

    public static final class Hud {
        public float scale = 1.0f;
        public float opacity = 0.85f;
        public boolean hideInF3 = true;
        public boolean hideWhileChatOpen = false;
        public boolean claimBorders = true;
        public boolean claimBordersOnlyWhenSneaking = false;
        public Map<String, WidgetConfig> widgets = defaults();
    }

    public static final class Notifications {
        public boolean levelUp = true;
        public boolean achievements = true;
        public boolean guild = true;
        public boolean errors = true;
    }

    public boolean enabled = true;
    public Map<String, Boolean> modules = moduleDefaults();
    public Map<String, Boolean> mapLayers = new LinkedHashMap<>(Map.of("players", true, "claims", true, "spawn", true, "warps", true, "homes", true, "shops", true, "waypoints", true));
    public java.util.List<Waypoint> waypoints = new java.util.ArrayList<>();
    public record Waypoint(String scope, String dimension, String name, double x, double z) {}
    public float minimapScale = 0.5f;
    public static Map<String, Boolean> moduleDefaults() {
        Map<String, Boolean> values = new LinkedHashMap<>();
        for (String id : new String[]{"map", "economy", "vaults", "casino", "analytics", "factions"}) values.put(id, true);
        return values;
    }
    public boolean module(String id) { return Boolean.TRUE.equals(modules.get(id)); }
    public Hud hud = new Hud();
    public Notifications notifications = new Notifications();

    public static Map<String, WidgetConfig> defaults() {
        Map<String, WidgetConfig> m = new LinkedHashMap<>();
        m.put("level", new WidgetConfig(true, "top_left", 8, 8));
        m.put("balance", new WidgetConfig(true, "top_right", -8, 8));
        m.put("guild", new WidgetConfig(true, "bottom_left", 8, -40));
        m.put("claim", new WidgetConfig(true, "top_center", 0, 8));
        m.put("quests", new WidgetConfig(false, "middle_right", -8, 0));
        m.put("minimap", new WidgetConfig(true, "top_right", -8, 42));
        return m;
    }

    public WidgetConfig widget(String id) { return hud.widgets.computeIfAbsent(id, k -> defaults().getOrDefault(k, new WidgetConfig())); }

    public static ClientConfig load() {
        ClientConfig config = new ClientConfig();
        if (Files.exists(FILE)) {
            try (Reader in = Files.newBufferedReader(FILE)) {
                JsonObject root = JsonParser.parseReader(in).getAsJsonObject();
                ClientConfig read = GSON.fromJson(root, ClientConfig.class);
                if (read != null) config = read;
            } catch (IOException | RuntimeException e) {
                System.err.println("[Velora] Could not read " + FILE + ", using defaults: " + e.getMessage());
            }
        }
        // Keep every widget present, and values sane.
        Map<String, WidgetConfig> merged = defaults();
        if (config.hud == null) config.hud = new Hud();
        if (config.hud.widgets != null) merged.putAll(config.hud.widgets);
        config.hud.widgets = merged;
        if (config.notifications == null) config.notifications = new Notifications();
        Map<String, Boolean> modules = moduleDefaults();
        if (config.modules != null) for (String id : modules.keySet()) if (config.modules.containsKey(id)) modules.put(id, Boolean.TRUE.equals(config.modules.get(id)));
        config.modules = modules;
        if (config.mapLayers == null) config.mapLayers = new LinkedHashMap<>();
        for (String layer : new String[]{"players", "claims", "spawn", "warps", "homes", "shops", "waypoints"}) config.mapLayers.putIfAbsent(layer, true);
        if (config.waypoints == null) config.waypoints = new java.util.ArrayList<>();
        config.minimapScale = Float.isFinite(config.minimapScale) ? Math.max(0.1f, Math.min(2f, config.minimapScale)) : 0.5f;
        config.hud.scale = Math.max(0.5f, Math.min(2.0f, config.hud.scale));
        config.hud.opacity = Math.max(0.1f, Math.min(1.0f, config.hud.opacity));
        return config;
    }

    public void save() {
        try {
            Path tmp = FILE.resolveSibling(FILE.getFileName() + ".tmp");
            try (Writer w = Files.newBufferedWriter(tmp)) { GSON.toJson(this, w); }
            Files.move(tmp, FILE, StandardCopyOption.REPLACE_EXISTING);
        } catch (IOException e) {
            System.err.println("[Velora] Could not save " + FILE + ": " + e.getMessage());
        }
    }

    public void resetHud() {
        hud.widgets = defaults();
        hud.scale = 1.0f;
        hud.opacity = 0.85f;
    }
}
