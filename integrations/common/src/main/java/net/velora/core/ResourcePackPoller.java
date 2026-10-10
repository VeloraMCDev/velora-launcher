package net.velora.core;

import com.google.gson.JsonObject;
import java.util.*;
import java.util.function.Supplier;

/**
 * Refreshes pack configuration in the background and offers each revision once per player session. A pack is offered a few seconds
 * after the player appears, not on the very first tick: applying it while the client is still building its first chunks makes it
 * reload resources mid-load, which is when the block atlas and the already-baked block models can end up out of step (grass solid
 * green, a stair facing without a texture, one texture everywhere).
 */
public final class ResourcePackPoller {
    private final Env env;
    private final Supplier<String> panelUrl;
    private final Map<UUID, String> offered = new HashMap<>();
    private final Map<UUID, Long> seen = new HashMap<>();
    /** How long a player must have been online before the pack is offered. */
    static final long SETTLE_MILLIS = 5_000;
    private long next;
    private boolean running, enabled, required;
    private String hash = "";
    public ResourcePackPoller(Env env, Supplier<String> panelUrl) { this.env = env; this.panelUrl = panelUrl; }
    public void tick() {
        Set<UUID> online = new HashSet<>();
        for (CorePlayer p : env.platform.online()) {
            online.add(p.uuid());
            long since = seen.computeIfAbsent(p.uuid(), k -> env.clock.getAsLong());
            if (env.clock.getAsLong() - since < SETTLE_MILLIS) continue;
            String revision = hash + required;
            if (enabled && !revision.equals(offered.get(p.uuid())) && env.platform.offerResourcePack(p.uuid(), panelUrl.get().replaceAll("/+$", "") + "/api/v1/resource-pack.zip?revision=" + hash, hash, required)) offered.put(p.uuid(), revision);
        }
        offered.keySet().retainAll(online);
        seen.keySet().retainAll(online);
        retries.keySet().retainAll(online);
        long now = env.clock.getAsLong();
        if (running || now < next) return;
        running = true; next = now + 60_000;
        env.io(() -> env.panel.call("resource-pack", new JsonObject()).getAsJsonObject(), r -> {
            enabled = r.has("enabled") && r.get("enabled").getAsBoolean();
            required = r.has("required") && r.get("required").getAsBoolean();
            hash = r.has("sha1") ? r.get("sha1").getAsString() : "";
            if (!hash.matches("[a-f0-9]{40}")) enabled = false;
            if (!enabled) offered.clear();
            running = false;
        }, e -> { running = false; env.log.warning("Resource pack sync failed: " + e); });
    }
    /** Retries allowed per player and session when the client could not download or apply the pack. */
    static final int RETRIES = 2;
    private final Map<UUID, Integer> retries = new HashMap<>();

    /**
     * The client reported that the pack failed (download error, bad URL, could not reload). A flaky download is the usual cause, so the
     * pack is offered again after the settle delay, a couple of times per session. Returns false when the retries are used up.
     */
    public boolean failed(UUID id) {
        int used = retries.merge(id, 1, Integer::sum);
        if (used > RETRIES) return false;
        offered.remove(id);
        seen.put(id, env.clock.getAsLong());
        return true;
    }

    public void left(UUID id) { offered.remove(id); seen.remove(id); retries.remove(id); }
}
