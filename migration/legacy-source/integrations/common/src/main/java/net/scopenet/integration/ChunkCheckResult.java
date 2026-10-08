package net.scopenet.integration;

public record ChunkCheckResult(
        boolean claimed,
        boolean allowed,
        String guildName,
        String guildTag
) {
}
