package net.scopenet.core.map;

import java.util.List;

/** What the map shows, independent of any map software. Built by {@link MapModel}. */
public record MapData(List<ClaimRegion> claims, List<Pin> pins, List<OnlinePlayer> players) {
    public static MapData empty() { return new MapData(List.of(), List.of(), List.of()); }

    /** One connected area of a guild's land. {@code color} is 0xRRGGBB. */
    public record ClaimRegion(String id, String guildId, String name, String tag, String icon, int color, String dimension,
                              ChunkRegions.Ring outer, List<ChunkRegions.Ring> holes, int chunks, double labelX, double labelZ, String detail) {}

    /** kind: spawn, warp, home, guild_home, market or shop. {@code detail} is HTML that is already escaped. */
    public record Pin(String id, String kind, String label, String dimension, double x, double y, double z, String detail) {}

    public record OnlinePlayer(String uuid, String name, long level, String title, long serverLevel, String guild, String guildTag,
                               String guildRole, String rank, long playtimeSecs) {}
}
