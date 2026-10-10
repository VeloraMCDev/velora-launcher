package net.scopenet.integration;

import net.velora.core.Legacy;
import java.io.IOException;
import java.io.Reader;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Properties;

public record Settings(
        URI panel,
        String token,
        boolean levelingEnabled,
        double globalXpMultiplier,
        double serverXpMultiplier,
        boolean questsEnabled,
        boolean achievementsEnabled,
        boolean guildsEnabled,
        boolean landClaimingEnabled,
        boolean socialEnabled,
        boolean chatPrefixesEnabled,
        boolean essentialsEnabled,
        boolean economyEnabled
) {
    public Settings {
        if (panel == null || panel.getHost() == null || panel.getRawUserInfo() != null
                || panel.getRawQuery() != null || panel.getRawFragment() != null
                || !("https".equals(panel.getScheme()) || "http".equals(panel.getScheme()))) {
            throw new IllegalArgumentException("panel-url must be an HTTP(S) URL without credentials, query or fragment");
        }
        token = token == null ? "" : token.trim();
        if (!token.matches("sn_[A-Za-z0-9]{40}")) {
            throw new IllegalArgumentException("Set token in config/velora-core.properties (copy it from the panel's Servers page)");
        }
    }

    public Settings(URI panel, String token) {
        this(panel, token, true, 1.0, 1.5, true, true, true, true, true, true, true, true);
    }

    public Settings(
            URI panel,
            String token,
            boolean levelingEnabled,
            double globalXpMultiplier,
            double serverXpMultiplier,
            boolean questsEnabled,
            boolean achievementsEnabled,
            boolean guildsEnabled,
            boolean landClaimingEnabled,
            boolean socialEnabled,
            boolean chatPrefixesEnabled
    ) {
        this(panel, token, levelingEnabled, globalXpMultiplier, serverXpMultiplier, questsEnabled, achievementsEnabled, guildsEnabled, landClaimingEnabled, socialEnabled, chatPrefixesEnabled, true, true);
    }

    public static Settings of(String panel, String token) {
        return of(panel, token, true, 1.0, 1.5, true, true, true, true, true, true, true, true);
    }

    public static Settings of(
            String panel,
            String token,
            boolean levelingEnabled,
            double globalXpMultiplier,
            double serverXpMultiplier,
            boolean questsEnabled,
            boolean achievementsEnabled,
            boolean guildsEnabled,
            boolean landClaimingEnabled,
            boolean socialEnabled,
            boolean chatPrefixesEnabled
    ) {
        return of(panel, token, levelingEnabled, globalXpMultiplier, serverXpMultiplier, questsEnabled, achievementsEnabled, guildsEnabled, landClaimingEnabled, socialEnabled, chatPrefixesEnabled, true, true);
    }

    public static Settings of(
            String panel,
            String token,
            boolean levelingEnabled,
            double globalXpMultiplier,
            double serverXpMultiplier,
            boolean questsEnabled,
            boolean achievementsEnabled,
            boolean guildsEnabled,
            boolean landClaimingEnabled,
            boolean socialEnabled,
            boolean chatPrefixesEnabled,
            boolean essentialsEnabled,
            boolean economyEnabled
    ) {
        return new Settings(
                URI.create(panel.trim().replaceAll("/+$", "")),
                token,
                levelingEnabled,
                globalXpMultiplier,
                serverXpMultiplier,
                questsEnabled,
                achievementsEnabled,
                guildsEnabled,
                landClaimingEnabled,
                socialEnabled,
                chatPrefixesEnabled,
                essentialsEnabled,
                economyEnabled
        );
    }

    /** Local opt-out for the Velora Map (the panel also has a per-server switch). */
    public static boolean mapEnabled(Path path) {
        try (Reader reader = Files.newBufferedReader(path, StandardCharsets.UTF_8)) {
            Properties properties = new Properties();
            properties.load(reader);
            return net.velora.core.CoreModules.local(properties).enabled("map") && Boolean.parseBoolean(properties.getProperty("map.enabled", properties.getProperty("livemap.enabled", "true")));
        } catch (IOException e) {
            return true;
        }
    }

    /** Writes the documented default file if none exists. Safe to call early; never overwrites. */
    public static void ensureDefault(Path path) throws IOException {
        if (!Files.exists(path) && !Legacy.copyServerConfig(path)) {
            Path parent = path.toAbsolutePath().getParent();
            if (parent != null) Files.createDirectories(parent);
            String defaultProps = """
                    # Velora Core (Server) configuration. Edit, then restart the server or run /velora reload.
                    #
                    # 1. In the panel open Servers, create (or open) this server and copy its token.
                    # 2. Set panel-url to your panel's address and paste the token below.
                    # 3. Keep online-mode=true and start the server with the panel's authlib-injector flag
                    #    (shown in the panel's connect dialog) so players sign in with their Velora account.
                    panel-url=https://panel.example.com
                    token=

                    # Feature Toggles (set false to disable systems on this server)
                    leveling.enabled=true
                    leveling.global_xp_multiplier=1.0
                    leveling.server_xp_multiplier=1.5
                    quests.enabled=true
                    achievements.enabled=true
                    guilds.enabled=true
                    guilds.land_claiming=true
                    social.enabled=true
                    social.chat_prefixes=true
                    essentials.enabled=true
                    economy.enabled=true

                    # Essentials limits. Players can exceed max_homes with the permission velora.homes.<number>
                    # (or velora.homes.unlimited). Cooldowns are seconds, 0 = none; velora.cooldown.bypass skips them.
                    essentials.max_homes=5
                    essentials.home_name_max_length=32
                    essentials.max_warps=0
                    essentials.tpa_timeout_seconds=60
                    essentials.rtp_radius=2500
                    essentials.rtp_min_radius=0
                    essentials.rtp_attempts=15
                    essentials.cooldown.spawn=0
                    essentials.cooldown.home=0
                    essentials.cooldown.back=0
                    essentials.cooldown.tpa=0
                    essentials.cooldown.rtp=0
                    essentials.cooldown.warp=0

                    # Rewards from quests, achievements and rank milestones (items, permissions, groups, messages).
                    # allow_commands lets the panel run its own console commands as rewards: leave off unless you trust every panel admin.
                    rewards.enabled=true
                    rewards.allow_commands=false
                    rewards.permission_command=lp user {player} permission set {node} true
                    rewards.permission_temp_command=lp user {player} permission settemp {node} {value} {duration}
                    rewards.group_command=lp user {player} parent add {group}

                    # Draw this world on the Velora Map: tiles, player positions, claims and pins go to the panel.
                    # It only runs when the map is also enabled for this server in the panel.
                    map.enabled=true

                    # Core modules. false narrows what the panel allows on this server; true never overrides the panel.
                    modules.map.enabled=true
                    modules.economy.enabled=true
                    modules.vaults.enabled=true
                    modules.casino.enabled=true
                    modules.analytics.enabled=true
                    modules.factions.enabled=true
                    modules.permissions_chat.enabled=true

                    # Who may use Velora commands when LuckPerms is absent: "all" or "op".
                    permissions.default_level=all
                    # Lets players with Velora Core (Client) use the in-game hub and map.
                    clientlink.enabled=true
                    economy.currency_symbol=$
                    """;
            Files.writeString(path, defaultProps, StandardCharsets.UTF_8);
        }
    }

    public static Settings load(Path path) throws IOException {
        ensureDefault(path);
        Properties properties = new Properties();
        try (Reader reader = Files.newBufferedReader(path, StandardCharsets.UTF_8)) {
            properties.load(reader);
        }
        return of(
                properties.getProperty("panel-url", ""),
                properties.getProperty("token", ""),
                Boolean.parseBoolean(properties.getProperty("leveling.enabled", "true")),
                Double.parseDouble(properties.getProperty("leveling.global_xp_multiplier", "1.0")),
                Double.parseDouble(properties.getProperty("leveling.server_xp_multiplier", "1.5")),
                Boolean.parseBoolean(properties.getProperty("quests.enabled", "true")),
                Boolean.parseBoolean(properties.getProperty("achievements.enabled", "true")),
                Boolean.parseBoolean(properties.getProperty("guilds.enabled", "true")),
                Boolean.parseBoolean(properties.getProperty("guilds.land_claiming", "true")),
                Boolean.parseBoolean(properties.getProperty("social.enabled", "true")),
                Boolean.parseBoolean(properties.getProperty("social.chat_prefixes", "true")),
                Boolean.parseBoolean(properties.getProperty("essentials.enabled", "true")),
                Boolean.parseBoolean(properties.getProperty("economy.enabled", "true"))
        );
    }
}
