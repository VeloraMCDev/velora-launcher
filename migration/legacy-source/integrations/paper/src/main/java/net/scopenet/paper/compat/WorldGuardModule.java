package net.scopenet.paper.compat;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import org.bukkit.Bukkit;
import org.bukkit.World;
import java.lang.reflect.Method;
import java.util.*;

/**
 * Sends WorldGuard's regions (who owns what, where, with which flags) to the panel, so admins
 * can review claims and server-owned territory next to guild claims. WorldGuard is read through
 * reflection, which keeps SCOPENET working across WorldGuard/WorldEdit versions.
 */
public final class WorldGuardModule implements IntegrationModule {
    private static final int MAX_REGIONS = 1500;
    private CompatContext context;
    private int task = -1;
    private volatile int lastCount;

    @Override public String id() { return "worldguard"; }
    @Override public String displayName() { return "WorldGuard"; }
    @Override public String pluginName() { return "WorldGuard"; }

    @Override public void enable(CompatContext context) throws Exception {
        this.context = context;
        collect(); // fail now, not silently later, if this WorldGuard can't be read
        long period = 20L * 60 * Math.max(1, context.getInt("worldguard", "report-interval-minutes", 5));
        // Regions are read on the server thread (their managers aren't promised to be thread-safe), sent off it.
        task = context.everySync(200, period, () -> {
            try {
                JsonObject data = collect();
                context.async(() -> {
                    try { context.report("worldguard", Bukkit.getPluginManager().getPlugin("WorldGuard").getDescription().getVersion(), data); }
                    catch (Exception e) { context.log().fine("WorldGuard report failed: " + e.getMessage()); }
                });
            } catch (Exception e) {
                context.log().fine("Could not read WorldGuard regions: " + e.getMessage());
            }
        });
    }

    @Override public void disable() {
        if (context != null && task >= 0) context.cancel(task);
        task = -1;
    }

    @Override public Map<String, Object> details() {
        return Map.of("regions", lastCount);
    }

    private static int coordinate(Object point, String axis) {
        Object v = Reflect.tryCall(point, "get" + axis);
        if (v == null) v = Reflect.tryCall(point, "getBlock" + axis);
        return (int) Reflect.number(v, 0);
    }

    private static JsonArray triple(Object point) {
        JsonArray a = new JsonArray();
        a.add(coordinate(point, "X"));
        a.add(coordinate(point, "Y"));
        a.add(coordinate(point, "Z"));
        return a;
    }

    /** Owners and members of a region as readable names (players first, then groups as "*group"). */
    private static JsonArray names(Object domain, int limit) {
        JsonArray out = new JsonArray();
        if (domain == null) return out;
        Set<String> seen = new LinkedHashSet<>();
        Object ids = Reflect.tryCall(domain, "getUniqueIds");
        if (ids instanceof Collection<?> uuids) {
            for (Object id : uuids) {
                if (id instanceof UUID uuid) {
                    String name = Bukkit.getOfflinePlayer(uuid).getName();
                    seen.add(name != null ? name : uuid.toString());
                }
            }
        }
        Object legacy = Reflect.tryCall(domain, "getPlayers");
        if (legacy instanceof Collection<?> players) players.forEach(p -> seen.add(String.valueOf(p)));
        Object groups = Reflect.tryCall(domain, "getGroups");
        if (groups instanceof Collection<?> g) g.forEach(x -> seen.add("*" + x));
        seen.stream().limit(limit).forEach(out::add);
        return out;
    }

    private static int size(Object domain) {
        Object n = Reflect.tryCall(domain, "size");
        return (int) Reflect.number(n, 0);
    }

    @SuppressWarnings("unchecked")
    private JsonObject collect() throws Exception {
        Object wg = Reflect.load("com.sk89q.worldguard.WorldGuard", Bukkit.getPluginManager().getPlugin("WorldGuard").getClass().getClassLoader())
                .getMethod("getInstance").invoke(null);
        Object container = Reflect.call(Reflect.call(wg, "getPlatform"), "getRegionContainer");
        Class<?> adapter = Reflect.load("com.sk89q.worldedit.bukkit.BukkitAdapter", Bukkit.getPluginManager().getPlugin("WorldEdit") != null
                ? Bukkit.getPluginManager().getPlugin("WorldEdit").getClass().getClassLoader()
                : Bukkit.getPluginManager().getPlugin("WorldGuard").getClass().getClassLoader());
        Method adapt = adapter.getMethod("adapt", World.class);

        List<JsonObject> regions = new ArrayList<>();
        boolean truncated = false;
        for (World world : Bukkit.getWorlds()) {
            Object manager = Reflect.callCompatible(container, "get", adapt.invoke(null, world));
            if (manager == null) continue;
            Object map = Reflect.call(manager, "getRegions");
            if (!(map instanceof Map<?, ?> all)) continue;
            for (Object region : all.values()) {
                if (regions.size() >= MAX_REGIONS) { truncated = true; break; }
                JsonObject r = new JsonObject();
                r.addProperty("world", world.getName());
                r.addProperty("id", String.valueOf(Reflect.tryCall(region, "getId")));
                Object type = Reflect.tryCall(region, "getType");
                r.addProperty("type", type == null ? "cuboid" : type.toString().toLowerCase(Locale.ROOT));
                r.add("min", triple(Reflect.tryCall(region, "getMinimumPoint")));
                r.add("max", triple(Reflect.tryCall(region, "getMaximumPoint")));
                r.addProperty("priority", (int) Reflect.number(Reflect.tryCall(region, "getPriority"), 0));
                Object parent = Reflect.tryCall(region, "getParent");
                if (parent != null) r.addProperty("parent", String.valueOf(Reflect.tryCall(parent, "getId")));
                Object owners = Reflect.tryCall(region, "getOwners");
                r.add("owners", names(owners, 4));
                r.addProperty("owner_count", size(owners));
                r.addProperty("member_count", size(Reflect.tryCall(region, "getMembers")));
                JsonObject flags = new JsonObject();
                Object flagMap = Reflect.tryCall(region, "getFlags");
                if (flagMap instanceof Map<?, ?> fm) {
                    int shown = 0;
                    for (Map.Entry<?, ?> f : fm.entrySet()) {
                        if (shown++ >= 12) break;
                        String value = String.valueOf(f.getValue());
                        flags.addProperty(String.valueOf(Reflect.tryCall(f.getKey(), "getName")), value.length() > 60 ? value.substring(0, 60) : value);
                    }
                }
                r.add("flags", flags);
                regions.add(r);
            }
        }
        regions.sort((a, b) -> Integer.compare(b.get("priority").getAsInt(), a.get("priority").getAsInt()));
        JsonArray list = new JsonArray();
        regions.forEach(list::add);
        lastCount = regions.size();
        JsonObject data = new JsonObject();
        data.add("regions", list);
        data.addProperty("truncated", truncated);
        return data;
    }
}
