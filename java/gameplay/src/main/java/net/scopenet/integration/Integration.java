package net.scopenet.integration;

import com.google.gson.*;
import net.scopenet.integration.map.MapPosition;
import net.scopenet.integration.map.MapSink;
import net.scopenet.integration.map.MapProducerFactory;
import java.util.UUID;
import java.util.concurrent.*;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.function.*;

public final class Integration implements AutoCloseable {
    public static final String UNAVAILABLE = "Velora sign-in is unavailable. Please try again shortly.";
    public final Activity activity = new Activity();
    private volatile Settings settings;
    private final PanelClient client;
    private final JsonObject hello;
    private final Consumer<String> log;
    private final BiConsumer<UUID, String> kick;
    private final ExecutorService syncWorker = Executors.newSingleThreadExecutor(r -> daemon(r, "scopenet-sync"));
    private final ExecutorService loginWorker = new ThreadPoolExecutor(2, 4, 30, TimeUnit.SECONDS,
            new ArrayBlockingQueue<>(64), r -> daemon(r, "scopenet-login"), new ThreadPoolExecutor.AbortPolicy());
    private final AtomicBoolean syncing = new AtomicBoolean();
    private volatile boolean closed;
    private volatile Consumer<JsonArray> notificationHandler;
    private volatile ChatLayout chat = ChatLayout.DEFAULT;
    private volatile net.scopenet.core.Utilities utilities = net.scopenet.core.Utilities.DEFAULT;
    private volatile MapSink map;
    private volatile java.util.function.Supplier<String> mapOverlay;
    private boolean registered;
    private JsonObject pending;
    private long nextSync;
    private long ticks;
    private long sampleStart = System.nanoTime();

    public Integration(Settings settings, String software, String mcVersion, String version,
                       boolean onlineMode, int maxPlayers, Consumer<String> log, BiConsumer<UUID, String> kick) {
        if (!onlineMode) throw new IllegalArgumentException("Velora requires online-mode=true and authlib-injector");
        this.settings = settings;
        this.client = new PanelClient(settings);
        this.log = log;
        this.kick = kick;
        hello = new JsonObject();
        hello.addProperty("software", software);
        hello.addProperty("mc_version", mcVersion);
        hello.addProperty("plugin_version", version);
        hello.addProperty("online_mode", onlineMode);
        hello.addProperty("max_players", maxPlayers);
    }

    /** Null means allowed. Errors, missing settings and overload always deny. */
    public CompletableFuture<String> login(UUID id, String name, String ip) {
        if (closed) return CompletableFuture.completedFuture(UNAVAILABLE);
        try {
            return CompletableFuture.supplyAsync(() -> {
                JsonObject request = new JsonObject();
                request.addProperty("uuid", id.toString());
                request.addProperty("name", name);
                if (ip != null) request.addProperty("ip", ip);
                try { return PanelClient.verdict(client.post("login", request)); }
                catch (Exception e) {
                    if (e instanceof InterruptedException) Thread.currentThread().interrupt();
                    return UNAVAILABLE;
                }
            }, loginWorker).completeOnTimeout(UNAVAILABLE, 6, TimeUnit.SECONDS);
        } catch (RejectedExecutionException e) {
            return CompletableFuture.completedFuture(UNAVAILABLE);
        }
    }

    /** Called on the server thread. Network and JSON work runs on the worker. */
    public void tick() {
        if (closed) return;
        ticks++;
        long now = System.nanoTime();
        if (now < nextSync || !syncing.compareAndSet(false, true)) return;
        double tps = Math.min(20, ticks * 1_000_000_000.0 / Math.max(1, now - sampleStart));
        ticks = 0;
        sampleStart = now;
        nextSync = now + TimeUnit.SECONDS.toNanos(30);
        syncWorker.execute(() -> {
            try { sync(tps); }
            catch (Exception e) {
                if (e instanceof InterruptedException) Thread.currentThread().interrupt();
                String msg = e.getMessage() != null && !e.getMessage().isBlank()
                        ? e.getClass().getSimpleName() + ": " + e.getMessage()
                        : e.getClass().getSimpleName();
                log.accept("Velora sync failed to connect to panel at " + settings.panel() + "; retained batch will be retried (" + msg + ")");
            } finally { syncing.set(false); }
        });
    }

    private void sync(double tps) throws Exception {
        if (!registered) {
            JsonObject response = client.post("hello", hello);
            if (!response.has("server_id")) throw new IllegalStateException("Invalid hello response");
            registered = true;
        }
        if (pending == null) {
            pending = activity.drain(tps);
            JsonObject features = new JsonObject();
            features.addProperty("leveling_enabled", settings.levelingEnabled());
            features.addProperty("global_xp_multiplier", settings.globalXpMultiplier());
            features.addProperty("server_xp_multiplier", settings.serverXpMultiplier());
            features.addProperty("quests_enabled", settings.questsEnabled());
            features.addProperty("achievements_enabled", settings.achievementsEnabled());
            features.addProperty("guilds_enabled", settings.guildsEnabled());
            features.addProperty("land_claiming_enabled", settings.landClaimingEnabled());
            features.addProperty("social_enabled", settings.socialEnabled());
            pending.add("features", features);
        }
        JsonObject response = client.post("sync", pending);
        if (!response.has("ok") || !response.get("ok").getAsBoolean() || !response.has("kick")
                || !response.get("kick").isJsonArray()) throw new IllegalStateException("Invalid sync response");
        pending = null;
        if (response.has("chat") && response.get("chat").isJsonObject()) chat = ChatLayout.fromJson(response.getAsJsonObject("chat"));
        if (response.has("utilities") && response.get("utilities").isJsonObject()) {
            utilities = net.scopenet.core.Utilities.fromJson(response.getAsJsonObject("utilities"),
                    response.has("custom_items") && response.get("custom_items").isJsonArray() ? response.getAsJsonArray("custom_items") : null,
                    response.has("content") && response.get("content").isJsonArray() ? response.getAsJsonArray("content") : null);
        }
        Consumer<JsonArray> handler = notificationHandler;
        if (handler != null && response.has("notifications") && response.get("notifications").isJsonArray()
                && response.getAsJsonArray("notifications").size() > 0) {
            try { handler.accept(response.getAsJsonArray("notifications")); }
            catch (RuntimeException e) { log.accept("Velora could not process panel events: " + e.getMessage()); }
        }
        for (JsonElement element : response.getAsJsonArray("kick")) {
            try {
                JsonObject entry = element.getAsJsonObject();
                String message = entry.has("message") && !entry.get("message").isJsonNull()
                        ? entry.get("message").getAsString() : "Your server access was revoked.";
                kick.accept(UUID.fromString(entry.get("uuid").getAsString()), message);
            } catch (RuntimeException e) { log.accept("Velora ignored a malformed kick entry"); }
        }
    }

    public void reload(Settings next) {
        client.updateSettings(next);
        settings = next;
    }

    /** Utility-command settings, kits and custom items from the panel (defaults until the first sync). */
    public net.scopenet.core.Utilities utilities() {
        return utilities;
    }

    /** The chat layout the admin designed in the panel (defaults until the first sync). */
    public ChatLayout chat() {
        return chat;
    }

    public Settings settings() {
        return settings;
    }

    public PanelClient client() {
        return client;
    }

    /**
     * Receive events the panel queued for this server (level-ups, achievements,
     * guild changes) with each sync. Called on the sync worker thread.
     */
    public void onNotifications(Consumer<JsonArray> handler) {
        this.notificationHandler = handler;
    }

    /**
     * Start the Velora Map: render this world's region files to tiles and send them, player positions and the claims
     * overlay to the panel. {@code worldRoot} is the level folder (e.g. {@code world}); {@code cacheDir} holds the rendered tiles.
     */
    public void enableMap(java.nio.file.Path worldRoot, java.nio.file.Path cacheDir) {
        if (closed || map != null) return;
        MapProducerFactory producer = null;
        try {
            for (MapProducerFactory candidate : java.util.ServiceLoader.load(MapProducerFactory.class, MapProducerFactory.class.getClassLoader())) {
                if (candidate.apiVersion() == MapProducerFactory.API_VERSION) { producer = candidate; break; }
                log.accept("Velora ignored an unsupported map producer API version: " + candidate.apiVersion());
            }
        } catch (java.util.ServiceConfigurationError | RuntimeException e) {
            log.accept("Velora map producer could not load: " + e.getMessage());
            return;
        }
        if (producer == null) {
            log.accept("Velora map producer is not installed; map rendering is unavailable.");
            return;
        }
        map = producer.create(client.transport(), worldRoot, cacheDir, log);
        if (mapOverlay != null) map.setOverlay(mapOverlay);
        map.start();
    }

    /** Where the map gets its claims and pins (JSON text); may be called before or after {@link #enableMap}. */
    public void setMapOverlay(java.util.function.Supplier<String> overlay) {
        mapOverlay = overlay;
        MapSink current = map;
        if (current != null) current.setOverlay(overlay);
    }

    /** Hand over a snapshot of player positions; call about once a second on the server thread. */
    public void mapPlayers(java.util.List<? extends MapPosition> players) {
        MapSink current = map;
        if (current != null) current.offerPlayers(players);
    }

    /** The world was just saved: pick up its changed regions soon. */
    public void mapSaved() {
        MapSink current = map;
        if (current != null) current.requestScan();
    }

    public String mapStatus() {
        MapSink current = map;
        return current == null ? "Map is not started. Check map.enabled in the server config and restart." : current.status();
    }

    public ChunkCheckResult checkChunk(String dimension, int chunkX, int chunkZ, UUID uuid) {
        return client.checkChunk(dimension, chunkX, chunkZ, uuid);
    }

    /** An admin claim's flag here, or null when the chunk is not in an admin claim. */
    public Boolean adminFlag(String dimension, int chunkX, int chunkZ, String flag) {
        return client.adminFlag(dimension, chunkX, chunkZ, flag);
    }

    /**
     * Does the claim here allow {@code flag}? True in wilderness. Admin claims use their flags; guild land uses the rules its guild
     * chose (visitor rules only ever apply to non-members: the callers check membership first).
     */
    public boolean adminAllows(String dimension, int chunkX, int chunkZ, String flag) {
        Boolean allowed = client.claimFlag(dimension, chunkX, chunkZ, flag);
        return allowed == null || allowed;
    }

    /** May this player change land here? See {@link ClaimIndex#mayModify}. */
    public boolean mayModify(String dimension, int chunkX, int chunkZ, java.util.UUID player, String flag) {
        return client.mayModify(dimension, chunkX, chunkZ, player, flag);
    }

    /** The flag at a chunk, or null for wilderness. */
    public Boolean claimFlag(String dimension, int chunkX, int chunkZ, String flag) {
        return client.claimFlag(dimension, chunkX, chunkZ, flag);
    }

    /** Environmental protection: is this chunk claimed by anyone? */
    public boolean isClaimed(String dimension, int chunkX, int chunkZ) {
        return client.isClaimed(dimension, chunkX, chunkZ);
    }

    /** Begin keeping the local claim index in sync with the panel. */
    public void startClaimSync() {
        client.startClaimSync(log);
    }

    @Override public void close() {
        closed = true;
        MapSink current = map;
        if (current != null) current.close();
        client.close();
        loginWorker.shutdownNow();
        activity.leaveAll();
        Future<?> flush = syncWorker.submit(() -> {
            try {
                // A retained retry may contain an earlier online snapshot.
                sync(20);
                sync(20);
            } catch (Exception e) { log.accept("Velora final sync failed; unsent activity may be lost"); }
        });
        try { flush.get(12, TimeUnit.SECONDS); }
        catch (InterruptedException e) { Thread.currentThread().interrupt(); }
        catch (ExecutionException | TimeoutException e) { log.accept("Velora final sync did not finish"); }
        finally { syncWorker.shutdownNow(); }
    }

    private static Thread daemon(Runnable task, String name) {
        Thread thread = new Thread(task, name);
        thread.setDaemon(true);
        return thread;
    }
}
