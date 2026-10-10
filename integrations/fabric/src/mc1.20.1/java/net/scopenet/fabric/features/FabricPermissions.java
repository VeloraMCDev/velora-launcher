package net.scopenet.fabric.features;

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
import java.util.*;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionException;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentLinkedQueue;
import java.util.concurrent.TimeUnit;

import net.scopenet.core.Env;
/** Fabric adapter for the panel manager. Operations adapt the repository's Paper LuckPermsModule. */
final class FabricPermissions {
    private static final int MAX_GROUPS=200, MAX_PERMISSIONS_PER_GROUP=300;
    private final LuckPerms luckPerms=LuckPermsProvider.get();
    private final Env env;
    private final java.util.function.BooleanSupplier applyMappings;
    private final Map<String,Integer> memberCounts=Map.of();
    private final Map<Long,JsonObject> results=new LinkedHashMap<>();
    private boolean running;
    private long nextPoll, nextReport;
    FabricPermissions(Env env,java.util.function.BooleanSupplier applyMappings) {this.env=env;this.applyMappings=applyMappings;}
    void tick() {
        long now=env.clock.getAsLong();
        if(running||now<nextPoll||!env.modules.get().enabled("permissions_chat"))return;
        running=true;nextPoll=now+5000;
        List<UUID> online=env.platform.online().stream().map(p->p.uuid()).toList();
        boolean full=now>=nextReport;
        env.io(()->{poll(online,full);return true;},ok->{running=false;if(full)nextReport=env.clock.getAsLong()+60000;},error->{running=false;env.log.fine("Permissions sync: "+error);});
    }
    private void poll(List<UUID> online,boolean full) throws Exception {
        JsonObject data=new JsonObject();data.addProperty("can_manage",true);
        JsonArray acknowledgments=new JsonArray();results.values().forEach(acknowledgments::add);data.add("command_results",acknowledgments);
        if(full) {data.add("groups",groups());JsonArray players=new JsonArray();for(UUID id:online){JsonObject p=player(id);if(p!=null)players.add(p);}data.add("players",players);}
        else data.addProperty("light",true);
        if(data.toString().getBytes(java.nio.charset.StandardCharsets.UTF_8).length>900_000){
            data.remove("groups");data.remove("players");data.addProperty("light",true);
            env.log.warning("LuckPerms snapshot exceeds the panel report budget; commands still synchronize. Reduce group/report size before relying on the snapshot.");
        }
        JsonObject body=new JsonObject();body.addProperty("name","luckperms");body.addProperty("version","Fabric API adapter");body.add("data",data);
        JsonObject answer=env.panel.call("integrations/report",body).getAsJsonObject();
        results.clear();
        JsonObject cfg=answer.has("config")?answer.getAsJsonObject("config"):new JsonObject();
        if(applyMappings.getAsBoolean())for(String key:List.of("assign","level_assign"))if(cfg.has(key))for(JsonElement a:cfg.getAsJsonArray(key))userGroups(a.getAsJsonObject()).get(15,TimeUnit.SECONDS);
        if(cfg.has("commands"))for(JsonElement e:cfg.getAsJsonArray("commands")) {
            JsonObject command=e.getAsJsonObject(),result=new JsonObject();long id=command.get("id").getAsLong();result.addProperty("id",id);
            try {execute(command.getAsJsonObject("op")).get(15,TimeUnit.SECONDS);result.addProperty("ok",true);}
            catch(Exception ex){result.addProperty("ok",false);result.addProperty("error",ex.getCause()==null?ex.toString():ex.getCause().toString());}
            results.put(id,result);nextReport=0;
        }
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
                    if (found.isEmpty()) return CompletableFuture.completedFuture(null);
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
