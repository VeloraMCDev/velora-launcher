package net.scopenet.paper.compat;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.scopenet.api.*;
import net.scopenet.integration.PanelClient;
import java.util.*;
import java.util.concurrent.*;

/**
 * The developer API, backed by the panel. Network calls run on a small worker
 * pool, so nothing here blocks the server thread.
 */
public final class ScopenetApiImpl implements ScopenetApi, AutoCloseable {
    private record Cached(ScopenetPlayer player, long loadedAt) {}

    private static final long STALE_MS = 30_000;
    private final PanelClient client;
    private final ExecutorService io = new ThreadPoolExecutor(2, 4, 30, TimeUnit.SECONDS, new LinkedBlockingQueue<>(512), r -> {
        Thread t = new Thread(r, "scopenet-api");
        t.setDaemon(true);
        return t;
    });
    private final Map<UUID, Cached> cache = new ConcurrentHashMap<>();
    private final Set<UUID> refreshing = ConcurrentHashMap.newKeySet();
    private volatile String rankNote = "";

    public ScopenetApiImpl(PanelClient client) {
        this.client = client;
    }

    // ---- JSON helpers (tolerant: a missing field is a default, never a crash) ----

    private static JsonObject obj(JsonObject o, String key) {
        JsonElement e = o == null ? null : o.get(key);
        return e != null && e.isJsonObject() ? e.getAsJsonObject() : null;
    }

    private static String str(JsonObject o, String key) {
        JsonElement e = o == null ? null : o.get(key);
        return e != null && e.isJsonPrimitive() ? e.getAsString() : null;
    }

    private static long num(JsonObject o, String key) {
        JsonElement e = o == null ? null : o.get(key);
        return e != null && e.isJsonPrimitive() && e.getAsJsonPrimitive().isNumber() ? e.getAsLong() : 0L;
    }

    private static double dbl(JsonObject o, String key) {
        JsonElement e = o == null ? null : o.get(key);
        return e != null && e.isJsonPrimitive() && e.getAsJsonPrimitive().isNumber() ? e.getAsDouble() : 0.0;
    }

    private static QuestProgress quests(JsonObject all, String period) {
        JsonObject q = obj(obj(all, "quests"), period);
        return new QuestProgress((int) num(q, "total"), (int) num(q, "completed"), (int) num(q, "claimed"));
    }

    /** Turns a {@code player/info} response into a profile, or {@code null} if the player has no account. */
    static ScopenetPlayer player(JsonObject j) {
        if (j.has("exists") && !j.get("exists").getAsBoolean()) return null;
        JsonObject global = obj(j, "global"), server = obj(j, "server"), guild = obj(j, "guild"), rank = obj(j, "rank");
        return new ScopenetPlayer(
                UUID.fromString(str(j, "uuid")), str(j, "name"),
                (int) num(global, "level"), num(global, "xp"), num(global, "current_level_xp"), num(global, "next_level_xp"), dbl(global, "progress_pct"),
                str(global, "title"), (int) num(global, "rank"),
                (int) num(server, "level"), num(server, "xp"), str(server, "rank_name"),
                guild == null ? null : new GuildSummary(str(guild, "id"), str(guild, "name"), str(guild, "tag"), str(guild, "role"), (int) num(guild, "claims")),
                dbl(j, "balance"), num(j, "playtime_secs"), num(j, "server_playtime_secs"), (int) num(j, "kills"), (int) num(j, "deaths"),
                (int) num(j, "friends"), (int) num(j, "achievements"), quests(j, "daily"), quests(j, "weekly"),
                str(j, "joined"), str(rank, "group"), str(rank, "display"), str(rank, "prefix"), str(rank, "suffix"), str(global, "title_glyph"));
    }

    static ScopenetGuild guild(JsonObject j) {
        if (!j.has("exists") || !j.get("exists").getAsBoolean()) return null;
        List<ScopenetGuild.Member> members = new ArrayList<>();
        for (JsonElement e : j.getAsJsonArray("members")) {
            JsonObject m = e.getAsJsonObject();
            members.add(new ScopenetGuild.Member(UUID.fromString(str(m, "uuid")), str(m, "name"), str(m, "role")));
        }
        return new ScopenetGuild(str(j, "id"), str(j, "name"), str(j, "tag"), str(j, "description"), UUID.fromString(str(j, "leader")),
                (int) num(j, "claims"), dbl(j, "balance"), List.copyOf(members));
    }

    private static JsonObject who(UUID uuid) {
        JsonObject o = new JsonObject();
        o.addProperty("uuid", uuid.toString());
        return o;
    }

    private <T> CompletableFuture<T> async(Callable<T> call) {
        CompletableFuture<T> future = new CompletableFuture<>();
        try {
            io.execute(() -> {
                try { future.complete(call.call()); }
                catch (Throwable t) { future.completeExceptionally(t); }
            });
        } catch (RejectedExecutionException e) {
            future.completeExceptionally(new IllegalStateException("SCOPENET is shutting down or overloaded"));
        }
        return future;
    }

    // ---- ScopenetApi ----

    @Override public CompletableFuture<Optional<ScopenetPlayer>> getPlayer(UUID uuid) {
        return async(() -> {
            ScopenetPlayer p = player(client.post("player/info", who(uuid)));
            if (p != null) cache.put(uuid, new Cached(p, System.currentTimeMillis()));
            return Optional.ofNullable(p);
        });
    }

    @Override public Optional<ScopenetPlayer> getCachedPlayer(UUID uuid) {
        Cached c = cache.get(uuid);
        return c == null ? Optional.empty() : Optional.of(c.player());
    }

    /**
     * Cached profile for scoreboards and placeholders: returns what we have at once and
     * quietly refreshes it in the background when it is getting old.
     */
    public Optional<ScopenetPlayer> peek(UUID uuid) {
        Cached c = cache.get(uuid);
        if ((c == null || System.currentTimeMillis() - c.loadedAt() > STALE_MS) && refreshing.add(uuid)) {
            getPlayer(uuid).whenComplete((p, e) -> refreshing.remove(uuid));
        }
        return c == null ? Optional.empty() : Optional.of(c.player());
    }

    public void invalidate(UUID uuid) {
        cache.remove(uuid);
    }

    public void forget(UUID uuid) {
        cache.remove(uuid);
        refreshing.remove(uuid);
    }

    @Override public CompletableFuture<Optional<ScopenetGuild>> getGuild(String idTagOrName) {
        return async(() -> {
            JsonObject q = new JsonObject();
            q.addProperty("guild", idTagOrName);
            return Optional.ofNullable(guild(client.post("guild/info", q)));
        });
    }

    @Override public CompletableFuture<Optional<ScopenetGuild>> getPlayerGuild(UUID uuid) {
        return async(() -> {
            JsonObject q = new JsonObject();
            q.addProperty("member", uuid.toString());
            return Optional.ofNullable(guild(client.post("guild/info", q)));
        });
    }

    @Override public CompletableFuture<Double> getBalance(UUID uuid) {
        return async(() -> client.getBalance(uuid, ""));
    }

    @Override public CompletableFuture<Double> addBalance(UUID uuid, String playerName, double amount, String reason) {
        return async(() -> client.addBalance(uuid, playerName == null ? "" : playerName, amount, reason == null ? "Plugin" : reason));
    }

    @Override public CompletableFuture<XpResult> addXP(UUID uuid, XpScope scope, long amount, String reason) {
        return async(() -> {
            JsonObject body = who(uuid);
            body.addProperty("operation_id", UUID.randomUUID().toString());
            body.addProperty("scope", scope == XpScope.SERVER ? "server" : "global");
            body.addProperty("amount", amount);
            body.addProperty("reason", reason == null ? "" : reason);
            JsonObject r = client.post("player/xp", body);
            invalidate(uuid);
            return new XpResult(scope, num(r, "xp"), (int) num(r, "level"), (int) num(r, "previous_level"));
        });
    }

    @Override public CompletableFuture<Boolean> completeQuestObjective(UUID uuid, String objective, int amount) {
        return async(() -> {
            JsonObject body = who(uuid);
            body.addProperty("operation_id", UUID.randomUUID().toString());
            body.addProperty("objective", objective);
            body.addProperty("amount", amount);
            JsonObject r = client.post("player/quest-objective", body);
            invalidate(uuid);
            return r.has("advanced") && r.get("advanced").getAsBoolean();
        });
    }

    @Override public CompletableFuture<List<ScopenetFriend>> getFriends(UUID uuid) {
        return async(() -> {
            List<ScopenetFriend> out = new ArrayList<>();
            JsonArray list = client.post("player/friends", who(uuid)).getAsJsonArray("friends");
            for (JsonElement e : list) {
                JsonObject f = e.getAsJsonObject();
                out.add(new ScopenetFriend(UUID.fromString(str(f, "uuid")), str(f, "name"), f.get("online").getAsBoolean(), str(f, "playing_on")));
            }
            return List.copyOf(out);
        });
    }

    @Override public void close() {
        io.shutdownNow();
    }

    @FunctionalInterface
    private interface Callable<T> {
        T call() throws Exception;
    }
}
