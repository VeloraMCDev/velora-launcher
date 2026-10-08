package net.scopenet.core;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

import java.util.Arrays;
import java.util.List;
import java.util.Locale;
import java.util.Set;

/**
 * The management side of {@code /guild}: browse guilds, ask to join, review requests, kick, promote, change roles, hand over
 * leadership, set the message of the day and description, and post announcements. Every rule lives in the panel, so what a
 * player can do here is exactly what the launcher allows, and each change appears there at once.
 */
public final class GuildManage {
    public static final Set<String> SUBCOMMANDS = Set.of("list", "join", "requests", "approve", "reject", "kick", "promote", "demote",
            "role", "roles", "transfer", "motd", "desc", "post", "posts", "flags");
    public static final List<String> COMPLETIONS = List.of("list", "join", "requests", "approve", "reject", "kick", "promote", "demote",
            "role", "roles", "transfer", "motd", "desc", "post", "posts", "flags");

    private final Env env;

    public GuildManage(Env env) { this.env = env; }

    /** True if {@code sub} is one of the management sub-commands (so the caller should hand it over). */
    public static boolean handles(String sub, int argCount) {
        String s = sub.toLowerCase(Locale.ROOT);
        return SUBCOMMANDS.contains(s) || (s.equals("info") && argCount > 0);
    }

    /** Runs a management sub-command; {@code rest} are the arguments after it. */
    public void run(CorePlayer p, String sub, String[] rest) {
        String s = sub.toLowerCase(Locale.ROOT);
        switch (s) {
            case "list", "roles", "requests", "posts" -> call(p, s, "", "", "");
            case "info" -> call(p, "info", rest.length > 0 ? String.join(" ", rest) : "", "", "");
            case "join" -> {
                if (rest.length < 1) { p.send(Format.RED + "Usage: /guild join <guild name or tag> [message]"); return; }
                call(p, "join", rest[0], rest.length > 1 ? String.join(" ", Arrays.copyOfRange(rest, 1, rest.length)) : "", "");
            }
            case "approve", "reject" -> {
                if (rest.length < 1) { p.send(Format.RED + "Usage: /guild " + s + " <player>"); return; }
                call(p, s.equals("approve") ? "accept" : "deny", rest[0], "", "");
            }
            case "kick", "promote", "demote" -> {
                if (rest.length < 1) { p.send(Format.RED + "Usage: /guild " + s + " <player>"); return; }
                call(p, s, rest[0], "", "");
            }
            case "role" -> {
                if (rest.length < 2) { p.send(Format.RED + "Usage: /guild role <player> <role>  (see /guild roles)"); return; }
                call(p, "role", rest[0], String.join(" ", Arrays.copyOfRange(rest, 1, rest.length)), "");
            }
            case "transfer" -> {
                if (rest.length < 1) { p.send(Format.RED + "Usage: /guild transfer <player> confirm"); return; }
                if (rest.length < 2 || !rest[1].equalsIgnoreCase("confirm")) {
                    p.send(Format.YELLOW + "This makes " + rest[0] + " the guild leader and you an officer. Run " + Format.WHITE + "/guild transfer " + rest[0] + " confirm" + Format.YELLOW + " to do it.");
                    return;
                }
                call(p, "transfer", rest[0], "", "");
            }
            case "motd", "desc" -> call(p, s, "", String.join(" ", rest), "");
            case "flags" -> {
                if (!p.hasPermission("scopenet.command.guild.flags")) { p.send(Format.RED + "You do not have permission to use /guild flags."); return; }
                if (rest.length == 0) call(p, "flags", "", "", "");
                else if (rest.length == 2) call(p, "flag", rest[0], rest[1], "");
                else p.send(Format.RED + "Usage: /guild flags  or  /guild flags <rule> <on|off>");
            }
            case "post" -> {
                String joined = String.join(" ", rest);
                int bar = joined.indexOf('|');
                if (bar <= 0 || bar == joined.length() - 1) { p.send(Format.RED + "Usage: /guild post <title> | <message>"); return; }
                call(p, "post", "", joined.substring(bar + 1).trim(), joined.substring(0, bar).trim());
            }
            default -> p.send(Format.RED + "Unknown guild command.");
        }
    }

    private void call(CorePlayer p, String action, String target, String text, String title) {
        JsonObject body = new JsonObject();
        body.addProperty("uuid", p.uuid().toString());
        body.addProperty("name", p.name());
        body.addProperty("action", action);
        body.addProperty("target", target);
        body.addProperty("text", text);
        body.addProperty("title", title);
        env.io(() -> env.panel.call("guilds/manage", body), r -> show(p, action, r.isJsonObject() ? r.getAsJsonObject() : new JsonObject()),
                e -> p.send(Format.RED + e));
    }

    /** The guild's land rules, grouped, with what each is set to and which ones the server admins have locked. */
    private void showFlags(CorePlayer p, JsonObject r) {
        p.send(Format.GOLD + "=== Your guild's land rules ===");
        String group = "";
        for (JsonElement e : r.has("rules") ? r.getAsJsonArray("rules") : new JsonArray()) {
            JsonObject q = e.getAsJsonObject();
            if (!str(q, "group").equals(group)) { group = str(q, "group"); p.send(Format.AQUA + group); }
            boolean on = q.has("value") && q.get("value").getAsBoolean();
            boolean editable = !q.has("editable") || q.get("editable").getAsBoolean();
            p.send(Format.GRAY + " " + (on ? Format.GREEN + "ON  " : Format.RED + "OFF ") + Format.YELLOW + str(q, "id") + Format.GRAY + " - " + str(q, "label") + (editable ? "" : Format.DARK_GRAY + " (locked by the server)"));
        }
        p.send(Format.GRAY + (r.has("can_edit") && r.get("can_edit").getAsBoolean()
                ? "Change one with " + Format.YELLOW + "/guild flags <rule> <on|off>" + Format.GRAY + ". Members are never held back by the visitor rules."
                : "Only the leader and officers can change these."));
    }

    private static String str(JsonObject o, String k) { return o.has(k) && o.get(k).isJsonPrimitive() ? o.get(k).getAsString() : ""; }
    private static long num(JsonObject o, String k) { return o.has(k) && o.get(k).isJsonPrimitive() ? o.get(k).getAsLong() : 0; }

    private void show(CorePlayer p, String action, JsonObject r) {
        switch (action) {
            case "list" -> {
                p.send(Format.GOLD + "=== Guilds ===");
                JsonArray guilds = r.has("guilds") ? r.getAsJsonArray("guilds") : new JsonArray();
                if (guilds.size() == 0) p.send(Format.GRAY + "No guilds yet. Start one with /guild create <name> <tag>.");
                for (JsonElement e : guilds) {
                    JsonObject g = e.getAsJsonObject();
                    p.send(Format.AQUA + "[" + str(g, "tag") + "] " + Format.YELLOW + str(g, "name") + Format.GRAY + " - " + num(g, "members") + " members, " + num(g, "claims") + " chunks, level " + num(g, "level"));
                }
                p.send(Format.GRAY + "Details: /guild info <name>  ·  Ask to join: /guild join <name>");
            }
            case "info" -> {
                p.send(Format.GOLD + "=== " + Format.AQUA + "[" + str(r, "tag") + "] " + Format.YELLOW + str(r, "name") + Format.GOLD + " ===");
                if (!str(r, "description").isBlank()) p.send(Format.GRAY + str(r, "description"));
                p.send(Format.GRAY + "Leader: " + Format.WHITE + str(r, "leader") + Format.GRAY + "  Members: " + Format.WHITE + num(r, "members")
                        + Format.GRAY + "  Land: " + Format.WHITE + num(r, "claims") + " chunks" + Format.GRAY + "  Level: " + Format.WHITE + num(r, "level"));
                if (!str(r, "motd").isBlank()) p.send(Format.GRAY + "Message of the day: " + Format.WHITE + str(r, "motd"));
                if (!str(r, "your_role").isBlank()) p.send(Format.GRAY + "Your role: " + Format.WHITE + str(r, "your_role"));
            }
            case "requests" -> {
                JsonArray list = r.has("requests") ? r.getAsJsonArray("requests") : new JsonArray();
                if (list.size() == 0) { p.send(Format.GRAY + "No one is waiting to join."); return; }
                p.send(Format.GOLD + "=== Join requests ===");
                for (JsonElement e : list) {
                    JsonObject q = e.getAsJsonObject();
                    p.send(Format.YELLOW + str(q, "name") + (str(q, "message").isBlank() ? "" : Format.GRAY + " - \"" + str(q, "message") + "\""));
                }
                p.send(Format.GRAY + "Answer with " + Format.YELLOW + "/guild approve <player>" + Format.GRAY + " or " + Format.YELLOW + "/guild reject <player>");
            }
            case "roles" -> {
                p.send(Format.GOLD + "=== Guild roles ===");
                p.send(Format.YELLOW + "leader" + Format.GRAY + " - everything" );
                p.send(Format.YELLOW + "officer" + Format.GRAY + " - invite, kick, claim, post, manage");
                p.send(Format.YELLOW + "member" + Format.GRAY + " - claim, post");
                JsonArray roles = r.has("roles") ? r.getAsJsonArray("roles") : new JsonArray();
                for (JsonElement e : roles) {
                    JsonObject q = e.getAsJsonObject();
                    StringBuilder perms = new StringBuilder();
                    for (String[] f : new String[][] {{"can_invite", "invite"}, {"can_kick", "kick"}, {"can_claim", "claim"}, {"can_post", "post"}, {"can_manage", "manage"}}) {
                        if (q.has(f[0]) && q.get(f[0]).getAsBoolean()) perms.append(perms.length() > 0 ? ", " : "").append(f[1]);
                    }
                    p.send(Format.YELLOW + str(q, "name") + Format.GRAY + " - " + (perms.length() == 0 ? "no extra powers" : perms));
                }
            }
            case "flags" -> showFlags(p, r);
            case "posts" -> {
                JsonArray posts = r.has("posts") ? r.getAsJsonArray("posts") : new JsonArray();
                if (posts.size() == 0) { p.send(Format.GRAY + "Your guild's board is empty. Post with /guild post <title> | <message>."); return; }
                p.send(Format.GOLD + "=== Guild board ===");
                for (JsonElement e : posts) {
                    JsonObject q = e.getAsJsonObject();
                    String content = str(q, "content");
                    p.send(Format.YELLOW + str(q, "title") + Format.DARK_GRAY + " by " + str(q, "author"));
                    p.send(Format.GRAY + (content.length() > 140 ? content.substring(0, 139) + "…" : content));
                }
            }
            default -> {
                String message = str(r, "message");
                p.send(Format.GREEN + (message.isBlank() ? "Done." : message));
                if (action.equals("accept") || action.equals("kick") || action.equals("transfer") || action.equals("join")) env.panel.claimsChanged();
            }
        }
    }
}
