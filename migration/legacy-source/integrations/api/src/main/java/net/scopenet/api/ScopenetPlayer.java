package net.scopenet.api;

import java.util.UUID;

/**
 * A snapshot of one SCOPENET account. It is a copy: fetch a new one (or use the
 * cached copy, which is refreshed every few seconds for online players) to see changes.
 *
 * @param rankTitle   the title earned from global level rewards, or {@code null}
 * @param rankGlyph   the private-use character the resource pack draws as the rank's PNG title, or {@code null}/empty
 * @param leaderboard the position on the global leaderboard, or 0 when unranked
 * @param rankGroup   the player's LuckPerms primary group as reported by the server, or {@code null}
 * @param guild       the player's guild on this server's instance, or {@code null}
 */
public record ScopenetPlayer(
        UUID uuid,
        String name,
        int globalLevel,
        long globalXp,
        long globalLevelXp,
        long globalNextLevelXp,
        double globalProgress,
        String rankTitle,
        int leaderboard,
        int serverLevel,
        long serverXp,
        String serverRank,
        GuildSummary guild,
        double balance,
        long playtimeSeconds,
        long serverPlaytimeSeconds,
        int kills,
        int deaths,
        int friends,
        int achievements,
        QuestProgress dailyQuests,
        QuestProgress weeklyQuests,
        String joined,
        String rankGroup,
        String rankDisplay,
        String rankPrefix,
        String rankSuffix,
        String rankGlyph) {

    public boolean inGuild() { return guild != null; }

    public double killDeathRatio() { return deaths == 0 ? kills : (double) kills / deaths; }
}
