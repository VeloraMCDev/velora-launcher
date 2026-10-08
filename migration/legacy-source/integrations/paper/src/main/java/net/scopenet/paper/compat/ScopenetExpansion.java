package net.scopenet.paper.compat;

import me.clip.placeholderapi.expansion.PlaceholderExpansion;
import net.scopenet.api.GuildSummary;
import net.scopenet.api.QuestProgress;
import net.scopenet.api.ScopenetPlayer;
import org.bukkit.OfflinePlayer;
import java.util.List;
import java.util.Locale;
import java.util.Optional;

/**
 * {@code %scopenet_<placeholder>%} for PlaceholderAPI: scoreboards, TAB, holograms, chat and menus.
 * Values come from the API's cache, so a placeholder never waits on the network; the first request
 * for a player answers with a neutral value while the data loads.
 */
public final class ScopenetExpansion extends PlaceholderExpansion {
    /** Every placeholder, for docs and /scopenet placeholders. */
    public static final List<String> PLACEHOLDERS = List.of(
            "level", "xp", "level_xp", "next_level_xp", "level_progress", "level_bar", "title", "rank_title", "leaderboard_rank",
            "server_level", "server_xp", "server_rank",
            "guild", "guild_tag", "guild_role", "guild_claims", "has_guild",
            "balance", "balance_formatted",
            "playtime", "playtime_seconds", "playtime_hours", "server_playtime",
            "quest_progress", "quest_daily", "quest_weekly", "quest_daily_done", "quest_daily_total", "quest_weekly_done", "quest_weekly_total",
            "kills", "deaths", "kdr", "friends", "achievements", "joined", "rank", "rank_prefix");

    private final ScopenetApiImpl api;
    private final String symbol;
    private final String version;

    public ScopenetExpansion(ScopenetApiImpl api, String currencySymbol, String version) {
        this.api = api;
        this.symbol = currencySymbol;
        this.version = version;
    }

    @Override public String getIdentifier() { return "scopenet"; }
    @Override public String getAuthor() { return "SCOPENET"; }
    @Override public String getVersion() { return version; }
    @Override public boolean persist() { return true; }

    @Override public String onRequest(OfflinePlayer player, String params) {
        if (player == null) return "";
        Optional<ScopenetPlayer> found = api.peek(player.getUniqueId());
        String key = params.toLowerCase(Locale.ROOT);
        if (found.isEmpty()) return neutral(key);
        ScopenetPlayer p = found.get();
        GuildSummary guild = p.guild();
        return switch (key) {
            case "level" -> String.valueOf(p.globalLevel());
            case "xp" -> String.valueOf(p.globalXp());
            case "level_xp" -> String.valueOf(p.globalLevelXp());
            case "next_level_xp" -> String.valueOf(p.globalNextLevelXp());
            case "level_progress" -> String.valueOf(Math.round(p.globalProgress()));
            case "level_bar" -> bar(p.globalProgress());
            case "title" -> p.rankTitle() == null ? "" : p.rankTitle();
            case "rank_title" -> p.rankGlyph() != null && !p.rankGlyph().isEmpty() ? p.rankGlyph() : p.rankTitle() == null ? "" : p.rankTitle();
            case "leaderboard_rank" -> p.leaderboard() <= 0 ? "-" : String.valueOf(p.leaderboard());
            case "server_level" -> String.valueOf(p.serverLevel());
            case "server_xp" -> String.valueOf(p.serverXp());
            case "server_rank" -> p.serverRank() == null ? "" : p.serverRank();
            case "guild" -> guild == null ? "" : guild.name();
            case "guild_tag" -> guild == null ? "" : guild.tag();
            case "guild_role" -> guild == null ? "" : guild.role();
            case "guild_claims" -> guild == null ? "0" : String.valueOf(guild.claims());
            case "has_guild" -> String.valueOf(guild != null);
            case "balance" -> String.format(Locale.US, "%.2f", p.balance());
            case "balance_formatted" -> symbol + String.format(Locale.US, "%,.2f", p.balance());
            case "playtime" -> duration(p.playtimeSeconds());
            case "playtime_seconds" -> String.valueOf(p.playtimeSeconds());
            case "playtime_hours" -> String.valueOf(p.playtimeSeconds() / 3600);
            case "server_playtime" -> duration(p.serverPlaytimeSeconds());
            case "quest_progress" -> combined(p.dailyQuests(), p.weeklyQuests());
            case "quest_daily" -> p.dailyQuests().display();
            case "quest_weekly" -> p.weeklyQuests().display();
            case "quest_daily_done" -> String.valueOf(p.dailyQuests().completed());
            case "quest_daily_total" -> String.valueOf(p.dailyQuests().total());
            case "quest_weekly_done" -> String.valueOf(p.weeklyQuests().completed());
            case "quest_weekly_total" -> String.valueOf(p.weeklyQuests().total());
            case "kills" -> String.valueOf(p.kills());
            case "deaths" -> String.valueOf(p.deaths());
            case "kdr" -> String.format(Locale.US, "%.2f", p.killDeathRatio());
            case "friends" -> String.valueOf(p.friends());
            case "achievements" -> String.valueOf(p.achievements());
            case "joined" -> p.joined() == null ? "" : p.joined().length() >= 10 ? p.joined().substring(0, 10) : p.joined();
            case "rank" -> p.rankGroup() == null ? "" : p.rankGroup();
            case "rank_prefix" -> p.rankPrefix() == null ? "" : p.rankPrefix();
            default -> null; // unknown placeholder: PlaceholderAPI leaves it as written
        };
    }

    /** What to show while a player's data is still loading. */
    private String neutral(String key) {
        return switch (key) {
            case "level", "server_level" -> "1";
            case "xp", "level_xp", "next_level_xp", "level_progress", "server_xp", "guild_claims", "kills", "deaths", "friends", "achievements", "playtime_seconds", "playtime_hours",
                 "quest_daily_done", "quest_daily_total", "quest_weekly_done", "quest_weekly_total" -> "0";
            case "balance" -> "0.00";
            case "balance_formatted" -> symbol + "0.00";
            case "kdr" -> "0.00";
            case "has_guild" -> "false";
            case "quest_progress", "quest_daily", "quest_weekly" -> "0/0";
            case "level_bar" -> bar(0);
            case "playtime", "server_playtime" -> "0m";
            case "leaderboard_rank" -> "-";
            default -> "";
        };
    }

    private static String combined(QuestProgress daily, QuestProgress weekly) {
        return (daily.completed() + weekly.completed()) + "/" + (daily.total() + weekly.total());
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
