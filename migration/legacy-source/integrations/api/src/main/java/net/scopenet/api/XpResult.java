package net.scopenet.api;

/** What a player's XP and level are after {@link ScopenetApi#addXP}. */
public record XpResult(XpScope scope, long xp, int level, int previousLevel) {
    public boolean leveledUp() { return level > previousLevel; }
}
