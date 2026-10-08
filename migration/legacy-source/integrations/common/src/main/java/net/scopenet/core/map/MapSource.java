package net.scopenet.core.map;

import net.scopenet.core.Pos;
import net.scopenet.core.ShopPoints;

import java.util.Collection;
import java.util.Map;
import java.util.UUID;

/** Where a platform keeps the places the map shows. Each method returns a copy that is safe to read from any thread. */
public interface MapSource {
    Map<String, Pos> warps();
    Map<UUID, Map<String, Pos>> homes();
    /** Guild homes by guild id. */
    Map<String, Pos> guildHomes();
    Collection<ShopPoints.Shop> shops();
}
