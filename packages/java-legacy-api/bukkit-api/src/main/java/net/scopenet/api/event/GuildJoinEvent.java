package net.scopenet.api.event;

import org.bukkit.event.HandlerList;
import java.util.UUID;

/** A player founded or joined a guild. */
public final class GuildJoinEvent extends ScopenetEvent {
    private static final HandlerList HANDLERS = new HandlerList();
    private final String guildId;
    private final String guildName;
    private final String guildTag;
    private final String role;

    public GuildJoinEvent(UUID playerId, String guildId, String guildName, String guildTag, String role) {
        super(playerId);
        this.guildId = guildId;
        this.guildName = guildName;
        this.guildTag = guildTag;
        this.role = role;
    }

    public String getGuildId() { return guildId; }
    public String getGuildName() { return guildName; }
    public String getGuildTag() { return guildTag; }
    /** {@code leader}, {@code officer} or {@code member}. */
    public String getRole() { return role; }

    @Override public HandlerList getHandlers() { return HANDLERS; }
    public static HandlerList getHandlerList() { return HANDLERS; }
}
