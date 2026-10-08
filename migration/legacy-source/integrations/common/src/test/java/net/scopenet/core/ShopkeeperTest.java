package net.scopenet.core;

import net.scopenet.core.map.MapModel;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

class ShopkeeperTest {
    @TempDir Path dir;

    @Test void shopkeepersAreRegisteredPersistedAndMarkedOnTheMap() {
        ShopPoints points = new ShopPoints(dir.resolve("shops.json"));
        Pos spot = new Pos("minecraft:overworld", 12.5, 64, -3.5, 90, 0);
        assertNull(points.addNpc("Grand_Bazaar", "market", spot, "11111111-1111-1111-1111-111111111111"));
        assertNotNull(points.addNpc("grand_bazaar", "market", spot, "22222222-2222-2222-2222-222222222222"), "names are unique");
        assertNotNull(points.addNpc("Other", "market", spot, "11111111-1111-1111-1111-111111111111"), "a mob can only keep one shop");
        assertNotNull(points.addNpc("bad name!", "market", spot, "3"), "names are checked");
        assertNotNull(points.addNpc("Smith", "weapons", spot, "3"), "kinds are checked");

        // Restarts keep them, and the mob finds its shop again.
        ShopPoints again = new ShopPoints(dir.resolve("shops.json"));
        ShopPoints.Shop shop = again.byEntity("11111111-1111-1111-1111-111111111111").orElseThrow();
        assertEquals("Grand_Bazaar", shop.name());
        assertTrue(shop.isNpc());
        assertTrue(again.at("minecraft:overworld", 12, 64, -4).isPresent());

        // The map gets a pin labelled as a shopkeeper.
        var data = MapModel.build(new MapModel.Sources(Map.of(), Map.of(), List.of(), Map.of(), Map.of(), again.all(), new com.google.gson.JsonArray(),
                List.of(), "", false, Map.of()));
        var pin = data.pins().get(0);
        assertEquals("Shopkeeper Grand Bazaar", pin.label());
        assertEquals("market", pin.kind());
        assertTrue(Math.abs(pin.x() - 12.5) < 0.001);
        assertTrue(pin.detail().contains("shopkeeper"));

        // A pushed mob drags its pin along; removing the shopkeeper frees the name.
        again.moveNpc("Grand_Bazaar", new Pos("minecraft:overworld", 20, 64, 20, 0, 0));
        assertTrue(Math.abs(again.named("grand_bazaar").orElseThrow().pos().x() - 20) < 0.001);
        assertTrue(again.removeNamed("Grand_Bazaar").isPresent());
        assertTrue(again.byEntity("11111111-1111-1111-1111-111111111111").isEmpty());
    }

    @Test void blockShopsStillWorkWithoutAnEntity() {
        ShopPoints points = new ShopPoints(dir.resolve("old.json"));
        ShopPoints.Shop block = new ShopPoints.Shop("Bazaar", "market", new Pos("minecraft:overworld", 1, 2, 3, 0, 0));
        assertFalse(block.isNpc());
    }
}
