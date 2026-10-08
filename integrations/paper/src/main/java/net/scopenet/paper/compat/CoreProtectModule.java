package net.scopenet.paper.compat;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import org.bukkit.Bukkit;
import org.bukkit.event.Event;
import org.bukkit.event.EventPriority;
import org.bukkit.event.Listener;
import org.bukkit.plugin.Plugin;
import java.io.File;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.LongAdder;

/**
 * Reports what CoreProtect is recording (actions per minute, the busiest players) and the
 * state of the server's world backups to the panel, so admins can see what's happening
 * in-game and whether backups are current without opening the console.
 * CoreProtect is used through reflection, so a CoreProtect update can't break Velora.
 */
public final class CoreProtectModule implements IntegrationModule {
    private final Map<String, LongAdder> byActor = new ConcurrentHashMap<>();
    private final LongAdder total = new LongAdder();
    private final long[] minuteKeys = new long[60];
    private final long[] minuteCounts = new long[60];
    private final long startedAt = System.currentTimeMillis();
    private CompatContext context;
    private Plugin coreProtect;
    private Listener listener;
    private int task = -1;
    private volatile boolean tracking;

    @Override public String id() { return "coreprotect"; }
    @Override public String displayName() { return "CoreProtect"; }
    @Override public String pluginName() { return "CoreProtect"; }

    @Override @SuppressWarnings("unchecked")
    public void enable(CompatContext context) {
        this.context = context;
        this.coreProtect = Bukkit.getPluginManager().getPlugin("CoreProtect");
        try {
            // CoreProtect 21.3+ fires an event for every action it logs.
            Class<? extends Event> type = (Class<? extends Event>) Reflect.load("net.coreprotect.event.CoreProtectPreLogEvent", coreProtect.getClass().getClassLoader());
            listener = new Listener() {};
            Bukkit.getPluginManager().registerEvent(type, listener, EventPriority.MONITOR, (l, event) -> record(event), context.plugin(), true);
            tracking = true;
        } catch (ClassNotFoundException | LinkageError e) {
            context.log().info("This CoreProtect version has no logging event; reporting its status and your backups only.");
        }
        task = context.everyAsync(200, 20L * 60, this::report);
    }

    @Override public void disable() {
        if (context != null && task >= 0) context.cancel(task);
        task = -1;
        if (listener != null) org.bukkit.event.HandlerList.unregisterAll(listener);
        listener = null;
    }

    @Override public Map<String, Object> details() {
        return Map.of("logged_this_session", total.sum(), "tracking_actions", tracking);
    }

    private void record(Event event) {
        Object user = Reflect.tryCall(event, "getUser");
        byActor.computeIfAbsent(user == null ? "?" : String.valueOf(user), k -> new LongAdder()).increment();
        total.increment();
        long minute = System.currentTimeMillis() / 60_000;
        int slot = (int) (minute % 60);
        synchronized (minuteKeys) {
            if (minuteKeys[slot] != minute) {
                minuteKeys[slot] = minute;
                minuteCounts[slot] = 0;
            }
            minuteCounts[slot]++;
        }
    }

    private void report() {
        try {
            JsonObject data = new JsonObject();
            Object api = Reflect.tryCall(coreProtect, "getAPI");
            Object apiVersion = Reflect.tryCall(api, "APIVersion");
            if (apiVersion != null) data.addProperty("api_version", String.valueOf(apiVersion));
            data.addProperty("tracking_actions", tracking);
            data.addProperty("logged_total", total.sum());
            data.addProperty("since", startedAt);

            long now = System.currentTimeMillis() / 60_000;
            JsonArray perMinute = new JsonArray();
            long lastHour = 0;
            synchronized (minuteKeys) {
                for (long m = now - 59; m <= now; m++) {
                    int slot = (int) (m % 60);
                    long count = minuteKeys[slot] == m ? minuteCounts[slot] : 0;
                    perMinute.add(count);
                    lastHour += count;
                }
            }
            data.add("per_minute", perMinute);
            data.addProperty("logged_last_hour", lastHour);

            JsonArray top = new JsonArray();
            byActor.entrySet().stream().sorted((a, b) -> Long.compare(b.getValue().sum(), a.getValue().sum())).limit(8).forEach(e -> {
                JsonObject o = new JsonObject();
                o.addProperty("name", e.getKey());
                o.addProperty("count", e.getValue().sum());
                top.add(o);
            });
            data.add("top_actors", top);
            data.add("backups", backups());
            context.report("coreprotect", coreProtect.getDescription().getVersion(), data);
        } catch (Exception e) {
            context.log().fine("CoreProtect report failed: " + e.getMessage());
        }
    }

    /** Looks for backup archives in {@code integrations.coreprotect.backup-folders} (default: ./backups). */
    private JsonObject backups() {
        JsonObject out = new JsonObject();
        List<String> folders = context.config().getStringList("integrations.coreprotect.backup-folders");
        if (folders.isEmpty()) folders = List.of("backups");
        long newest = 0, size = 0;
        int count = 0;
        String newestName = null;
        for (String folder : folders) {
            File[] files = new File(folder).listFiles(f -> f.isFile() || f.isDirectory());
            if (files == null) continue;
            for (File f : files) {
                if (count >= 2000) break;
                count++;
                size += f.isFile() ? f.length() : 0;
                if (f.lastModified() > newest) {
                    newest = f.lastModified();
                    newestName = f.getName();
                }
            }
        }
        out.addProperty("found", count > 0);
        out.addProperty("count", count);
        out.addProperty("bytes", size);
        if (newestName != null) {
            out.addProperty("latest_name", newestName);
            out.addProperty("latest_at", newest);
        }
        return out;
    }
}
