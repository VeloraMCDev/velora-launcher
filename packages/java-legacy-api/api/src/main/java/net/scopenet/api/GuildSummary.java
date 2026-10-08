package net.scopenet.api;

/** A player's guild, as seen from the server they are on. */
public record GuildSummary(String id, String name, String tag, String role, int claims) {
    public boolean isLeader() { return "leader".equalsIgnoreCase(role); }
    public boolean isOfficer() { return isLeader() || "officer".equalsIgnoreCase(role); }
}
