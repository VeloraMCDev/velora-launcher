package net.velora.paper.analytics;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import net.kyori.adventure.text.Component;
import net.velora.paper.config.VeloraConfig;
import net.velora.paper.panel.PanelClient;
import net.velora.paper.panel.SyncResult;
import org.bukkit.Bukkit;
import org.bukkit.World;
import org.bukkit.entity.Player;
import org.bukkit.event.EventHandler;
import org.bukkit.event.EventPriority;
import org.bukkit.event.Listener;
import org.bukkit.event.block.BlockBreakEvent;
import org.bukkit.event.block.BlockPlaceEvent;
import org.bukkit.event.entity.EntityDeathEvent;
import org.bukkit.event.entity.PlayerDeathEvent;
import org.bukkit.event.player.PlayerAdvancementDoneEvent;
import org.bukkit.event.player.PlayerCommandPreprocessEvent;
import org.bukkit.event.player.PlayerJoinEvent;
import org.bukkit.event.player.PlayerQuitEvent;
import org.bukkit.plugin.Plugin;
import org.bukkit.scheduler.BukkitTask;

import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;
import java.util.logging.Logger;

public final class AnalyticsTracker implements Listener {
    private final Plugin plugin;
    private final VeloraConfig config;
    private final PanelClient panelClient;
    private final Logger logger;

    // Online sessions: UUID -> SessionInfo
    record Session(String name, long joinTimeNanos) {}
    final Map<UUID, Session> sessions = new ConcurrentHashMap<>();

    // Aggregated stats: UUID -> { statKey -> count }
    record StatHolder(String name, ConcurrentHashMap<String, AtomicLong> counters) {}
    final Map<UUID, StatHolder> stats = new ConcurrentHashMap<>();

    // Event queue
    final List<JsonObject> events = Collections.synchronizedList(new ArrayList<>());

    // Action deltas: key (uuid + ":" + action) -> ActionHolder
    record ActionHolder(UUID uuid, String name, String action, AtomicLong count, long at) {}
    final Map<String, ActionHolder> actions = new ConcurrentHashMap<>();

    private final AtomicBoolean isSyncing = new AtomicBoolean(false);
    private BukkitTask syncTask;
    private BukkitTask tpsTask;

    // Rolling TPS tracker
    private long lastTickNanos = System.nanoTime();
    private int tickCount = 0;
    private double currentTps = 20.0;
    private long sampleStartNanos = System.nanoTime();

    public AnalyticsTracker(Plugin plugin, VeloraConfig config, PanelClient panelClient, Logger logger) {
        this.plugin = plugin;
        this.config = config;
        this.panelClient = panelClient;
        this.logger = logger;
    }

    public void start() {
        // Measure rolling TPS every tick
        tpsTask = Bukkit.getScheduler().runTaskTimer(plugin, () -> {
            tickCount++;
            long now = System.nanoTime();
            long elapsed = now - sampleStartNanos;
            if (elapsed >= 1_000_000_000L) { // every 1 second
                currentTps = Math.min(20.0, Math.max(0.0, (tickCount * 1_000_000_000.0) / elapsed));
                tickCount = 0;
                sampleStartNanos = now;
            }
        }, 1L, 1L);

        // Schedule periodic sync with panel
        long intervalTicks = config.getSyncIntervalSeconds() * 20L;
        syncTask = Bukkit.getScheduler().runTaskTimerAsynchronously(plugin, this::sync, intervalTicks, intervalTicks);
    }

    public void stop() {
        if (tpsTask != null) tpsTask.cancel();
        if (syncTask != null) syncTask.cancel();

        // Flush remaining playtime for online players
        long now = System.nanoTime();
        for (Player p : Bukkit.getOnlinePlayers()) {
            Session s = sessions.remove(p.getUniqueId());
            if (s != null) {
                long seconds = Math.max(0L, (now - s.joinTimeNanos()) / 1_000_000_000L);
                addStat(p.getUniqueId(), s.name(), "playtime_secs", seconds);
                addEvent(p.getUniqueId(), s.name(), "leave", null);
            }
        }

        // Final sync
        try {
            JsonObject payload = buildPayload(currentTps);
            panelClient.sync(payload);
        } catch (Exception ignored) {}
    }

    public double getCurrentTps() {
        return currentTps;
    }

    public void recordChatMessage(UUID uuid, String name) {
        if (config != null && !config.isAnalyticsEnabled()) return;
        addStat(uuid, name, "messages", 1);
    }

    void addStat(UUID uuid, String name, String key, long amount) {
        if (amount <= 0) return;
        StatHolder holder = stats.computeIfAbsent(uuid, id -> new StatHolder(name, new ConcurrentHashMap<>()));
        holder.counters.computeIfAbsent(key, k -> new AtomicLong()).addAndGet(amount);
    }

    void addEvent(UUID uuid, String name, String kind, String detail) {
        if (events.size() >= 5000) return; // drop if excessive buffer
        JsonObject obj = new JsonObject();
        obj.addProperty("uuid", uuid.toString());
        obj.addProperty("name", name);
        obj.addProperty("kind", kind);
        if (detail != null && !detail.isBlank()) {
            obj.addProperty("detail", detail.substring(0, Math.min(256, detail.length())));
        }
        obj.addProperty("at", System.currentTimeMillis());
        events.add(obj);
    }

    void addAction(UUID uuid, String name, String action, long count) {
        if (count <= 0) return;
        String key = uuid + ":" + action;
        if (actions.size() >= 5000 && !actions.containsKey(key)) return;
        actions.compute(key, (k, existing) -> {
            if (existing == null) {
                return new ActionHolder(uuid, name, action, new AtomicLong(count), System.currentTimeMillis());
            } else {
                existing.count().addAndGet(count);
                return existing;
            }
        });
    }

    private String getDimensionName(World world) {
        if (world == null) return "minecraft:overworld";
        return world.getKey().toString();
    }

    @EventHandler(priority = EventPriority.MONITOR)
    public void onPlayerJoin(PlayerJoinEvent event) {
        if (!config.isAnalyticsEnabled()) return;
        Player p = event.getPlayer();
        sessions.put(p.getUniqueId(), new Session(p.getName(), System.nanoTime()));
        addStat(p.getUniqueId(), p.getName(), "joins", 1);
        addEvent(p.getUniqueId(), p.getName(), "join", null);
    }

    @EventHandler(priority = EventPriority.MONITOR)
    public void onPlayerQuit(PlayerQuitEvent event) {
        if (!config.isAnalyticsEnabled()) return;
        Player p = event.getPlayer();
        Session s = sessions.remove(p.getUniqueId());
        long seconds = s != null ? Math.max(0L, (System.nanoTime() - s.joinTimeNanos()) / 1_000_000_000L) : 0L;
        addStat(p.getUniqueId(), p.getName(), "playtime_secs", seconds);
        addEvent(p.getUniqueId(), p.getName(), "leave", null);
    }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void onBlockBreak(BlockBreakEvent event) {
        if (!config.isAnalyticsEnabled() || !config.isTrackBlocks()) return;
        Player p = event.getPlayer();
        addStat(p.getUniqueId(), p.getName(), "blocks_broken", 1);
        String action = "block_broken:" + event.getBlock().getType().name() + "@" + getDimensionName(event.getBlock().getWorld());
        addAction(p.getUniqueId(), p.getName(), action, 1);
    }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void onBlockPlace(BlockPlaceEvent event) {
        if (!config.isAnalyticsEnabled() || !config.isTrackBlocks()) return;
        Player p = event.getPlayer();
        addStat(p.getUniqueId(), p.getName(), "blocks_placed", 1);
        String action = "block_placed:" + event.getBlock().getType().name() + "@" + getDimensionName(event.getBlock().getWorld());
        addAction(p.getUniqueId(), p.getName(), action, 1);
    }

    @EventHandler(priority = EventPriority.MONITOR)
    public void onPlayerDeath(PlayerDeathEvent event) {
        if (!config.isAnalyticsEnabled() || !config.isTrackCombat()) return;
        Player victim = event.getEntity();
        addStat(victim.getUniqueId(), victim.getName(), "deaths", 1);
        String deathMsg = event.getDeathMessage();
        addEvent(victim.getUniqueId(), victim.getName(), "death", deathMsg);

        Player killer = victim.getKiller();
        if (killer != null && !killer.getUniqueId().equals(victim.getUniqueId())) {
            addStat(killer.getUniqueId(), killer.getName(), "player_kills", 1);
            addEvent(killer.getUniqueId(), killer.getName(), "pvp_kill", victim.getUniqueId().toString());
        }
    }

    @EventHandler(priority = EventPriority.MONITOR)
    public void onEntityDeath(EntityDeathEvent event) {
        if (!config.isAnalyticsEnabled() || !config.isTrackCombat()) return;
        if (event instanceof PlayerDeathEvent) return; // handled above

        Player killer = event.getEntity().getKiller();
        if (killer != null) {
            addStat(killer.getUniqueId(), killer.getName(), "mob_kills", 1);
            addAction(killer.getUniqueId(), killer.getName(), "mob_kills:" + event.getEntityType().name(), 1);
        }
    }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void onCommand(PlayerCommandPreprocessEvent event) {
        if (!config.isAnalyticsEnabled() || !config.isTrackCommands()) return;
        String raw = event.getMessage().strip();
        String cmd = raw.split("\\s+", 2)[0];
        addEvent(event.getPlayer().getUniqueId(), event.getPlayer().getName(), "command", cmd);
    }

    @EventHandler(priority = EventPriority.MONITOR)
    public void onAdvancement(PlayerAdvancementDoneEvent event) {
        if (!config.isAnalyticsEnabled()) return;
        String key = event.getAdvancement().getKey().toString();
        // Ignore recipe advancements
        if (key.startsWith("minecraft:recipes/")) return;
        addEvent(event.getPlayer().getUniqueId(), event.getPlayer().getName(), "advancement", key);
    }

    public void sync() {
        if (!config.isAnalyticsEnabled() || !panelClient.isLinked()) return;
        if (!isSyncing.compareAndSet(false, true)) return;

        try {
            JsonObject payload = buildPayload(currentTps);
            SyncResult result = panelClient.sync(payload);

            if (result.ok()) {
                // Handle kicked players if specified by the panel
                if (!result.kickPlayers().isEmpty()) {
                    Bukkit.getScheduler().runTask(plugin, () -> {
                        for (UUID id : result.kickPlayers()) {
                            Player p = Bukkit.getPlayer(id);
                            if (p != null && p.isOnline()) {
                                p.kick(Component.text("Your account session was revoked by the panel."));
                            }
                        }
                    });
                }
            } else {
                logger.warning("Panel analytics sync reported failure: " + result.errorMessage());
            }
        } catch (Exception e) {
            logger.warning("Error performing panel analytics sync: " + e.getMessage());
        } finally {
            isSyncing.set(false);
        }
    }

    JsonObject buildPayload(double tps) {
        JsonObject payload = new JsonObject();
        payload.addProperty("batch_id", UUID.randomUUID().toString());
        payload.addProperty("tps", Double.isFinite(tps) ? Math.min(20.0, Math.max(0.0, tps)) : 20.0);

        // Online roster & active playtime update
        JsonArray onlineArray = new JsonArray();
        long now = System.nanoTime();
        sessions.replaceAll((uuid, session) -> {
            long seconds = Math.max(0L, (now - session.joinTimeNanos()) / 1_000_000_000L);
            addStat(uuid, session.name(), "playtime_secs", seconds);

            JsonObject pObj = new JsonObject();
            pObj.addProperty("uuid", uuid.toString());
            pObj.addProperty("name", session.name());
            onlineArray.add(pObj);

            return new Session(session.name(), session.joinTimeNanos() + seconds * 1_000_000_000L);
        });
        payload.add("online", onlineArray);

        // Stats deltas
        JsonArray statsArray = new JsonArray();
        for (Iterator<Map.Entry<UUID, StatHolder>> it = stats.entrySet().iterator(); it.hasNext(); ) {
            Map.Entry<UUID, StatHolder> entry = it.next();
            UUID uuid = entry.getKey();
            StatHolder holder = entry.getValue();

            JsonObject row = new JsonObject();
            row.addProperty("uuid", uuid.toString());
            row.addProperty("name", holder.name);

            boolean hasValues = false;
            for (Iterator<Map.Entry<String, AtomicLong>> cIt = holder.counters.entrySet().iterator(); cIt.hasNext(); ) {
                Map.Entry<String, AtomicLong> cEntry = cIt.next();
                long val = cEntry.getValue().getAndSet(0);
                if (val > 0) {
                    row.addProperty(cEntry.getKey(), val);
                    hasValues = true;
                }
            }
            if (hasValues) {
                statsArray.add(row);
            }
        }
        payload.add("stats", statsArray);

        // Events
        JsonArray eventsArray = new JsonArray();
        synchronized (events) {
            int take = Math.min(events.size(), 300);
            for (int i = 0; i < take; i++) {
                eventsArray.add(events.get(i));
            }
            events.subList(0, take).clear();
        }

        // Actions converted to event entries
        for (Iterator<Map.Entry<String, ActionHolder>> it = actions.entrySet().iterator(); it.hasNext(); ) {
            Map.Entry<String, ActionHolder> entry = it.next();
            ActionHolder holder = entry.getValue();
            long count = holder.count().getAndSet(0);
            if (count > 0 && eventsArray.size() < 450) {
                JsonObject actObj = new JsonObject();
                actObj.addProperty("uuid", holder.uuid().toString());
                actObj.addProperty("name", holder.name());
                actObj.addProperty("kind", "action");
                actObj.addProperty("detail", holder.action() + " +" + count);
                actObj.addProperty("at", holder.at());
                eventsArray.add(actObj);
            }
            it.remove();
        }
        payload.add("events", eventsArray);

        return payload;
    }
}
