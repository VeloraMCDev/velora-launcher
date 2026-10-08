package net.scopenet.api.event;

import org.bukkit.event.HandlerList;
import java.util.UUID;

/** A player unlocked a SCOPENET achievement. */
public final class AchievementUnlockEvent extends ScopenetEvent {
    private static final HandlerList HANDLERS = new HandlerList();
    private final String achievementId;
    private final String title;
    private final long xp;

    public AchievementUnlockEvent(UUID playerId, String achievementId, String title, long xp) {
        super(playerId);
        this.achievementId = achievementId;
        this.title = title;
        this.xp = xp;
    }

    public String getAchievementId() { return achievementId; }
    public String getTitle() { return title; }
    /** XP the achievement was worth. */
    public long getXp() { return xp; }

    @Override public HandlerList getHandlers() { return HANDLERS; }
    public static HandlerList getHandlerList() { return HANDLERS; }
}
