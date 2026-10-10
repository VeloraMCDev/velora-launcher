package net.velora.core;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

import java.util.Map;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;

/**
 * The panel's view of each online player (level, guild, balance, quests…), refreshed in the background so
 * placeholders and the client HUD never wait on the network.
 */
public final class PlayerCache {
    public static final long REFRESH_MS = 20_000;

    private record Entry(JsonObject info, long at) {}

    private final Env env;
    private final Map<UUID, Entry> entries = new ConcurrentHashMap<>();
    private final Map<UUID, Long> inFlight = new ConcurrentHashMap<>();

    public PlayerCache(Env env) { this.env = env; }

    /** The last answer for this player, or null if none yet. */
    public JsonObject info(UUID uuid) {
        Entry e = entries.get(uuid);
        return e == null ? null : e.info();
    }

    public void forget(UUID uuid) { entries.remove(uuid); inFlight.remove(uuid); }

    /** Fetch now (unless a fetch is already running); {@code done} runs on the server thread with the new info or null. */
    public void refresh(CorePlayer player, java.util.function.Consumer<JsonObject> done) {
        if (inFlight.putIfAbsent(player.uuid(), env.clock.getAsLong()) != null) return;
        JsonObject body = new JsonObject();
        body.addProperty("uuid", player.uuid().toString());
        env.io(() -> env.panel.call("player/info", body).getAsJsonObject(), info -> {
            inFlight.remove(player.uuid());
            if (info.has("exists") && !info.get("exists").getAsBoolean()) { if (done != null) done.accept(null); return; }
            entries.put(player.uuid(), new Entry(info, env.clock.getAsLong()));
            if (done != null) done.accept(info);
        }, e -> {
            inFlight.remove(player.uuid());
            if (done != null) done.accept(null);
        });
    }

    /** Keep everyone online fresh. Call about once a second on the server thread. */
    public void tick(java.util.function.Consumer<CorePlayer> refreshed) {
        long now = env.clock.getAsLong();
        for (CorePlayer p : env.platform.online()) {
            Entry e = entries.get(p.uuid());
            if (e == null || now - e.at() >= REFRESH_MS) refresh(p, info -> { if (info != null && refreshed != null) refreshed.accept(p); });
        }
        entries.keySet().removeIf(id -> env.platform.player(id).isEmpty());
    }

    // ---- small accessors used by placeholders and the client link -----------------------------

    public static JsonObject obj(JsonObject o, String key) {
        JsonElement e = o == null ? null : o.get(key);
        return e != null && e.isJsonObject() ? e.getAsJsonObject() : null;
    }

    public static double num(JsonObject o, String key, double fallback) {
        JsonElement e = o == null ? null : o.get(key);
        return e != null && e.isJsonPrimitive() && e.getAsJsonPrimitive().isNumber() ? e.getAsDouble() : fallback;
    }

    public static String str(JsonObject o, String key, String fallback) {
        JsonElement e = o == null ? null : o.get(key);
        return e != null && e.isJsonPrimitive() ? e.getAsString() : fallback;
    }

    public static JsonArray arr(JsonObject o, String key) {
        JsonElement e = o == null ? null : o.get(key);
        return e != null && e.isJsonArray() ? e.getAsJsonArray() : new JsonArray();
    }
}
