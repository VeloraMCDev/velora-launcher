package net.velora.core;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

import java.util.Optional;
import java.util.UUID;

/**
 * Collects what the panel queued for this server (a teleport request or message sent from the map, an invitation
 * notice) every few seconds, and carries it out with the server's normal rules.
 */
public final class ActionPoller {
    /** Teleport requests use each platform's own TPA, so they obey its rules and permissions. */
    public interface Tpa { void request(CorePlayer requester, CorePlayer target); }

    public static final long EVERY_MS = 3_000;

    private final Env env;
    private final Tpa tpa;
    private long next;
    private volatile boolean running;

    public ActionPoller(Env env, Tpa tpa) { this.env = env; this.tpa = tpa; }

    /** Call often (every tick or second) on the server thread; it asks the panel at most every few seconds. */
    public void tick() {
        long now = env.clock.getAsLong();
        if (running || now < next) return;
        next = now + EVERY_MS;
        running = true;
        env.platform.runAsync(() -> {
            try {
                JsonElement answer = env.panel.call("actions/poll", new JsonObject());
                JsonArray actions = answer.isJsonObject() && answer.getAsJsonObject().has("actions") ? answer.getAsJsonObject().getAsJsonArray("actions") : new JsonArray();
                if (actions.size() > 0) env.platform.runMain(() -> actions.forEach(a -> apply(a.getAsJsonObject())));
            } catch (Exception e) {
                // The panel is briefly unreachable: try again on the next round.
            } finally {
                running = false;
            }
        });
    }

    private static String text(JsonObject o, String k) { return o.has(k) && o.get(k).isJsonPrimitive() ? o.get(k).getAsString() : ""; }

    private Optional<CorePlayer> player(String uuid) {
        try { return env.platform.player(UUID.fromString(uuid)); } catch (IllegalArgumentException e) { return Optional.empty(); }
    }

    void apply(JsonObject a) {
        String kind = text(a, "kind");
        Optional<CorePlayer> to = player(text(a, "to_uuid"));
        if (to.isEmpty()) return; // left the server since it was queued
        switch (kind) {
            case "notify" -> to.get().send(text(a, "text"));
            case "message" -> to.get().send(Format.GRAY + "[" + Format.AQUA + "Map" + Format.GRAY + "] " + Format.YELLOW + text(a, "from_name") + Format.GRAY + ": " + Format.WHITE + text(a, "text"));
            case "tpa" -> {
                Optional<CorePlayer> from = player(text(a, "from_uuid"));
                if (from.isEmpty()) { to.get().send(Format.GRAY + text(a, "from_name") + " asked to teleport to you, but has left."); return; }
                if (!from.get().hasPermission("velora.command.tpa")) { from.get().send(Format.RED + "You do not have permission to send teleport requests."); return; }
                tpa.request(from.get(), to.get());
            }
            default -> { }
        }
    }
}
