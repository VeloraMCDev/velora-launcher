package net.velora.paper;

import net.velora.paper.analytics.AnalyticsTracker;
import net.velora.paper.auth.AuthListener;
import net.velora.paper.chat.ChatFormatter;
import net.velora.paper.chat.ChatListener;
import net.velora.paper.chat.LuckPermsHook;
import net.velora.paper.command.VeloraCommand;
import net.velora.paper.config.VeloraConfig;
import net.velora.paper.panel.HelloResult;
import net.velora.paper.panel.PanelClient;
import org.bukkit.Bukkit;
import org.bukkit.command.PluginCommand;
import org.bukkit.plugin.java.JavaPlugin;

public final class VeloraPlugin extends JavaPlugin {
    private VeloraConfig config;
    private PanelClient panelClient;
    private AnalyticsTracker analyticsTracker;
    private LuckPermsHook luckPerms;
    private ChatFormatter chatFormatter;

    @Override
    public void onEnable() {
        saveDefaultConfig();

        this.config = new VeloraConfig(this);
        this.panelClient = new PanelClient(config, getLogger());
        this.analyticsTracker = new AnalyticsTracker(this, config, panelClient, getLogger());
        this.luckPerms = new LuckPermsHook();
        this.chatFormatter = new ChatFormatter(config, luckPerms);

        // Register event listeners
        getServer().getPluginManager().registerEvents(new AuthListener(config, panelClient, getLogger()), this);
        getServer().getPluginManager().registerEvents(analyticsTracker, this);
        getServer().getPluginManager().registerEvents(new ChatListener(config, chatFormatter, analyticsTracker), this);

        // Register command
        PluginCommand cmd = getCommand("velora");
        if (cmd != null) {
            VeloraCommand executor = new VeloraCommand(this, config, panelClient, analyticsTracker);
            cmd.setExecutor(executor);
            cmd.setTabCompleter(executor);
        }

        // Start analytics rolling TPS and sync schedule
        analyticsTracker.start();

        // Perform initial handshake with the panel asynchronously
        Bukkit.getScheduler().runTaskAsynchronously(this, this::performHandshake);

        getLogger().info("Velora Paper plugin v" + getDescription().getVersion() + " (Paper 26.3) enabled.");
    }

    @Override
    public void onDisable() {
        if (analyticsTracker != null) {
            analyticsTracker.stop();
        }
        getLogger().info("Velora Paper plugin disabled.");
    }

    public HelloResult performHandshake() {
        String software = "Paper " + Bukkit.getVersion();
        String mcVersion = Bukkit.getMinecraftVersion();
        String pluginVersion = getDescription().getVersion();
        boolean onlineMode = Bukkit.getOnlineMode();
        int maxPlayers = Bukkit.getMaxPlayers();

        HelloResult result = panelClient.hello(software, mcVersion, pluginVersion, onlineMode, maxPlayers);
        if (result.ok()) {
            getLogger().info("Successfully linked to Velora Panel as server '" + result.serverName() + "' (ID: " + result.serverId() + ")");
        } else {
            getLogger().warning("Panel linking status: " + result.errorMessage());
            if (!config.hasToken()) {
                getLogger().warning("Configure 'token' in config.yml or run '/velora link <token>' to link this server to the panel.");
            }
        }
        return result;
    }

    public VeloraConfig getPluginConfig() {
        return config;
    }

    public PanelClient getPanelClient() {
        return panelClient;
    }

    public AnalyticsTracker getAnalyticsTracker() {
        return analyticsTracker;
    }

    public ChatFormatter getChatFormatter() {
        return chatFormatter;
    }
}
