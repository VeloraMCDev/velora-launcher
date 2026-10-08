package net.scopenet.core;

import com.google.gson.JsonObject;
import net.scopenet.integration.ChunkCheckResult;

import java.util.UUID;

/** The panel calls the shared logic makes. {@link PanelAdapter} implements it over HTTP; tests use a fake. */
public interface Panel {
    /** Claim names from the sync cache, for completion without network calls. */
    default java.util.List<String> claimNames(boolean admin) { return java.util.List.of(); }
    /** POST to {@code /api/server/v1/<endpoint>} and return the JSON answer. Throws with the panel's message on errors. */
    com.google.gson.JsonElement call(String endpoint, JsonObject body) throws Exception;

    /** Answered from the local claim index; never blocks on the network. */
    /** Tell the claim index to re-sync soon (after claiming, unclaiming or founding a guild). */
    default void claimsChanged() {}

    ChunkCheckResult check(String dimension, int chunkX, int chunkZ, UUID player);

    /** Who owns this chunk (guild or admin claim), from the local claim index; null for wilderness. */
    /** Does an admin claim here allow this flag (pvp, fly, mob_spawning…)? True anywhere that is not an admin claim. */
    default boolean adminAllows(String dimension, int chunkX, int chunkZ, String flag) { return true; }

    default net.scopenet.integration.ClaimIndex.ClaimInfo info(String dimension, int chunkX, int chunkZ) { return null; }
}
