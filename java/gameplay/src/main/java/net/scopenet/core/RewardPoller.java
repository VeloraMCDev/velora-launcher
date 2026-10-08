package net.scopenet.core;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

import java.util.Optional;
import java.util.UUID;

/**
 * Collects the rewards the panel owes to players on this server (items, permissions, groups, commands, messages) and carries
 * them out, then tells the panel what worked. The panel only hands rewards over for players who are online here.
 */
public final class RewardPoller {
    public static final long EVERY_MS = 5_000;

    private final Env env;
    private volatile RewardRules rules;
    private long next;
    private volatile boolean running;

    public RewardPoller(Env env, RewardRules rules) { this.env = env; this.rules = rules; }

    public void rules(RewardRules rules) { this.rules = rules; }

    /** Call often on the server thread; it asks the panel at most every few seconds. */
    public void tick() {
        if (!rules.enabled()) return;
        long now = env.clock.getAsLong();
        if (running || now < next) return;
        next = now + EVERY_MS;
        running = true;
        env.platform.runAsync(() -> {
            try {
                JsonElement answer = env.panel.call("rewards/poll", new JsonObject());
                JsonArray list = answer.isJsonObject() && answer.getAsJsonObject().has("deliveries") ? answer.getAsJsonObject().getAsJsonArray("deliveries") : new JsonArray();
                if (list.size() > 0) env.platform.runMain(() -> carryOut(list));
            } catch (Exception e) {
                // The panel is briefly unreachable: try again on the next round.
            } finally {
                running = false;
            }
        });
    }

    private static String text(JsonObject o, String k) { return o.has(k) && o.get(k).isJsonPrimitive() ? o.get(k).getAsString() : ""; }

    /** Server thread. Applies each delivery, then reports back from the background. */
    void carryOut(JsonArray list) {
        JsonArray done = new JsonArray(), failed = new JsonArray();
        for (JsonElement el : list) {
            JsonObject d = el.getAsJsonObject();
            long id = d.get("id").getAsLong();
            String error;
            try {
                error = apply(d);
            } catch (RuntimeException e) {
                error = e.getClass().getSimpleName();
            }
            if (error == null) done.add(id);
            else {
                JsonObject f = new JsonObject();
                f.addProperty("id", id);
                f.addProperty("error", error);
                failed.add(f);
            }
        }
        JsonObject body = new JsonObject();
        body.add("done", done);
        body.add("failed", failed);
        env.platform.runAsync(() -> {
            try { env.panel.call("rewards/ack", body); } catch (Exception e) { /* the panel re-offers it, and a second try is harmless */ }
        });
    }

    /** Null when carried out, otherwise why not. */
    String apply(JsonObject d) {
        UUID uuid;
        try { uuid = UUID.fromString(text(d, "uuid")); } catch (IllegalArgumentException e) { return "bad player id"; }
        Optional<CorePlayer> player = env.platform.player(uuid);
        if (player.isEmpty()) return "player left"; // retried when they are back
        JsonObject a = d.has("payload") && d.get("payload").isJsonObject() ? d.getAsJsonObject("payload") : new JsonObject();
        String name = player.get().name();
        RewardRules r = rules;
        switch (text(d, "kind")) {
            case "item" -> {
                int amount = a.has("amount") ? a.get("amount").getAsInt() : 1;
                if (!env.platform.giveItem(uuid, text(a, "item"), amount)) return "unknown item " + text(a, "item");
                player.get().send(Format.GREEN + "You received " + Format.WHITE + amount + "x " + prettyItem(text(a, "item")) + Format.GREEN + "!");
            }
            case "custom_item" -> {
                if (!a.has("spec")) return "custom item definition is missing";
                int amount = a.has("amount") ? a.get("amount").getAsInt() : 1;
                ItemSpec spec = ItemSpec.fromJson(a.getAsJsonObject("spec")).withAmount(amount);
                if (!env.platform.giveSpec(uuid, spec)) return "custom item unavailable on this server";
                player.get().send(Format.GREEN + "You received " + Format.WHITE + amount + "x " + prettyItem(text(a, "custom")) + Format.GREEN + "!");
            }
            case "permission" -> {
                boolean value = !a.has("value") || a.get("value").getAsBoolean();
                long minutes = a.has("minutes") ? a.get("minutes").getAsLong() : 0;
                String template = minutes > 0 ? r.permissionTempCommand() : value ? r.permissionCommand() : r.permissionDenyCommand();
                String command = template.replace("{player}", name).replace("{uuid}", uuid.toString()).replace("{node}", text(a, "node"))
                        .replace("{value}", Boolean.toString(value)).replace("{duration}", minutes + "m");
                if (!env.platform.console(command)) return "the permissions command did not run (is LuckPerms installed?)";
                player.get().send(Format.GREEN + "You gained a new permission: " + Format.WHITE + text(a, "node") + Format.GREEN + ".");
            }
            case "group" -> {
                String command = r.groupCommand().replace("{player}", name).replace("{uuid}", uuid.toString()).replace("{group}", text(a, "group"));
                if (!env.platform.console(command)) return "the group command did not run (is LuckPerms installed?)";
                player.get().send(Format.GREEN + "You joined the " + Format.WHITE + text(a, "group") + Format.GREEN + " group.");
            }
            case "command" -> {
                if (!r.allowCommands()) return "custom commands are switched off on this server (rewards.allow-commands)";
                String command = text(a, "command").replace("{player}", name).replace("{uuid}", uuid.toString());
                if (!env.platform.console(command)) return "command not found";
            }
            case "message" -> player.get().send(Format.GOLD + text(a, "text"));
            default -> { return "unknown reward kind"; }
        }
        return null;
    }

    private static String prettyItem(String id) {
        String n = id.contains(":") ? id.substring(id.indexOf(':') + 1) : id;
        StringBuilder out = new StringBuilder();
        for (String w : n.split("[_/]")) if (!w.isEmpty()) out.append(Character.toUpperCase(w.charAt(0))).append(w.substring(1)).append(' ');
        return out.toString().trim();
    }
}
