package net.velora.paper.panel;

import java.util.Collections;
import java.util.List;
import java.util.UUID;

public record SyncResult(
        boolean ok,
        List<UUID> kickPlayers,
        String errorMessage
) {
    public static SyncResult success(List<UUID> kickPlayers) {
        return new SyncResult(true, kickPlayers != null ? kickPlayers : Collections.emptyList(), null);
    }

    public static SyncResult failure(String errorMessage) {
        return new SyncResult(false, Collections.emptyList(), errorMessage);
    }
}
