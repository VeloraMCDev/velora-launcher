package net.scopenet.integration;

import com.google.gson.*;
import java.util.*;
import java.util.function.LongSupplier;

/** All callers (including asynchronous chat events) share one short lock. */
public final class Activity {
    private record Player(String name, long since) {}
    private final Map<UUID, Player> online = new LinkedHashMap<>();
    private final Map<UUID, JsonObject> stats = new LinkedHashMap<>();
    private final List<JsonObject> events = new ArrayList<>();
    private final Map<String, JsonObject> actions = new LinkedHashMap<>();
    private long dropped;
    private final LongSupplier clock;
    private static final int MAX_PLAYERS = 5000;
    private static final int MAX_EVENTS = 500;

    public Activity() { this(System::nanoTime); }
    Activity(LongSupplier clock) { this.clock = clock; }

    public synchronized void join(UUID id, String name) {
        if (online.containsKey(id)) return;
        if (online.size() >= MAX_PLAYERS) return;
        online.put(id, new Player(name, clock.getAsLong()));
        add(id, name, "joins", 1);
        event(id, name, "join", null);
    }

    public synchronized void leave(UUID id, String name) {
        Player player = online.remove(id);
        if (player == null) return;
        add(id, name, "playtime_secs", Math.max(0, (clock.getAsLong() - player.since()) / 1_000_000_000L));
        event(id, name, "leave", null);
    }

    public synchronized void add(UUID id, String name, String key, long count) {
        if (count <= 0 || (!stats.containsKey(id) && stats.size() >= MAX_PLAYERS)) return;
        JsonObject row = stats.computeIfAbsent(id, unused -> identity(id, name));
        row.addProperty(key, Math.min(1_000_000L, (row.has(key) ? row.get(key).getAsLong() : 0) + count));
    }

    public synchronized void event(UUID id, String name, String kind, String detail) {
        if (events.size() >= 10_000) { dropped++; return; }
        JsonObject event = identity(id, name);
        event.addProperty("kind", kind);
        if (detail != null) event.addProperty("detail", detail.substring(0, Math.min(256, detail.length())));
        event.addProperty("at", System.currentTimeMillis());
        events.add(event);
    }

    /** Repeated vanilla actions are aggregated within each reporting interval. */
    public synchronized void action(UUID id, String name, String action, long count) {
        String key = id + ":" + action;
        if (count <= 0) return;
        if (!actions.containsKey(key) && actions.size() >= 10_000) { dropped++; return; }
        JsonObject row = actions.computeIfAbsent(key, unused -> {
            JsonObject value = identity(id, name);
            value.addProperty("kind", "action");
            value.addProperty("action", action);
            value.addProperty("count", 0);
            value.addProperty("at", System.currentTimeMillis());
            return value;
        });
        row.addProperty("count", row.get("count").getAsLong() + count);
    }

    public synchronized JsonObject drain(double tps) {
        long now = clock.getAsLong();
        JsonArray players = new JsonArray();
        online.replaceAll((id, player) -> {
            long seconds = Math.max(0, (now - player.since()) / 1_000_000_000L);
            add(id, player.name(), "playtime_secs", seconds);
            players.add(identity(id, player.name()));
            return new Player(player.name(), player.since() + seconds * 1_000_000_000L);
        });
        JsonObject payload = new JsonObject();
        payload.addProperty("batch_id", UUID.randomUUID().toString());
        payload.addProperty("tps", Double.isFinite(tps) ? Math.max(0, Math.min(20, tps)) : 20);
        payload.add("online", players);
        JsonArray rows = new JsonArray();
        stats.values().forEach(rows::add);
        payload.add("stats", rows);
        JsonArray activity = new JsonArray();
        int eventCount = Math.min(events.size(), MAX_EVENTS - 1);
        events.subList(0, eventCount).forEach(activity::add);
        events.subList(0, eventCount).clear();
        var iterator = actions.values().iterator();
        while (iterator.hasNext() && activity.size() < MAX_EVENTS - 1) {
            JsonObject action = iterator.next();
            action.addProperty("detail", action.remove("action").getAsString() + " +" + action.remove("count").getAsLong());
            activity.add(action);
            iterator.remove();
        }
        if (dropped > 0) {
            JsonObject gap = new JsonObject();
            gap.addProperty("kind", "telemetry_gap");
            gap.addProperty("detail", dropped + " events exceeded the local queue capacity");
            activity.add(gap);
            dropped = 0;
        }
        payload.add("events", activity);
        stats.clear();
        return payload;
    }

    public synchronized void leaveAll() {
        new LinkedHashMap<>(online).forEach((id, player) -> leave(id, player.name()));
    }

    private static JsonObject identity(UUID id, String name) {
        JsonObject object = new JsonObject();
        object.addProperty("uuid", id.toString());
        object.addProperty("name", name);
        return object;
    }
}
