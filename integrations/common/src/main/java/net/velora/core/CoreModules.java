package net.velora.core;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.Properties;
import java.util.Set;

/** Immutable seven-module policy. Authentication is mandatory and has no off switch. */
public final class CoreModules {
    public static final Set<String> IDS = Set.of("map", "economy", "vaults", "casino", "analytics", "factions", "permissions_chat");
    private final Map<String, Boolean> enabled;

    private CoreModules(Map<String, Boolean> enabled) { this.enabled = Map.copyOf(enabled); }
    public static CoreModules all() { return defaults(true); }
    public static CoreModules pending() { return defaults(false); }
    private static CoreModules defaults(boolean value) {
        Map<String, Boolean> map = new LinkedHashMap<>(); IDS.forEach(id -> map.put(id, value)); return new CoreModules(map);
    }
    public boolean enabled(String id) { return enabled.getOrDefault(id, false); }
    public CoreModules intersect(CoreModules other) {
        Map<String, Boolean> map = new LinkedHashMap<>(); IDS.forEach(id -> map.put(id, enabled(id) && other.enabled(id))); return new CoreModules(map);
    }
    public static CoreModules local(Properties properties) {
        Map<String, Boolean> map = new LinkedHashMap<>();
        for (String id : IDS) {
            String raw = properties.getProperty("modules." + id + ".enabled", "true").trim();
            if (!raw.equalsIgnoreCase("true") && !raw.equalsIgnoreCase("false")) throw new IllegalArgumentException("modules." + id + ".enabled must be true or false");
            map.put(id, Boolean.parseBoolean(raw));
        }
        return new CoreModules(map);
    }
    /** Missing policy means a legacy panel. Malformed/newer policies fail closed. */
    public static CoreModules fromPanel(JsonObject response) {
        JsonElement policy = response.get("velora_core");
        if (policy == null || policy.isJsonNull()) return all();
        try {
            JsonObject root = policy.getAsJsonObject();
            if (root.get("schema").getAsInt() != 1) return pending();
            JsonObject modules = root.getAsJsonObject("modules");
            if (modules.size() != IDS.size()) return pending();
            Map<String, Boolean> map = new LinkedHashMap<>();
            for (String id : IDS) {
                JsonElement value = modules.get(id);
                if (value == null || !value.isJsonPrimitive() || !value.getAsJsonPrimitive().isBoolean()) return pending();
                map.put(id, value.getAsBoolean());
            }
            return new CoreModules(map);
        } catch (RuntimeException e) { return pending(); }
    }
    public JsonObject toJson() {
        JsonObject object = new JsonObject(); enabled.forEach(object::addProperty); return object;
    }
    public boolean commandEnabled(String name) {
        return switch (name) {
            case "vault", "pv" -> enabled("vaults");
            case "guild", "g", "faction", "f", "claim", "unclaim", "adminclaim" -> enabled("factions");
            case "balance", "bal", "pay", "baltop", "market", "auction", "orders", "contracts", "board", "shop", "sell", "darknet" -> enabled("economy");
            case "casino" -> enabled("casino") && enabled("economy");
            default -> true;
        };
    }
}
