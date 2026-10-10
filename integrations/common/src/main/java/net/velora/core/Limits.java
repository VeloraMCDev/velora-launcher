package net.velora.core;

import java.util.HashMap;
import java.util.Map;
import java.util.UUID;
import java.util.function.Predicate;

/** Per-player limits that permission nodes can raise, and command cooldowns. */
public final class Limits {
    private Limits() {}

    /**
     * The highest tier a player has, e.g. {@code velora.homes.10}, otherwise {@code base}. {@code <prefix>unlimited}
     * means no limit. Only values above {@code base} are looked for, up to {@code cap}.
     */
    public static int tier(Predicate<String> has, String prefix, int base, int cap) {
        if (has.test(prefix + "unlimited")) return Integer.MAX_VALUE;
        for (int n = cap; n > base; n--) if (has.test(prefix + n)) return n;
        return base;
    }

    public static String show(int limit) { return limit == Integer.MAX_VALUE ? "unlimited" : Integer.toString(limit); }

    /** When each player may use each command again. Not thread-safe: use from the server thread. */
    public static final class Cooldowns {
        private final Map<String, Long> until = new HashMap<>();

        public long remainingMs(UUID player, String command, long now) {
            Long end = until.get(player + ":" + command);
            return end == null ? 0 : Math.max(0, end - now);
        }

        public void start(UUID player, String command, long now, int seconds) {
            if (seconds > 0) until.put(player + ":" + command, now + seconds * 1000L);
            if (until.size() > 4000) until.values().removeIf(end -> end < now);
        }

        public static String waitText(long ms) { return Math.max(1, (ms + 999) / 1000) + "s"; }
    }
}
