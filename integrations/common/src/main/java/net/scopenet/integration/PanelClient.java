package net.scopenet.integration;

import com.google.gson.*;
import java.io.IOException;
import java.net.http.*;

public final class PanelClient {
    private volatile Settings settings;
    private final ClaimIndex claimIndex = new ClaimIndex();
    private final java.util.concurrent.ScheduledExecutorService claimSync = java.util.concurrent.Executors.newSingleThreadScheduledExecutor(task -> {
        Thread t = new Thread(task, "scopenet-claims"); t.setDaemon(true); return t;
    });
    private final java.util.concurrent.atomic.AtomicBoolean claimRefreshQueued = new java.util.concurrent.atomic.AtomicBoolean();
    private volatile boolean claimSyncStarted;
    private volatile long claimFailureLogged;
    private volatile java.util.function.Consumer<String> claimLog = m -> {};

    public void updateSettings(Settings next) { settings = next; requestClaimRefresh(); }
    public void close() { claimSync.shutdownNow(); }
    public ClaimIndex claims() { return claimIndex; }

    /**
     * Keep the local claim index fresh: load it now (so protection is on from
     * the first tick) and then check the panel's revision every few seconds.
     */
    public void startClaimSync(java.util.function.Consumer<String> log) {
        if (claimSyncStarted) return;
        claimSyncStarted = true;
        claimLog = log;
        claimSync.scheduleWithFixedDelay(this::refreshClaimsQuietly, 0, 5, java.util.concurrent.TimeUnit.SECONDS);
    }

    /** Pull a fresh index soon, e.g. right after this server changed a claim. */
    public void requestClaimRefresh() {
        if (!claimSyncStarted || !claimRefreshQueued.compareAndSet(false, true)) return;
        try { claimSync.execute(() -> { claimRefreshQueued.set(false); refreshClaimsQuietly(); }); }
        catch (java.util.concurrent.RejectedExecutionException e) { claimRefreshQueued.set(false); }
    }

    /** One synchronous refresh. Returns true when the index changed. */
    public boolean refreshClaims() throws IOException, InterruptedException {
        JsonObject body = new JsonObject();
        String revision = claimIndex.revision();
        if (revision != null) body.addProperty("revision", revision);
        return claimIndex.apply(post("guilds/claim-index", body));
    }

    private void refreshClaimsQuietly() {
        if (!settings.guildsEnabled() || !settings.landClaimingEnabled()) return;
        try {
            boolean first = !claimIndex.loaded();
            boolean changed = refreshClaims();
            if (first && changed) claimLog.accept("Velora claim protection loaded (" + claimIndex.claimCount() + " claimed chunks)");
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
        } catch (Exception e) {
            // Keep enforcing the last copy we have; just don't spam the log.
            long now = System.nanoTime();
            if (now - claimFailureLogged > java.util.concurrent.TimeUnit.MINUTES.toNanos(5)) {
                claimFailureLogged = now;
                claimLog.accept("Velora couldn't refresh claims from the panel (" + e.getMessage() + "); using the last known copy");
            }
        }
    }

    public static class HttpFailure extends PanelTransport.HttpFailure {
        public HttpFailure(int status, String message) { super(status, message); }
    }
    private final PanelTransport transport = new PanelTransport(
            () -> new ServerConnection(settings.panel(), settings.token()), HttpFailure::new);

    public PanelClient(Settings settings) { this.settings = settings; }
    public HttpClient http() { return transport.http(); }
    public PanelTransport transport() { return transport; }
    public Settings settings() { return settings; }
    public JsonElement postElement(String endpoint, JsonObject payload) throws IOException, InterruptedException { return transport.postElement(endpoint, payload); }
    public JsonObject get(String endpoint) throws IOException, InterruptedException { return transport.get(endpoint); }
    public JsonObject postBytes(String endpoint, byte[] body) throws IOException, InterruptedException { return transport.postBytes(endpoint, body); }
    public JsonObject post(String endpoint, JsonObject payload) throws IOException, InterruptedException { return transport.post(endpoint, payload); }

    public static String verdict(JsonObject response) throws IOException {
        JsonElement allowed = response.get("allowed");
        if (allowed == null || !allowed.isJsonPrimitive() || !allowed.getAsJsonPrimitive().isBoolean()) {
            throw new IOException("Missing login verdict");
        }
        if (allowed.getAsBoolean()) return null;
        JsonElement message = response.get("message");
        return message != null && message.isJsonPrimitive() && message.getAsJsonPrimitive().isString()
                ? message.getAsString() : "You do not have access to this server.";
    }

    /** Local lookup; never touches the network. */
    public ChunkCheckResult checkChunk(String dimension, int chunkX, int chunkZ, java.util.UUID uuid) {
        if (!settings.guildsEnabled() || !settings.landClaimingEnabled()) return new ChunkCheckResult(false, true, null, null);
        return claimIndex.check(dimension, chunkX, chunkZ, uuid);
    }

    public boolean isClaimed(String dimension, int chunkX, int chunkZ) {
        return settings.guildsEnabled() && settings.landClaimingEnabled() && claimIndex.isClaimed(dimension, chunkX, chunkZ);
    }

    /** May this player change land here (members always; visitors only where the claim's {@code flag} is on)? Local lookup. */
    public boolean mayModify(String dimension, int chunkX, int chunkZ, java.util.UUID uuid, String flag) {
        if (!settings.guildsEnabled() || !settings.landClaimingEnabled()) return true;
        return claimIndex.mayModify(dimension, chunkX, chunkZ, uuid, flag);
    }

    /** Any claim's flag at this chunk (admin claim or guild land), or null for wilderness. See {@link ClaimIndex#claimFlag}. */
    public Boolean claimFlag(String dimension, int chunkX, int chunkZ, String flag) {
        return settings.guildsEnabled() && settings.landClaimingEnabled() ? claimIndex.claimFlag(dimension, chunkX, chunkZ, flag) : null;
    }

    /** An admin claim's flag at this chunk, or null when it is not an admin claim. See {@link ClaimIndex#adminFlag}. */
    public Boolean adminFlag(String dimension, int chunkX, int chunkZ, String flag) {
        return settings.guildsEnabled() && settings.landClaimingEnabled() ? claimIndex.adminFlag(dimension, chunkX, chunkZ, flag) : null;
    }

    public JsonObject claimChunk(String dimension, int chunkX, int chunkZ, java.util.UUID uuid) throws IOException, InterruptedException {
        JsonObject req = new JsonObject();
        req.addProperty("uuid", uuid.toString());
        req.addProperty("dimension", dimension);
        req.addProperty("chunk_x", chunkX);
        req.addProperty("chunk_z", chunkZ);
        JsonObject result = post("guilds/claim", req);
        requestClaimRefresh();
        return result;
    }

    public JsonObject unclaimChunk(String dimension, int chunkX, int chunkZ, java.util.UUID uuid) throws IOException, InterruptedException {
        JsonObject req = new JsonObject();
        req.addProperty("uuid", uuid.toString());
        req.addProperty("dimension", dimension);
        req.addProperty("chunk_x", chunkX);
        req.addProperty("chunk_z", chunkZ);
        JsonObject result = post("guilds/unclaim", req);
        requestClaimRefresh();
        return result;
    }

    public JsonObject getPlayerGuild(java.util.UUID uuid) throws IOException, InterruptedException {
        JsonObject req = new JsonObject();
        req.addProperty("uuid", uuid.toString());
        return post("guilds/player", req);
    }

    public JsonObject createGuild(java.util.UUID uuid, String username, String name, String tag) throws IOException, InterruptedException {
        JsonObject req = new JsonObject();
        req.addProperty("uuid", uuid.toString());
        req.addProperty("username", username);
        req.addProperty("name", name);
        req.addProperty("tag", tag);
        JsonObject result = post("guilds/create", req);
        requestClaimRefresh();
        return result;
    }

    public JsonObject leaveGuild(java.util.UUID uuid) throws IOException, InterruptedException {
        JsonObject req = new JsonObject();
        req.addProperty("uuid", uuid.toString());
        JsonObject result = post("guilds/leave", req);
        requestClaimRefresh();
        return result;
    }

    public double getBalance(java.util.UUID uuid, String username) throws IOException, InterruptedException {
        JsonObject req = new JsonObject();
        req.addProperty("uuid", uuid.toString());
        req.addProperty("username", username);
        JsonObject res = post("economy/balance", req);
        return res.has("balance") ? res.get("balance").getAsDouble() : 0.0;
    }

    public double transfer(java.util.UUID fromUuid, String fromName, java.util.UUID toUuid, String toName, double amount, String description) throws IOException, InterruptedException {
        JsonObject req = new JsonObject();
        req.addProperty("from_uuid", fromUuid.toString());
        req.addProperty("from_name", fromName);
        req.addProperty("to_uuid", toUuid.toString());
        req.addProperty("to_name", toName);
        req.addProperty("amount", amount);
        req.addProperty("description", description);
        JsonObject res = post("economy/transfer", req);
        return res.has("from_balance") ? res.get("from_balance").getAsDouble() : 0.0;
    }

    public void syncBalance(java.util.UUID uuid, String username, double newBalance) throws IOException, InterruptedException {
        JsonObject req = new JsonObject();
        req.addProperty("uuid", uuid.toString());
        req.addProperty("username", username);
        req.addProperty("new_balance", newBalance);
        post("economy/sync-balance", req);
    }

    public JsonArray getBaltop() throws IOException, InterruptedException {
        JsonElement el = postElement("economy/baltop", new JsonObject());
        return el.isJsonArray() ? el.getAsJsonArray() : new JsonArray();
    }

    public JsonObject marketList(java.util.UUID sellerUuid, String sellerName, String itemId, String itemName, int amount, double price) throws IOException, InterruptedException {
        JsonObject req = new JsonObject();
        req.addProperty("operation_id", java.util.UUID.randomUUID().toString());
        req.addProperty("seller_uuid", sellerUuid.toString());
        req.addProperty("seller_name", sellerName);
        req.addProperty("item_id", itemId);
        req.addProperty("item_name", itemName);
        req.addProperty("amount", amount);
        req.addProperty("price", price);
        return post("economy/market/list", req);
    }

    public JsonObject marketBuy(long listingId, java.util.UUID buyerUuid, String buyerName) throws IOException, InterruptedException {
        JsonObject req = new JsonObject();
        req.addProperty("operation_id", java.util.UUID.randomUUID().toString());
        req.addProperty("listing_id", listingId);
        req.addProperty("buyer_uuid", buyerUuid.toString());
        req.addProperty("buyer_name", buyerName);
        return post("economy/market/buy", req);
    }

    /**
     * Atomically add (or subtract, if amount is negative) to a player's balance.
     * Returns the new balance. This avoids the read-modify-write race in the old
     * getBalance + syncBalance pattern.
     */
    public double addBalance(java.util.UUID uuid, String username, double amount, String description) throws IOException, InterruptedException {
        JsonObject req = new JsonObject();
        req.addProperty("operation_id", java.util.UUID.randomUUID().toString());
        req.addProperty("uuid", uuid.toString());
        req.addProperty("username", username);
        req.addProperty("delta", amount);
        req.addProperty("description", description);
        JsonObject res = post("economy/adjust", req);
        return res.has("balance") ? res.get("balance").getAsDouble() : 0.0;
    }

    /** Fetch active player marketplace listings. */
    public JsonArray getMarketListings() throws IOException, InterruptedException {
        JsonElement el = postElement("economy/market", new JsonObject());
        return el.isJsonArray() ? el.getAsJsonArray() : new JsonArray();
    }
}
