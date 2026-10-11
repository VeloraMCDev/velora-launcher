package net.velora.paper.config;

import org.bukkit.configuration.file.FileConfiguration;
import org.bukkit.plugin.Plugin;

public final class VeloraConfig {
    private final Plugin plugin;
    private String panelUrl;
    private String token;

    // Auth
    private boolean authEnabled;
    private int authTimeoutSeconds;
    private String unavailableMessage;
    private String unauthorizedMessage;

    // Analytics
    private boolean analyticsEnabled;
    private int syncIntervalSeconds;
    private boolean trackBlocks;
    private boolean trackCombat;
    private boolean trackCommands;

    // Chat
    private boolean chatEnabled;
    private String chatFormat;
    private boolean allowColors;

    public VeloraConfig(Plugin plugin) {
        this.plugin = plugin;
        load();
    }

    public void load() {
        plugin.reloadConfig();
        FileConfiguration config = plugin.getConfig();

        String rawUrl = config.getString("panel-url", "http://localhost:3000");
        if (rawUrl == null || rawUrl.isBlank()) {
            rawUrl = "http://localhost:3000";
        }
        if (rawUrl.endsWith("/")) {
            rawUrl = rawUrl.substring(0, rawUrl.length() - 1);
        }
        this.panelUrl = rawUrl;

        String rawToken = config.getString("token", "");
        this.token = rawToken != null ? rawToken.trim() : "";

        this.authEnabled = config.getBoolean("auth.enabled", true);
        this.authTimeoutSeconds = Math.max(1, config.getInt("auth.timeout-seconds", 6));
        this.unavailableMessage = config.getString("auth.unavailable-message", "Velora sign-in is unavailable. Please try again shortly.");
        this.unauthorizedMessage = config.getString("auth.unauthorized-message", "You are not authorized to join this server. Please use the Velora launcher.");

        this.analyticsEnabled = config.getBoolean("analytics.enabled", true);
        this.syncIntervalSeconds = Math.max(5, config.getInt("analytics.sync-interval-seconds", 30));
        this.trackBlocks = config.getBoolean("analytics.track-blocks", true);
        this.trackCombat = config.getBoolean("analytics.track-combat", true);
        this.trackCommands = config.getBoolean("analytics.track-commands", true);

        this.chatEnabled = config.getBoolean("chat.enabled", true);
        this.chatFormat = config.getString("chat.format", "{prefix}&f{player}{suffix}&7: &f{message}");
        this.allowColors = config.getBoolean("chat.allow-colors", true);
    }

    public void setToken(String token) {
        this.token = token != null ? token.trim() : "";
        plugin.getConfig().set("token", this.token);
        plugin.saveConfig();
    }

    public String getPanelUrl() { return panelUrl; }
    public String getToken() { return token; }
    public boolean hasToken() { return token != null && !token.isBlank(); }

    public boolean isAuthEnabled() { return authEnabled; }
    public int getAuthTimeoutSeconds() { return authTimeoutSeconds; }
    public String getUnavailableMessage() { return unavailableMessage; }
    public String getUnauthorizedMessage() { return unauthorizedMessage; }

    public boolean isAnalyticsEnabled() { return analyticsEnabled; }
    public int getSyncIntervalSeconds() { return syncIntervalSeconds; }
    public boolean isTrackBlocks() { return trackBlocks; }
    public boolean isTrackCombat() { return trackCombat; }
    public boolean isTrackCommands() { return trackCommands; }

    public boolean isChatEnabled() { return chatEnabled; }
    public String getChatFormat() { return chatFormat; }
    public boolean isAllowColors() { return allowColors; }
}
