package net.scopenet.core;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

import java.util.UUID;

/** Shows the panel's bell notifications to players who are in game ("Your guild was renamed", "You won an auction!"). */
public final class PanelMessages {
    private PanelMessages() {}

    /** Handles the {@code chat_message} entries of a sync's notifications; other kinds are ignored. */
    public static void deliver(Platform platform, JsonArray notifications) {
        for (JsonElement element : notifications) {
            if (!element.isJsonObject()) continue;
            JsonObject n = element.getAsJsonObject();
            if (!n.has("kind") || !"chat_message".equals(n.get("kind").getAsString()) || !n.has("uuid")) continue;
            JsonObject d = n.has("data") && n.get("data").isJsonObject() ? n.getAsJsonObject("data") : new JsonObject();
            UUID uuid;
            try { uuid = UUID.fromString(n.get("uuid").getAsString()); } catch (IllegalArgumentException e) { continue; }
            String title = d.has("title") ? Format.plain(d.get("title").getAsString()) : "";
            String body = d.has("body") ? Format.plain(d.get("body").getAsString()) : "";
            if (title.isBlank() && body.isBlank()) continue;
            platform.runMain(() -> platform.player(uuid).ifPresent(p -> p.send(Format.GOLD + "[Velora] " + Format.YELLOW + title + (body.isBlank() ? "" : Format.GRAY + " - " + body))));
        }
    }
}
