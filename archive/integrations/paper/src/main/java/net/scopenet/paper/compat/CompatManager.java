package net.scopenet.paper.compat;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import net.scopenet.integration.Integration;
import org.bukkit.Bukkit;
import org.bukkit.event.EventHandler;
import org.bukkit.event.EventPriority;
import org.bukkit.event.Listener;
import org.bukkit.event.server.PluginDisableEvent;
import org.bukkit.event.server.PluginEnableEvent;
import org.bukkit.plugin.Plugin;
import org.bukkit.plugin.java.JavaPlugin;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.function.Supplier;

/**
 * Finds the plugins Velora can work with and switches each integration on or
 * off as they come and go, so nothing is hard-wired into {@code ScopenetPlugin}.
 * Every integration can also be turned off with {@code integrations.<id>.enabled: false}.
 *
 * Integrations are only instantiated once their plugin is present: their classes refer to that
 * plugin's API, which must not be loaded on servers that don't have it.
 */
public final class CompatManager implements Listener {
    public enum State { ACTIVE, NOT_INSTALLED, DISABLED_IN_CONFIG, FAILED }

    public record Status(String id, String name, String plugin, State state, String version, String message, Map<String, Object> details) {}

    /** What we know about an integration before (and without) loading its code. */
    private record Candidate(String id, String name, String plugin, Supplier<IntegrationModule> factory) {}

    private final JavaPlugin plugin;
    private final CompatContext context;
    private final List<Candidate> candidates = new ArrayList<>();
    private final Map<String, IntegrationModule> running = new ConcurrentHashMap<>();
    private final Map<String, String> failures = new ConcurrentHashMap<>();
    private int reportTask = -1;

    public CompatManager(JavaPlugin plugin, Integration integration, ScopenetApiImpl api) {
        this.plugin = plugin;
        this.context = new CompatContext(plugin, integration, api);
        add("luckperms", "LuckPerms", "LuckPerms", LuckPermsModule::new);
        add("placeholderapi", "PlaceholderAPI", "PlaceholderAPI", PlaceholderApiModule::new);
        add("vault", "Vault", "Vault", VaultModule::new);
        add("coreprotect", "CoreProtect", "CoreProtect", CoreProtectModule::new);
        add("worldguard", "WorldGuard", "WorldGuard", WorldGuardModule::new);
        add("spark", "Spark", "spark", SparkModule::new);
    }

    private void add(String id, String name, String pluginName, Supplier<IntegrationModule> factory) {
        candidates.add(new Candidate(id, name, pluginName, factory));
    }

    /** Register listeners and enable whatever is installed. Call on the server thread once all plugins have loaded. */
    public void start() {
        Bukkit.getPluginManager().registerEvents(this, plugin);
        for (Candidate candidate : candidates) tryEnable(candidate);
        // Tell the panel which integrations are live, now and every few minutes.
        reportTask = context.everyAsync(100, 20L * 60 * 5, this::reportStatus);
    }

    public void stop() {
        if (reportTask >= 0) context.cancel(reportTask);
        for (Candidate candidate : candidates) disable(candidate);
    }

    private Plugin installed(Candidate candidate) {
        Plugin other = Bukkit.getPluginManager().getPlugin(candidate.plugin());
        return other != null && other.isEnabled() ? other : null;
    }

    private boolean allowed(Candidate candidate) {
        return context.getBoolean(candidate.id(), "enabled", true);
    }

    private void tryEnable(Candidate candidate) {
        if (running.containsKey(candidate.id()) || !allowed(candidate) || installed(candidate) == null) return;
        IntegrationModule module = null;
        try {
            module = candidate.factory().get();
            module.enable(context);
            running.put(candidate.id(), module);
            failures.remove(candidate.id());
            context.log().info("Integration enabled: " + candidate.name());
        } catch (Throwable t) {
            // A broken or incompatible plugin version must never take Velora down with it.
            String why = t.getClass().getSimpleName() + (t.getMessage() != null ? ": " + t.getMessage() : "");
            failures.put(candidate.id(), why);
            context.log().warning("Could not enable the " + candidate.name() + " integration: " + why);
            if (module != null) {
                try { module.disable(); } catch (Throwable ignored) { /* partially started */ }
            }
        }
    }

    private void disable(Candidate candidate) {
        IntegrationModule module = running.remove(candidate.id());
        if (module == null) return;
        try { module.disable(); }
        catch (Throwable t) { context.log().warning("Error stopping the " + candidate.name() + " integration: " + t.getMessage()); }
        context.log().info("Integration disabled: " + candidate.name());
    }

    @EventHandler(priority = EventPriority.MONITOR)
    public void pluginEnabled(PluginEnableEvent event) {
        for (Candidate candidate : candidates) {
            if (candidate.plugin().equalsIgnoreCase(event.getPlugin().getName())) Bukkit.getScheduler().runTask(plugin, () -> tryEnable(candidate));
        }
    }

    @EventHandler(priority = EventPriority.MONITOR)
    public void pluginDisabled(PluginDisableEvent event) {
        for (Candidate candidate : candidates) {
            if (candidate.plugin().equalsIgnoreCase(event.getPlugin().getName())) disable(candidate);
        }
    }

    /** Current state of every integration, for /scopenet status and the panel. */
    public List<Status> statuses() {
        List<Status> out = new ArrayList<>();
        for (Candidate c : candidates) {
            Plugin other = Bukkit.getPluginManager().getPlugin(c.plugin());
            String version = other == null ? "" : other.getDescription().getVersion();
            IntegrationModule module = running.get(c.id());
            State state;
            String message = "";
            if (!allowed(c)) { state = State.DISABLED_IN_CONFIG; message = "Turned off in config.yml"; }
            else if (failures.containsKey(c.id())) { state = State.FAILED; message = failures.get(c.id()); }
            else if (module != null) state = State.ACTIVE;
            else { state = State.NOT_INSTALLED; message = c.plugin() + " is not installed"; }
            out.add(new Status(c.id(), c.name(), c.plugin(), state, version, message, module == null ? Map.of() : module.details()));
        }
        return out;
    }

    private void reportStatus() {
        try {
            JsonObject data = new JsonObject();
            JsonArray list = new JsonArray();
            for (Status s : statuses()) {
                JsonObject o = new JsonObject();
                o.addProperty("id", s.id());
                o.addProperty("name", s.name());
                o.addProperty("plugin", s.plugin());
                o.addProperty("state", s.state().name().toLowerCase(Locale.ROOT));
                o.addProperty("version", s.version());
                o.addProperty("message", s.message());
                JsonObject details = new JsonObject();
                s.details().forEach((k, v) -> {
                    if (v instanceof Number n) details.addProperty(k, n);
                    else if (v instanceof Boolean b) details.addProperty(k, b);
                    else details.addProperty(k, String.valueOf(v));
                });
                o.add("details", details);
                list.add(o);
            }
            data.add("modules", list);
            context.report("compat", plugin.getDescription().getVersion(), data);
        } catch (Exception e) {
            context.log().fine("Could not report integrations to the panel: " + e.getMessage());
        }
    }
}
