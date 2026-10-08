package net.scopenet.paper.compat;

import com.google.gson.JsonObject;
import net.scopenet.integration.Integration;
import org.bukkit.Bukkit;
import org.bukkit.configuration.file.FileConfiguration;
import org.bukkit.plugin.java.JavaPlugin;
import java.nio.file.Path;
import java.util.logging.Logger;

/** What an {@link IntegrationModule} gets to work with. */
public final class CompatContext {
    private final JavaPlugin plugin;
    private final Integration integration;
    private final ScopenetApiImpl api;

    CompatContext(JavaPlugin plugin, Integration integration, ScopenetApiImpl api) {
        this.plugin = plugin;
        this.integration = integration;
        this.api = api;
    }

    public JavaPlugin plugin() { return plugin; }
    public Integration integration() { return integration; }
    public ScopenetApiImpl api() { return api; }
    public Logger log() { return plugin.getLogger(); }
    public FileConfiguration config() { return plugin.getConfig(); }
    public Path dataFolder() { return plugin.getDataFolder().toPath(); }

    public boolean getBoolean(String module, String key, boolean fallback) {
        return config().getBoolean("integrations." + module + "." + key, fallback);
    }

    public int getInt(String module, String key, int fallback) {
        return config().getInt("integrations." + module + "." + key, fallback);
    }

    public String getString(String module, String key, String fallback) {
        return config().getString("integrations." + module + "." + key, fallback);
    }

    /** Run later on the server thread. */
    public void sync(Runnable task) {
        if (plugin.isEnabled()) Bukkit.getScheduler().runTask(plugin, task);
    }

    /** Run on a worker thread. */
    public void async(Runnable task) {
        if (plugin.isEnabled()) Bukkit.getScheduler().runTaskAsynchronously(plugin, task);
    }

    /** Repeat on the server thread. Returns the task id to pass to {@link #cancel}. */
    public int everySync(long delayTicks, long periodTicks, Runnable task) {
        return Bukkit.getScheduler().runTaskTimer(plugin, task, delayTicks, periodTicks).getTaskId();
    }

    /** Repeat on a worker thread. */
    public int everyAsync(long delayTicks, long periodTicks, Runnable task) {
        return Bukkit.getScheduler().runTaskTimerAsynchronously(plugin, task, delayTicks, periodTicks).getTaskId();
    }

    public void cancel(int taskId) {
        Bukkit.getScheduler().cancelTask(taskId);
    }

    /**
     * Send a report about an integration to the panel (call from a worker thread). Returns the
     * panel's answer, whose {@code config} member carries settings for that integration.
     */
    public JsonObject report(String name, String version, JsonObject data) throws Exception {
        JsonObject body = new JsonObject();
        body.addProperty("name", name);
        body.addProperty("version", version == null ? "" : version);
        body.add("data", data);
        return integration.client().post("integrations/report", body);
    }
}
