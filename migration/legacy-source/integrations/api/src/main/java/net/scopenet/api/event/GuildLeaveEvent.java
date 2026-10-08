package net.scopenet.api.event;

import org.bukkit.event.HandlerList;
import java.util.UUID;

/** A player left (or was removed from) a guild. */
public final class GuildLeaveEvent extends ScopenetEvent {
    private static final HandlerList HANDLERS = new HandlerList();
    private final String guildId;
    private final String guildName;
    private final String guildTag;

    public GuildLeaveEvent(UUID playerId, String guildId, String guildName, String guildTag) {
        super(playerId);
        this.guildId = guildId;
        this.guildName = guildName;
        this.guildTag = guildTag;
    }

    public String getGuildId() { return guildId; }
    /** May be {@code null} if the guild no longer exists. */
    public String getGuildName() { return guildName; }
    public String getGuildTag() { return guildTag; }

    @Override public HandlerList getHandlers() { return HANDLERS; }
    public static HandlerList getHandlerList() { return HANDLERS; }
}
