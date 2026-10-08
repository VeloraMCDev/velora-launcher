package net.scopenet.paper.compat;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.scopenet.api.XpScope;
import net.scopenet.api.event.*;
import org.bukkit.Bukkit;
import org.bukkit.event.Event;
import org.bukkit.plugin.java.JavaPlugin;
import java.util.UUID;
import java.util.logging.Logger;

/** Turns the events the panel queued for this server into Bukkit events, on the server thread. */
public final class EventBridge {
    private final JavaPlugin plugin;
    private final ScopenetApiImpl api;
    private final Logger log;

    public EventBridge(JavaPlugin plugin, ScopenetApiImpl api, Logger log) {
        this.plugin = plugin;
        this.api = api;
        this.log = log;
    }

    /** Called on the sync worker thread. */
    public void accept(JsonArray notifications) {
        for (JsonElement element : notifications) {
            try {
                JsonObject n = element.getAsJsonObject();
                UUID uuid = UUID.fromString(n.get("uuid").getAsString());
                Event event = toEvent(n.get("kind").getAsString(), uuid, n.has("data") && n.get("data").isJsonObject() ? n.getAsJsonObject("data") : new JsonObject());
                api.invalidate(uuid);
                if (event == null) continue;
                Bukkit.getScheduler().runTask(plugin, () -> Bukkit.getPluginManager().callEvent(event));
            } catch (RuntimeException e) {
                log.warning("Ignoring a malformed SCOPENET event: " + e.getMessage());
            }
        }
    }

    static Event toEvent(String kind, UUID uuid, JsonObject d) {
        return switch (kind) {
            case "level_up" -> new ScopenetLevelUpEvent(uuid, "server".equals(text(d, "scope")) ? XpScope.SERVER : XpScope.GLOBAL,
                    (int) number(d, "level"), (int) number(d, "previous"), number(d, "xp"));
            case "achievement" -> new AchievementUnlockEvent(uuid, text(d, "id"), text(d, "title"), number(d, "xp"));
            case "guild_join" -> new GuildJoinEvent(uuid, text(d, "guild_id"), text(d, "guild"), text(d, "tag"), text(d, "role"));
            case "guild_leave" -> new GuildLeaveEvent(uuid, text(d, "guild_id"), text(d, "guild"), text(d, "tag"));
            default -> null;
        };
    }

    private static String text(JsonObject o, String key) {
        JsonElement e = o.get(key);
        return e != null && e.isJsonPrimitive() ? e.getAsString() : null;
    }

    private static long number(JsonObject o, String key) {
        JsonElement e = o.get(key);
        return e != null && e.isJsonPrimitive() && e.getAsJsonPrimitive().isNumber() ? e.getAsLong() : 0;
    }
}
