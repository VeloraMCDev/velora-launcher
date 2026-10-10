package net.velora.core;

import java.util.Map;

/**
 * Every tunable of homes, warps, TPA and RTP. Players can exceed {@code maxHomes} with a permission node
 * ({@code velora.homes.<number>}, or {@code velora.homes.unlimited}); {@code cooldowns} are seconds per command
 * (0 = none) and are skipped for anyone with {@code velora.cooldown.bypass}.
 */
public record EssentialsConfig(int maxHomes, int rtpRadius, long tpaTimeoutMs, int rtpAttempts, int rtpMinRadius, int maxWarps,
                               int homeNameMax, Map<String, Integer> cooldowns) {
    public static final int TIER_CAP = 500;

    public EssentialsConfig(int maxHomes, int rtpRadius, long tpaTimeoutMs, int rtpAttempts) {
        this(maxHomes, rtpRadius, tpaTimeoutMs, rtpAttempts, 0, 0, 32, Map.of());
    }

    public static EssentialsConfig defaults() { return new EssentialsConfig(5, 2500, 60_000, 15); }

    public int cooldown(String command) { return cooldowns == null ? 0 : cooldowns.getOrDefault(command, 0); }
}
