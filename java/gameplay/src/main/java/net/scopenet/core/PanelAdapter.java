package net.scopenet.core;

import com.google.gson.JsonObject;
import net.scopenet.integration.ChunkCheckResult;
import net.scopenet.integration.Integration;

import java.util.UUID;

public final class PanelAdapter implements Panel {
    private final Integration integration;
    public PanelAdapter(Integration integration) { this.integration = integration; }
    @Override public java.util.List<String> claimNames(boolean admin) {
        return integration.client().claims().byDimension().values().stream().flatMap(java.util.Collection::stream)
                .filter(c -> c.admin() == admin).map(net.scopenet.integration.ClaimIndex.GuildClaims::name).distinct().sorted().toList();
    }

    @Override public com.google.gson.JsonElement call(String endpoint, JsonObject body) throws Exception {
        return integration.client().postElement(endpoint, body);
    }

    @Override public void claimsChanged() { integration.client().requestClaimRefresh(); }

    @Override public net.scopenet.integration.ClaimIndex.ClaimInfo info(String dimension, int chunkX, int chunkZ) {
        return integration.client().claims().info(dimension, chunkX, chunkZ);
    }

    @Override public boolean adminAllows(String dimension, int chunkX, int chunkZ, String flag) {
        return integration.adminAllows(dimension, chunkX, chunkZ, flag);
    }

    @Override public ChunkCheckResult check(String dimension, int chunkX, int chunkZ, UUID player) {
        return integration.checkChunk(dimension, chunkX, chunkZ, player);
    }
}
