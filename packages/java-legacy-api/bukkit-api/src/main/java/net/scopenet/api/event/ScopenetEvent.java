package net.scopenet.api.event;

import org.bukkit.event.Event;
import java.util.UUID;

/**
 * Base class for Velora events. They are fired on the server thread when the
 * panel reports a change for a player who is on this server.
 */
public abstract class ScopenetEvent extends Event {
    private final UUID playerId;

    protected ScopenetEvent(UUID playerId) {
        this.playerId = playerId;
    }

    public UUID getPlayerId() { return playerId; }
}
