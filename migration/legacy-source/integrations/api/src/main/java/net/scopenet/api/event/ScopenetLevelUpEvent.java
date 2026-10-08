package net.scopenet.api.event;

import net.scopenet.api.XpScope;
import org.bukkit.event.HandlerList;
import java.util.UUID;

/** A player's global or server level went up. */
public final class ScopenetLevelUpEvent extends ScopenetEvent {
    private static final HandlerList HANDLERS = new HandlerList();
    private final XpScope scope;
    private final int level;
    private final int previousLevel;
    private final long xp;

    public ScopenetLevelUpEvent(UUID playerId, XpScope scope, int level, int previousLevel, long xp) {
        super(playerId);
        this.scope = scope;
        this.level = level;
        this.previousLevel = previousLevel;
        this.xp = xp;
    }

    public XpScope getScope() { return scope; }
    public int getLevel() { return level; }
    public int getPreviousLevel() { return previousLevel; }
    public long getXp() { return xp; }

    @Override public HandlerList getHandlers() { return HANDLERS; }
    public static HandlerList getHandlerList() { return HANDLERS; }
}
