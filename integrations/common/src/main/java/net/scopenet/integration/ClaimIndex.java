package net.scopenet.integration;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;
import java.util.UUID;

/**
 * Every claimed chunk on this server, and who may build in each, held in
 * memory. Block events ask this instead of the panel, so protection costs a
 * couple of hash lookups and keeps working if the panel is briefly away.
 * The panel is polled for a new copy only when its revision changes.
 */
public final class ClaimIndex {
    private record Guild(String id, String name, String tag, String icon, boolean admin, String description, int color, Map<String, Boolean> flags) {}

    private record Snapshot(String revision, Map<String, Map<Long, Guild>> claims, Map<String, Set<UUID>> members, int total) {}

    private static final ChunkCheckResult WILDERNESS = new ChunkCheckResult(false, true, null, null);
    private volatile Snapshot snapshot;

    static long key(int chunkX, int chunkZ) {
        return ((long) chunkX << 32) | (chunkZ & 0xffffffffL);
    }

    /**
     * Whether an admin claim allows something (build, pvp, fly, mob_spawning…). Answers {@code null} when the chunk is not in an
     * admin claim, so callers keep their normal behaviour for wilderness and guild land. A flag the panel did not send falls back
     * to the same default the panel uses.
     */
    public Boolean adminFlag(String dimension, int chunkX, int chunkZ, String flag) {
        Snapshot s = snapshot;
        if (s == null) return null;
        Map<Long, Guild> dim = s.claims().get(dimension);
        Guild g = dim == null ? null : dim.get(key(chunkX, chunkZ));
        if (g == null || !g.admin()) return null;
        Boolean set = g.flags().get(flag);
        return set != null ? set : DEFAULT_FLAGS.getOrDefault(flag, true);
    }

    /**
     * Like {@link #adminFlag}, but also answers for guild claims, whose leaders choose the flags (see the panel's guild land rules).
     * A rule the panel did not send takes the same default: guild land behaves as it always did until a guild changes something.
     * Answers {@code null} for wilderness.
     */
    public Boolean claimFlag(String dimension, int chunkX, int chunkZ, String flag) {
        Snapshot s = snapshot;
        if (s == null) return null;
        Map<Long, Guild> dim = s.claims().get(dimension);
        Guild g = dim == null ? null : dim.get(key(chunkX, chunkZ));
        if (g == null) return null;
        Boolean set = g.flags().get(flag);
        return set != null ? set : DEFAULT_FLAGS.getOrDefault(flag, true);
    }

    /**
     * May {@code player} change land here? True in wilderness and for members of the claiming guild; visitors only where the claim's
     * rule for {@code flag} (build, interact, containers) is on. Admin claims have no members, so for them it is the flag alone.
     */
    public boolean mayModify(String dimension, int chunkX, int chunkZ, UUID player, String flag) {
        ChunkCheckResult here = check(dimension, chunkX, chunkZ, player);
        if (!here.claimed() || here.allowed()) return true;
        Boolean open = claimFlag(dimension, chunkX, chunkZ, flag);
        return open != null && open;
    }

    /** What each flag means when an admin claim says nothing (mirrors the panel): build, interact, containers, griefing and fire are off. */
    static final Map<String, Boolean> DEFAULT_FLAGS = Map.ofEntries(
            Map.entry("build", false), Map.entry("interact", false), Map.entry("containers", false), Map.entry("entry", true),
            Map.entry("pvp", true), Map.entry("player_damage", true), Map.entry("animal_damage", true), Map.entry("mob_spawning", true),
            Map.entry("mob_griefing", false), Map.entry("explosions", false), Map.entry("fire_spread", false), Map.entry("fluid_flow", false),
            Map.entry("fly", true), Map.entry("ender_pearls", true), Map.entry("hunger", true), Map.entry("item_drop", true));

    /** False until the first copy arrives from the panel. */
    public boolean loaded() { return snapshot != null; }

    public String revision() {
        Snapshot s = snapshot;
        return s == null ? null : s.revision();
    }

    public int claimCount() {
        Snapshot s = snapshot;
        return s == null ? 0 : s.total();
    }

    /** Is this chunk claimed by any guild? Used for environmental changes (fire, pistons, explosions). */
    public boolean isClaimed(String dimension, int chunkX, int chunkZ) {
        Snapshot s = snapshot;
        if (s == null) return false;
        Map<Long, Guild> dim = s.claims().get(dimension);
        return dim != null && dim.containsKey(key(chunkX, chunkZ));
    }

    /** May {@code player} build here? Unclaimed land always answers yes. */
    public ChunkCheckResult check(String dimension, int chunkX, int chunkZ, UUID player) {
        Snapshot s = snapshot;
        if (s == null) return WILDERNESS;
        Map<Long, Guild> dim = s.claims().get(dimension);
        Guild guild = dim == null ? null : dim.get(key(chunkX, chunkZ));
        if (guild == null) return WILDERNESS;
        Set<UUID> members = s.members().get(guild.id());
        return new ChunkCheckResult(true, members != null && members.contains(player), guild.name(), guild.tag());
    }

    /**
     * One owner's claimed chunks in one dimension, for drawing on a map. Admin claims are owners too: {@code admin} is
     * set, {@code color} is their chosen colour ({@code -1} for guilds, which get one derived from their id) and
     * {@code description} is the text the admin wrote.
     */
    public record GuildClaims(String id, String name, String tag, String icon, Set<Long> chunks, boolean admin, String description, int color) {
        public GuildClaims(String id, String name, String tag, String icon, Set<Long> chunks) {
            this(id, name, tag, icon, chunks, false, "", -1);
        }
    }

    /** What stands on a chunk, for the "you are entering…" banner. */
    public record ClaimInfo(String id, String name, String tag, String description, boolean admin) {}

    /** The owner of a chunk, or null for wilderness. */
    public ClaimInfo info(String dimension, int chunkX, int chunkZ) {
        Snapshot s = snapshot;
        if (s == null) return null;
        Map<Long, Guild> dim = s.claims().get(dimension);
        Guild g = dim == null ? null : dim.get(key(chunkX, chunkZ));
        return g == null ? null : new ClaimInfo(g.id(), g.name(), g.tag(), g.description(), g.admin());
    }

    public static int chunkX(long key) { return (int) (key >> 32); }
    public static int chunkZ(long key) { return (int) key; }

    /** Every claim grouped by dimension and guild (a copy; safe to use from any thread). */
    public Map<String, java.util.List<GuildClaims>> byDimension() {
        Snapshot s = snapshot;
        Map<String, java.util.List<GuildClaims>> out = new HashMap<>();
        if (s == null) return out;
        s.claims().forEach((dimension, chunks) -> {
            Map<String, GuildClaims> perGuild = new java.util.LinkedHashMap<>();
            chunks.forEach((key, guild) -> perGuild.computeIfAbsent(guild.id(),
                    id -> new GuildClaims(guild.id(), guild.name(), guild.tag(), guild.icon(), new HashSet<>(), guild.admin(), guild.description(), guild.color())).chunks().add(key));
            out.put(dimension, new java.util.ArrayList<>(perGuild.values()));
        });
        return out;
    }

    private static Map<String, Boolean> flagsOf(JsonObject g) {
        Map<String, Boolean> flags = new HashMap<>();
        if (g.has("flags") && g.get("flags").isJsonObject())
            g.getAsJsonObject("flags").entrySet().forEach(e -> { if (e.getValue().isJsonPrimitive()) flags.put(e.getKey(), e.getValue().getAsBoolean()); });
        return flags;
    }

    private static int parseColor(String hex) {
        if (hex == null || !hex.matches("#[0-9a-fA-F]{6}")) return -1;
        return Integer.parseInt(hex.substring(1), 16);
    }

    /** Applies a {@code guilds/claim-index} response. Returns true when the contents changed. */
    public boolean apply(JsonObject response) {
        if (response.has("unchanged") && response.get("unchanged").getAsBoolean()) return false;
        String revision = response.has("revision") ? response.get("revision").getAsString() : "";
        JsonArray guildList = response.getAsJsonArray("guilds");
        Guild[] guilds = new Guild[guildList.size()];
        for (int i = 0; i < guilds.length; i++) {
            JsonObject g = guildList.get(i).getAsJsonObject();
            guilds[i] = new Guild(g.get("id").getAsString(), g.get("name").getAsString(), g.get("tag").getAsString(),
                    g.has("icon_url") && g.get("icon_url").isJsonPrimitive() ? g.get("icon_url").getAsString() : "",
                    g.has("admin") && g.get("admin").isJsonPrimitive() && g.get("admin").getAsBoolean(),
                    g.has("description") && g.get("description").isJsonPrimitive() ? g.get("description").getAsString() : "",
                    parseColor(g.has("color") && g.get("color").isJsonPrimitive() ? g.get("color").getAsString() : null),
                    flagsOf(g));
        }
        Map<String, Map<Long, Guild>> claims = new HashMap<>();
        JsonArray claimList = response.getAsJsonArray("claims");
        for (JsonElement element : claimList) {
            JsonArray c = element.getAsJsonArray();
            int guild = c.get(3).getAsInt();
            if (guild < 0 || guild >= guilds.length) continue;
            claims.computeIfAbsent(c.get(0).getAsString(), d -> new HashMap<>()).put(key(c.get(1).getAsInt(), c.get(2).getAsInt()), guilds[guild]);
        }
        Map<String, Set<UUID>> members = new HashMap<>();
        for (Map.Entry<String, JsonElement> entry : response.getAsJsonObject("members").entrySet()) {
            Set<UUID> set = new HashSet<>();
            for (JsonElement uuid : entry.getValue().getAsJsonArray()) {
                try { set.add(UUID.fromString(uuid.getAsString())); } catch (IllegalArgumentException ignored) { /* skip malformed */ }
            }
            members.put(entry.getKey(), set);
        }
        snapshot = new Snapshot(revision, claims, members, claimList.size());
        return true;
    }
}
