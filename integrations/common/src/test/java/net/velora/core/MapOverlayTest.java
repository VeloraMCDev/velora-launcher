package net.velora.core;

import com.google.gson.JsonObject;
import net.velora.core.map.*;
import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class MapOverlayTest {
    @Test void overlayCarriesShapesAndPlainText() {
        var ring = new ChunkRegions.Ring(List.of(new ChunkRegions.Point(0, 0), new ChunkRegions.Point(16, 0), new ChunkRegions.Point(16, 16), new ChunkRegions.Point(0, 16)));
        var claim = new MapData.ClaimRegion("g:o:0", "g", "Iron <Wolves>", "IRN", "", 0x3366CC, "minecraft:overworld", ring, List.of(), 1, 8, 8,
                "<div class=\"velora-detail\"><strong>Iron &lt;Wolves&gt;</strong> <span>[IRN]</span><br>1 claimed chunk in this area</div>");
        var pin = new MapData.Pin("warp-a", "warp", "Warp: a", "minecraft:overworld", 5, 64, 6,
                "<div class=\"velora-detail\"><strong>Warp</strong> a<br><code>/warp a</code></div>");
        JsonObject json = MapOverlay.toJson(new MapData(List.of(claim), List.of(pin), List.of()));
        JsonObject c = json.getAsJsonArray("claims").get(0).getAsJsonObject();
        assertEquals("#3366cc", c.get("color").getAsString());
        assertEquals(4, c.getAsJsonArray("outer").size());
        assertEquals("Iron <Wolves> [IRN]", c.getAsJsonArray("lines").get(0).getAsString());
        assertEquals("1 claimed chunk in this area", c.getAsJsonArray("lines").get(1).getAsString());
        JsonObject p = json.getAsJsonArray("pins").get(0).getAsJsonObject();
        assertEquals("warp", p.get("kind").getAsString());
        assertEquals("/warp a", p.getAsJsonArray("lines").get(1).getAsString());
        assertFalse(json.toString().contains("<div"), "no markup reaches the viewer");
    }
}
