package net.scopenet.paper.compat;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.luckperms.api.LuckPerms;
import net.luckperms.api.LuckPermsProvider;
import net.luckperms.api.cacheddata.CachedMetaData;
import net.luckperms.api.context.DefaultContextKeys;
import net.luckperms.api.model.group.Group;
import net.luckperms.api.model.user.User;
import net.luckperms.api.node.Node;
import net.luckperms.api.node.NodeType;
import net.luckperms.api.node.matcher.NodeMatcher;
import net.luckperms.api.node.types.DisplayNameNode;
import net.luckperms.api.node.types.InheritanceNode;
import net.luckperms.api.node.types.PermissionNode;
import net.luckperms.api.node.types.PrefixNode;
import net.luckperms.api.node.types.SuffixNode;
import net.luckperms.api.node.types.WeightNode;
import net.luckperms.api.query.QueryOptions;
import org.bukkit.Bukkit;
import org.bukkit.entity.Player;
import java.util.*;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionException;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentLinkedQueue;
import java.util.concurrent.TimeUnit;

/**
 * Connects LuckPerms to the SCOPENET panel.
 *
 * <ul>
 *   <li><b>Reports</b> every group (display name, weight, prefix, suffix, parents, permissions, member count) and each online
 *       player's rank, so the panel's LuckPerms manager and player profiles show the real thing.</li>
 *   <li><b>Runs instructions</b> an admin queued in the panel: create, edit and delete groups, set permissions and parents,
 *       move players between groups. They are collected by a quick poll (every few seconds) and the result of each one is
 *       reported back. Switch this off with {@code integrations.luckperms.allow-panel-commands: false}.</li>
 *   <li><b>Keeps mapped groups in step</b> with panel groups and level milestones, when an admin opts in.</li>
 * </ul>
 * LuckPerms stays in charge of permissions: nothing here ever acts without an explicit admin instruction or mapping.
 */
public final class LuckPermsModule implements IntegrationModule {
    private static final int MAX_PERMISSIONS_PER_GROUP = 300;
    private static final int MAX_GROUPS = 200;
    private static final long COMMAND_TIMEOUT_SECONDS = 15;

    private LuckPerms luckPerms;
    private CompatContext context;
    private int reportTask = -1;
    private int pollTask = -1;
    private volatile String lastMode = "off";
    private volatile int lastPlayers;
    private volatile int commandsRun;
    private volatile long lastMemberCount;
    private final Queue<JsonObject> results = new ConcurrentLinkedQueue<>();
    private final Map<String, Integer> memberCounts = new ConcurrentHashMap<>();

    @Override public String id() { return "luckperms"; }
    @Override public String displayName() { return "LuckPerms"; }
    @Override public String pluginName() { return "LuckPerms"; }

    @Override public void enable(CompatContext context) {
        this.context = context;
        this.luckPerms = LuckPermsProvider.get();
        // Chat reads prefixes live, so a rank change shows on the next message instead of after the next report.
        net.scopenet.paper.ChatFormatter.metaSource(this::chatMeta);
        long period = 20L * Math.max(15, context.getInt("luckperms", "report-interval-seconds", 60));
        // Who is online is read on the server thread; LuckPerms and the panel are talked to off it.
        reportTask = context.everySync(100, period, this::reportSoon);
        // Always running: poll() checks the option each time, so /scopenet reload can switch the manager on without a restart.
        long poll = 20L * Math.max(2, context.getInt("luckperms", "command-poll-seconds", 5));
        pollTask = context.everyAsync(140, poll, this::poll);
    }

    @Override public void disable() {
        net.scopenet.paper.ChatFormatter.metaSource(null);
        if (context != null) {
            if (reportTask >= 0) context.cancel(reportTask);
            if (pollTask >= 0) context.cancel(pollTask);
        }
        reportTask = pollTask = -1;
    }

    @Override public Map<String, Object> details() {
        return Map.of("sync_mode", lastMode, "players_reported", lastPlayers, "commands_run", commandsRun,
                "apply_panel_groups", context != null && context.getBoolean("luckperms", "apply-panel-groups", false),
                "allow_panel_commands", context != null && allowCommands());
    }

    private boolean allowCommands() { return context.getBoolean("luckperms", "allow-panel-commands", true); }

    /** Cached LuckPerms data for an online player; cheap and safe to call from the chat thread. */
    private net.scopenet.paper.ChatFormatter.Meta chatMeta(UUID id) {
        User user = luckPerms.getUserManager().getUser(id);
        if (user == null) return null;
        CachedMetaData meta = user.getCachedData().getMetaData();
        Group primary = luckPerms.getGroupManager().getGroup(user.getPrimaryGroup());
        String group = primary == null ? user.getPrimaryGroup() : primary.getDisplayName() != null ? primary.getDisplayName() : primary.getName();
        return new net.scopenet.paper.ChatFormatter.Meta(meta.getPrefix(), meta.getSuffix(), group);
    }

    // ------------------------------------------------------------------------------------------------------------
    // Reporting
    // ------------------------------------------------------------------------------------------------------------

    /** Read who is online on the server thread, then send the full report from a worker. */
    private void reportSoon() {
        List<UUID> online = new ArrayList<>();
        for (Player p : Bukkit.getOnlinePlayers()) online.add(p.getUniqueId());
        context.async(() -> report(online));
    }

    private synchronized void report(List<UUID> online) {
        try {
            JsonObject data = new JsonObject();
            data.addProperty("can_manage", allowCommands());
            data.add("groups", groups());
            JsonArray players = new JsonArray();
            for (UUID id : online) {
                JsonObject p = player(id);
                if (p != null) players.add(p);
            }
            data.add("players", players);
            lastPlayers = players.size();
            data.add("command_results", drainResults());

            JsonObject answer = context.report("luckperms", Bukkit.getPluginManager().getPlugin("LuckPerms").getDescription().getVersion(), data);
            JsonObject config = answer.has("config") && answer.get("config").isJsonObject() ? answer.getAsJsonObject("config") : new JsonObject();
            if (config.has("mode")) lastMode = config.get("mode").getAsString();
            if (context.getBoolean("luckperms", "apply-panel-groups", false) && config.has("assign")) apply(config.getAsJsonArray("assign"));
            if (config.has("level_assign")) apply(config.getAsJsonArray("level_assign"));
            if (allowCommands() && config.has("commands")) runCommands(config.getAsJsonArray("commands"));
            refreshMemberCounts();
        } catch (Exception e) {
            context.log().fine("LuckPerms report failed: " + e.getMessage());
        }
    }

    /** The quick poll: hand in finished results, pick up new instructions. Cheap enough to run every few seconds. */
    private synchronized void poll() {
        try {
            if (!allowCommands()) return;
            JsonObject data = new JsonObject();
            data.addProperty("light", true);
            data.add("command_results", drainResults());
            JsonObject answer = context.report("luckperms", "", data);
            JsonObject config = answer.has("config") && answer.get("config").isJsonObject() ? answer.getAsJsonObject("config") : new JsonObject();
            if (config.has("commands") && config.getAsJsonArray("commands").size() > 0) {
                runCommands(config.getAsJsonArray("commands"));
                // Show the result in the panel straight away instead of at the next full report.
                context.sync(this::reportSoon);
            }
        } catch (Exception e) {
            context.log().fine("LuckPerms poll failed: " + e.getMessage());
        }
    }

    private JsonArray drainResults() {
        JsonArray out = new JsonArray();
        JsonObject r;
        while ((r = results.poll()) != null && out.size() < 200) out.add(r);
        return out;
    }

    private JsonArray groups() {
        JsonArray groups = new JsonArray();
        List<Group> loaded = new ArrayList<>(luckPerms.getGroupManager().getLoadedGroups());
        loaded.sort(Comparator.comparingInt((Group g) -> -g.getWeight().orElse(0)).thenComparing(Group::getName));
        for (Group g : loaded) {
            if (groups.size() >= MAX_GROUPS) break;
            JsonObject o = new JsonObject();
            o.addProperty("name", g.getName());
            if (g.getDisplayName() != null) o.addProperty("display", g.getDisplayName());
            o.addProperty("weight", g.getWeight().orElse(0));
            // The group's own prefix and suffix (the highest priority one), not what it inherits.
            o.addProperty("prefix", g.getNodes(NodeType.PREFIX).stream().max(Comparator.comparingInt(PrefixNode::getPriority)).map(PrefixNode::getMetaValue).orElse(""));
            o.addProperty("suffix", g.getNodes(NodeType.SUFFIX).stream().max(Comparator.comparingInt(SuffixNode::getPriority)).map(SuffixNode::getMetaValue).orElse(""));
            JsonArray parents = new JsonArray();
            for (InheritanceNode n : g.getNodes(NodeType.INHERITANCE)) parents.add(n.getGroupName());
            o.add("parents", parents);
            JsonArray perms = new JsonArray();
            for (PermissionNode n : g.getNodes(NodeType.PERMISSION)) {
                if (perms.size() >= MAX_PERMISSIONS_PER_GROUP) break;
                JsonObject node = new JsonObject();
                node.addProperty("key", n.getKey());
                node.addProperty("value", n.getValue());
                n.getContexts().getAnyValue(DefaultContextKeys.SERVER_KEY).ifPresent(v -> node.addProperty("server", v));
                n.getContexts().getAnyValue(DefaultContextKeys.WORLD_KEY).ifPresent(v -> node.addProperty("world", v));
                if (n.hasExpiry()) node.addProperty("temporary", true);
                perms.add(node);
            }
            o.add("permissions", perms);
            Integer members = memberCounts.get(g.getName());
            if (members != null) o.addProperty("members", members);
            groups.add(o);
        }
        return groups;
    }

    private JsonObject player(UUID id) {
        User user = luckPerms.getUserManager().getUser(id);
        if (user == null) return null;
        JsonObject p = new JsonObject();
        p.addProperty("uuid", id.toString());
        String primary = user.getPrimaryGroup();
        p.addProperty("primary", primary);
        Group primaryGroup = luckPerms.getGroupManager().getGroup(primary);
        if (primaryGroup != null) {
            p.addProperty("display", primaryGroup.getDisplayName() != null ? primaryGroup.getDisplayName() : primaryGroup.getName());
            p.addProperty("weight", primaryGroup.getWeight().orElse(0));
        }
        CachedMetaData meta = user.getCachedData().getMetaData();
        if (meta.getPrefix() != null) p.addProperty("prefix", meta.getPrefix());
        if (meta.getSuffix() != null) p.addProperty("suffix", meta.getSuffix());
        JsonArray memberOf = new JsonArray();
        for (Group g : user.getInheritedGroups(QueryOptions.nonContextual())) memberOf.add(g.getName());
        p.add("groups", memberOf);
        // Only permissions set directly on the player; group permissions are the group's business.
        JsonArray nodes = new JsonArray();
        int shown = 0;
        for (var node : user.getNodes(NodeType.PERMISSION)) {
            if (shown++ >= 40) break;
            nodes.add((node.getValue() ? "" : "-") + node.getKey());
        }
        p.add("permissions", nodes);
        return p;
    }

    /** Count members per group in the background, at most every five minutes: a search over every user is not free. */
    private void refreshMemberCounts() {
        long now = System.currentTimeMillis();
        if (now - lastMemberCount < 300_000) return;
        lastMemberCount = now;
        int done = 0;
        for (Group g : luckPerms.getGroupManager().getLoadedGroups()) {
            if (done++ >= 60) break;
            try {
                var found = luckPerms.getUserManager().searchAll(NodeMatcher.key(InheritanceNode.builder(g.getName()).build())).get(COMMAND_TIMEOUT_SECONDS, TimeUnit.SECONDS);
                memberCounts.put(g.getName(), found.size());
            } catch (Exception e) {
                return; // storage is busy or slow; try again at the next window
            }
        }
    }

    // ------------------------------------------------------------------------------------------------------------
    // Mapped groups (panel groups, level milestones)
    // ------------------------------------------------------------------------------------------------------------

    /** Add or remove membership of mapped groups. Nothing else about a player's permissions is touched. */
    private void apply(JsonArray assignments) {
        for (JsonElement element : assignments) {
            JsonObject a = element.getAsJsonObject();
            UUID uuid = UUID.fromString(a.get("uuid").getAsString());
            List<String> add = strings(a, "add"), remove = strings(a, "remove");
            luckPerms.getUserManager().modifyUser(uuid, user -> {
                for (String group : add) {
                    if (luckPerms.getGroupManager().getGroup(group) != null) user.data().add(InheritanceNode.builder(group).build());
                }
                for (String group : remove) user.data().remove(InheritanceNode.builder(group).build());
            });
        }
    }

    // ------------------------------------------------------------------------------------------------------------
    // Instructions from the panel
    // ------------------------------------------------------------------------------------------------------------

    private void runCommands(JsonArray commands) {
        for (JsonElement element : commands) {
            JsonObject command = element.getAsJsonObject();
            long id = command.get("id").getAsLong();
            JsonObject result = new JsonObject();
            result.addProperty("id", id);
            try {
                execute(command.getAsJsonObject("op")).get(COMMAND_TIMEOUT_SECONDS, TimeUnit.SECONDS);
                result.addProperty("ok", true);
                commandsRun++;
            } catch (Exception e) {
                result.addProperty("ok", false);
                result.addProperty("error", describe(e));
                context.log().warning("LuckPerms instruction " + id + " failed: " + describe(e));
            }
            results.add(result);
        }
    }

    private static String describe(Throwable e) {
        while ((e instanceof CompletionException || e instanceof java.util.concurrent.ExecutionException) && e.getCause() != null) e = e.getCause();
        String message = e.getMessage();
        return message == null || message.isBlank() ? e.getClass().getSimpleName() : message;
    }

    private CompletableFuture<Void> execute(JsonObject op) {
        String kind = str(op, "kind");
        switch (kind) {
            case "batch": {
                CompletableFuture<Void> chain = CompletableFuture.completedFuture(null);
                for (JsonElement e : op.getAsJsonArray("ops")) {
                    JsonObject next = e.getAsJsonObject();
                    chain = chain.thenCompose(v -> execute(next));
                }
                return chain;
            }
            case "group_create":
                return luckPerms.getGroupManager().createAndLoadGroup(str(op, "group")).thenApply(g -> null);
            case "group_delete":
                return luckPerms.getGroupManager().loadGroup(str(op, "group")).thenCompose(found -> {
                    if (found.isEmpty()) throw new IllegalStateException("Group " + str(op, "group") + " does not exist");
                    return luckPerms.getGroupManager().deleteGroup(found.get());
                });
            case "group_update":
                return luckPerms.getGroupManager().modifyGroup(str(op, "group"), g -> updateGroup(g, op));
            case "perm_set":
                return luckPerms.getGroupManager().modifyGroup(str(op, "group"), g -> setPermission(g, op));
            case "perm_unset":
                return luckPerms.getGroupManager().modifyGroup(str(op, "group"), g -> clearPermission(g, op));
            case "parent_add": {
                String parent = str(op, "parent");
                return luckPerms.getGroupManager().loadGroup(parent).thenCompose(found -> {
                    if (found.isEmpty()) throw new IllegalStateException("Group " + parent + " does not exist");
                    return luckPerms.getGroupManager().modifyGroup(str(op, "group"), g -> g.data().add(InheritanceNode.builder(parent).build()));
                });
            }
            case "parent_remove":
                return luckPerms.getGroupManager().modifyGroup(str(op, "group"), g -> g.data().remove(InheritanceNode.builder(str(op, "parent")).build()));
            case "user_groups":
                return userGroups(op);
            default:
                return CompletableFuture.failedFuture(new IllegalArgumentException("Unknown instruction: " + kind));
        }
    }

    private void updateGroup(Group g, JsonObject op) {
        if (op.has("weight")) {
            g.data().clear(n -> NodeType.WEIGHT.matches(n));
            g.data().add(WeightNode.builder(op.get("weight").getAsInt()).build());
        }
        if (op.has("display")) {
            g.data().clear(n -> NodeType.DISPLAY_NAME.matches(n));
            String display = str(op, "display");
            if (!display.isEmpty()) g.data().add(DisplayNameNode.builder(display).build());
        }
        int priority = op.has("weight") ? op.get("weight").getAsInt() : g.getWeight().orElse(0);
        if (op.has("prefix")) {
            g.data().clear(n -> NodeType.PREFIX.matches(n));
            String prefix = str(op, "prefix");
            if (!prefix.isEmpty()) g.data().add(PrefixNode.builder(prefix, priority).build());
        }
        if (op.has("suffix")) {
            g.data().clear(n -> NodeType.SUFFIX.matches(n));
            String suffix = str(op, "suffix");
            if (!suffix.isEmpty()) g.data().add(SuffixNode.builder(suffix, priority).build());
        }
    }

    private static boolean sameNode(Node n, String key, String server, String world) {
        return NodeType.PERMISSION.matches(n) && n.getKey().equalsIgnoreCase(key)
                && n.getContexts().getAnyValue(DefaultContextKeys.SERVER_KEY).orElse("").equalsIgnoreCase(server)
                && n.getContexts().getAnyValue(DefaultContextKeys.WORLD_KEY).orElse("").equalsIgnoreCase(world);
    }

    private void setPermission(Group g, JsonObject op) {
        String key = str(op, "permission"), server = str(op, "server"), world = str(op, "world");
        g.data().clear(n -> sameNode(n, key, server, world));
        PermissionNode.Builder builder = PermissionNode.builder(key).value(!op.has("value") || op.get("value").getAsBoolean());
        if (!server.isEmpty()) builder.withContext(DefaultContextKeys.SERVER_KEY, server);
        if (!world.isEmpty()) builder.withContext(DefaultContextKeys.WORLD_KEY, world);
        g.data().add(builder.build());
    }

    private void clearPermission(Group g, JsonObject op) {
        String key = str(op, "permission"), server = str(op, "server"), world = str(op, "world");
        g.data().clear(n -> sameNode(n, key, server, world));
    }

    private CompletableFuture<Void> userGroups(JsonObject op) {
        UUID uuid = UUID.fromString(str(op, "uuid"));
        List<String> add = strings(op, "add"), remove = strings(op, "remove");
        for (String group : add) {
            if (luckPerms.getGroupManager().getGroup(group) == null) {
                return CompletableFuture.failedFuture(new IllegalStateException("Group " + group + " does not exist"));
            }
        }
        // Works for players who are offline too: LuckPerms loads them, changes them and saves.
        return luckPerms.getUserManager().modifyUser(uuid, user -> {
            for (String group : add) user.data().add(InheritanceNode.builder(group).build());
            for (String group : remove) user.data().remove(InheritanceNode.builder(group).build());
        });
    }

    private static String str(JsonObject o, String key) {
        return o.has(key) && !o.get(key).isJsonNull() ? o.get(key).getAsString() : "";
    }

    private static List<String> strings(JsonObject o, String key) {
        List<String> out = new ArrayList<>();
        if (o.has(key) && o.get(key).isJsonArray()) o.getAsJsonArray(key).forEach(e -> out.add(e.getAsString()));
        return out;
    }
}
