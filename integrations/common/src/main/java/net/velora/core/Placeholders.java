package net.velora.core;

import com.google.gson.JsonObject;

import java.util.List;
import java.util.Locale;
import java.util.Optional;
import java.util.UUID;

/**
 * %velora_*% values, the same names and formats as the Paper PlaceholderAPI expansion. On Fabric they appear as
 * %velora:level% (Placeholder API). Reads the {@link PlayerCache}, so it never blocks.
 */
public final class Placeholders {
    public static final List<String> KEYS = List.of(
            "level", "xp", "level_xp", "next_level_xp", "level_progress", "level_bar", "title", "rank_title", "leaderboard_rank",
            "server_level", "server_xp", "server_rank", "guild", "guild_tag", "guild_role", "guild_claims", "has_guild",
            "balance", "balance_formatted", "playtime", "playtime_seconds", "playtime_hours", "server_playtime",
            "quest_progress", "quest_daily", "quest_weekly", "quest_daily_done", "quest_daily_total", "quest_weekly_done", "quest_weekly_total",
            "kills", "deaths", "kdr", "friends", "achievements", "joined", "rank", "rank_prefix");

    private final PlayerCache cache;
    private final Env env;

    public Placeholders(Env env, PlayerCache cache) { this.env = env; this.cache = cache; }

    /** Empty means "not one of ours". Before the first panel answer, sensible zero values are returned. */
    public Optional<String> resolve(UUID player, String key) {
        String k = key.toLowerCase(Locale.ROOT);
        if (!KEYS.contains(k)) return Optional.empty();
        JsonObject i = cache.info(player);
        JsonObject global = PlayerCache.obj(i, "global"), server = PlayerCache.obj(i, "server"), guild = PlayerCache.obj(i, "guild");
        JsonObject quests = PlayerCache.obj(i, "quests"), daily = PlayerCache.obj(quests, "daily"), weekly = PlayerCache.obj(quests, "weekly");
        JsonObject rank = PlayerCache.obj(i, "rank");
        double balance = PlayerCache.num(i, "balance", 0);
        long playtime = (long) PlayerCache.num(i, "playtime_secs", 0);
        long kills = (long) PlayerCache.num(i, "kills", 0), deaths = (long) PlayerCache.num(i, "deaths", 0);
        int dDone = (int) PlayerCache.num(daily, "completed", 0), dTotal = (int) PlayerCache.num(daily, "total", 0);
        int wDone = (int) PlayerCache.num(weekly, "completed", 0), wTotal = (int) PlayerCache.num(weekly, "total", 0);
        String symbol = env.features.currencySymbol();
        String out = switch (k) {
            case "level" -> String.valueOf((long) PlayerCache.num(global, "level", 1));
            case "xp" -> String.valueOf((long) PlayerCache.num(global, "xp", 0));
            case "level_xp" -> String.valueOf((long) PlayerCache.num(global, "current_level_xp", 0));
            case "next_level_xp" -> String.valueOf((long) PlayerCache.num(global, "next_level_xp", 0));
            case "level_progress" -> String.valueOf(Math.round(PlayerCache.num(global, "progress_pct", 0)));
            case "level_bar" -> bar(PlayerCache.num(global, "progress_pct", 0));
            case "title" -> PlayerCache.str(global, "title", "");
            case "rank_title" -> { String g = PlayerCache.str(global, "title_glyph", ""); yield g.isEmpty() ? PlayerCache.str(global, "title", "") : g; }
            case "leaderboard_rank" -> { long r = (long) PlayerCache.num(global, "rank", 0); yield r <= 0 ? "-" : String.valueOf(r); }
            case "server_level" -> String.valueOf((long) PlayerCache.num(server, "level", 1));
            case "server_xp" -> String.valueOf((long) PlayerCache.num(server, "xp", 0));
            case "server_rank" -> PlayerCache.str(server, "rank_name", "");
            case "guild" -> PlayerCache.str(guild, "name", "");
            case "guild_tag" -> PlayerCache.str(guild, "tag", "");
            case "guild_role" -> PlayerCache.str(guild, "role", "");
            case "guild_claims" -> String.valueOf((long) PlayerCache.num(guild, "claims", 0));
            case "has_guild" -> String.valueOf(guild != null);
            case "balance" -> String.format(Locale.US, "%.2f", balance);
            case "balance_formatted" -> Format.money(symbol, balance);
            case "playtime" -> duration(playtime);
            case "playtime_seconds" -> String.valueOf(playtime);
            case "playtime_hours" -> String.valueOf(playtime / 3600);
            case "server_playtime" -> duration((long) PlayerCache.num(i, "server_playtime_secs", 0));
            case "quest_progress" -> (dDone + wDone) + "/" + (dTotal + wTotal);
            case "quest_daily" -> dDone + "/" + dTotal;
            case "quest_weekly" -> wDone + "/" + wTotal;
            case "quest_daily_done" -> String.valueOf(dDone);
            case "quest_daily_total" -> String.valueOf(dTotal);
            case "quest_weekly_done" -> String.valueOf(wDone);
            case "quest_weekly_total" -> String.valueOf(wTotal);
            case "kills" -> String.valueOf(kills);
            case "deaths" -> String.valueOf(deaths);
            case "kdr" -> String.format(Locale.US, "%.2f", deaths == 0 ? (double) kills : (double) kills / deaths);
            case "friends" -> String.valueOf((long) PlayerCache.num(i, "friends", 0));
            case "achievements" -> String.valueOf((long) PlayerCache.num(i, "achievements", 0));
            case "joined" -> { String j = PlayerCache.str(i, "joined", ""); yield j.length() >= 10 ? j.substring(0, 10) : j; }
            case "rank" -> PlayerCache.str(rank, "group", "");
            case "rank_prefix" -> PlayerCache.str(rank, "prefix", "");
            default -> "";
        };
        return Optional.of(out);
    }

    static String bar(double percent) {
        int filled = (int) Math.max(0, Math.min(10, Math.round(percent / 10.0)));
        return "§a" + "■".repeat(filled) + "§7" + "■".repeat(10 - filled);
    }

    static String duration(long seconds) {
        long d = seconds / 86400, h = seconds % 86400 / 3600, m = seconds % 3600 / 60;
        if (d > 0) return d + "d " + h + "h";
        if (h > 0) return h + "h " + m + "m";
        return m + "m";
    }
}
