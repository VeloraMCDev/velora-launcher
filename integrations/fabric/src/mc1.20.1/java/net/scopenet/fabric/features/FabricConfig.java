package net.scopenet.fabric.features;

import net.scopenet.core.EssentialsConfig;
import net.scopenet.core.Features;
import net.scopenet.integration.Settings;

import java.io.IOException;
import java.io.Reader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Properties;

/** The Fabric-only keys in config/scopenet.properties (the shared keys are read by {@link Settings}). */
final class FabricConfig {
    static final Path FILE = Path.of("config", "scopenet.properties");
    private final Properties p = new Properties();

    static FabricConfig load() {
        FabricConfig c = new FabricConfig();
        if (Files.exists(FILE)) {
            try (Reader in = Files.newBufferedReader(FILE, StandardCharsets.UTF_8)) { c.p.load(in); }
            catch (IOException e) { System.err.println("[Velora] Could not read " + FILE + ": " + e.getMessage()); }
        }
        return c;
    }

    private int integer(String key, int fallback) {
        try { return Integer.parseInt(p.getProperty(key, String.valueOf(fallback)).trim()); } catch (NumberFormatException e) { return fallback; }
    }

    boolean bool(String key, boolean fallback) { return Boolean.parseBoolean(p.getProperty(key, String.valueOf(fallback)).trim()); }
    String text(String key, String fallback) { return p.getProperty(key, fallback).trim(); }

    EssentialsConfig essentials() {
        java.util.Map<String, Integer> cooldowns = new java.util.HashMap<>();
        for (String cmd : new String[]{"spawn", "home", "back", "tpa", "rtp", "warp"}) cooldowns.put(cmd, Math.max(0, integer("essentials.cooldown." + cmd, 0)));
        return new EssentialsConfig(Math.max(0, integer("essentials.max_homes", 5)), Math.max(10, integer("essentials.rtp_radius", 2500)),
                Math.max(5, integer("essentials.tpa_timeout_seconds", 60)) * 1000L, Math.max(1, integer("essentials.rtp_attempts", 15)),
                Math.max(0, integer("essentials.rtp_min_radius", 0)), Math.max(0, integer("essentials.max_warps", 0)),
                Math.max(1, integer("essentials.home_name_max_length", 32)), cooldowns);
    }

    Features features(Settings s) {
        return new Features(s.essentialsEnabled(), s.economyEnabled(), s.guildsEnabled(), s.socialEnabled(), s.landClaimingEnabled(),
                bool("clientlink.enabled", true), text("economy.currency_symbol", "$"));
    }

    net.scopenet.core.RewardRules rewards() {
        net.scopenet.core.RewardRules d = net.scopenet.core.RewardRules.defaults();
        return new net.scopenet.core.RewardRules(bool("rewards.enabled", true), bool("rewards.allow_commands", false),
                text("rewards.permission_command", d.permissionCommand()), text("rewards.permission_deny_command", d.permissionDenyCommand()),
                text("rewards.permission_temp_command", d.permissionTempCommand()), text("rewards.group_command", d.groupCommand()));
    }

    /** {@code permissions.default_level}: "all" lets everyone use commands unless LuckPerms says otherwise, "op" limits them to operators. */
    boolean everyoneByDefault() { return !text("permissions.default_level", "all").equalsIgnoreCase("op"); }
}
