package net.velora.core.map;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.velora.core.*;
import net.scopenet.integration.ClaimIndex;

import java.util.*;

/**
 * Gathers everything the map needs, from any thread. The map overlay asks for a {@link MapData} every
 * few seconds; the market listings are refreshed here on a slower clock so the map never waits on the panel.
 */
public final class MapService {
    public static final long MARKET_REFRESH_MS = 60_000;

    private final Env env;
    private final ClaimIndex claims;
    private final MapSource places;
    private final PlayerCache cache;
    private final String panelBase;
    private final boolean showHomes;
    private final String serverName;
    private volatile JsonArray market = new JsonArray();
    private volatile long marketAt = Long.MIN_VALUE / 2;
    private volatile boolean fetching;

    public MapService(Env env, ClaimIndex claims, MapSource places, PlayerCache cache, String panelBase, boolean showHomes, String serverName) {
        this.env = env; this.claims = claims; this.places = places; this.cache = cache; this.panelBase = panelBase; this.showHomes = showHomes; this.serverName = serverName;
    }

    public String serverName() { return serverName; }

    /** Server thread, about once a second. */
    public void tick() {
        long now = env.clock.getAsLong();
        if (fetching || now - marketAt < MARKET_REFRESH_MS || !env.features.economy()) return;
        fetching = true;
        marketAt = now;
        env.io(() -> env.panel.call("economy/market", new JsonObject()), el -> {
            market = el.isJsonArray() ? el.getAsJsonArray() : new JsonArray();
            fetching = false;
        }, e -> fetching = false);
    }

    /** Safe from any thread: reads only copies and concurrent collections. */
    public MapData snapshot() {
        List<MapData.OnlinePlayer> players = onlinePlayers();
        Map<UUID, String> names = new HashMap<>();
        for (CorePlayer p : env.platform.online()) names.put(p.uuid(), p.name());
        Map<String, MapModel.NamedPos> guildHomes = new TreeMap<>();
        Map<String, String> labels = new HashMap<>();
        claims.byDimension().values().forEach(list -> list.forEach(g -> labels.put(g.id(), "[" + g.tag() + "] " + g.name())));
        places.guildHomes().forEach((id, pos) -> guildHomes.put(id, new MapModel.NamedPos(labels.getOrDefault(id, "Faction home"), pos)));
        Pos spawn = env.platform.worldSpawn("minecraft:overworld");
        MapModel.Sources sources = new MapModel.Sources(
                env.features.guilds() && env.features.landClaiming() ? claims.byDimension() : Map.of(),
                env.features.essentials() ? places.warps() : Map.of(),
                List.of(spawn), showHomes ? places.homes() : Map.of(), guildHomes, places.shops(),
                market, players, panelBase, showHomes, names);
        return MapModel.build(sources);
    }

    private List<MapData.OnlinePlayer> onlinePlayers() {
        List<MapData.OnlinePlayer> out = new ArrayList<>();
        for (CorePlayer p : env.platform.online()) {
            JsonObject i = cache.info(p.uuid());
            JsonObject global = PlayerCache.obj(i, "global"), server = PlayerCache.obj(i, "server"), guild = PlayerCache.obj(i, "guild"), rank = PlayerCache.obj(i, "rank");
            out.add(new MapData.OnlinePlayer(p.uuid().toString(), p.name(), (long) PlayerCache.num(global, "level", 1), PlayerCache.str(global, "title", ""),
                    (long) PlayerCache.num(server, "level", 1), PlayerCache.str(guild, "name", ""), PlayerCache.str(guild, "tag", ""), PlayerCache.str(guild, "role", ""),
                    PlayerCache.str(rank, "display", PlayerCache.str(rank, "group", "")), (long) PlayerCache.num(i, "playtime_secs", 0)));
        }
        return out;
    }

    public JsonObject playersJson() { return MapModel.playersJson(serverName, onlinePlayers(), System.currentTimeMillis()); }
}
