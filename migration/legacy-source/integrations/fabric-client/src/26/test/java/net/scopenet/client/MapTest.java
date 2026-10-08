package net.scopenet.client;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

/** The in-game map must agree with the panel's engine, so its maths is checked against the same numbers. */
class MapTest {
    static final String DATA = "{\"info\":{\"ready\":true,\"token\":\"abc\",\"server_id\":3,\"max_zoom\":6,\"dimensions\":["
            + "{\"id\":\"minecraft:overworld\",\"slug\":\"overworld\",\"available\":true,\"tiles\":40,\"bounds\":{\"min_x\":-4,\"max_x\":3,\"min_y\":-4,\"max_y\":3},\"core\":{\"min_x\":-2,\"max_x\":1,\"min_y\":-2,\"max_y\":1},\"levels\":[40,10,3,1,0,0,0]},"
            + "{\"id\":\"minecraft:the_nether\",\"slug\":\"the_nether\",\"available\":true,\"tiles\":2,\"bounds\":{\"min_x\":0,\"max_x\":1,\"min_y\":0,\"max_y\":0}}]},"
            + "\"claims\":[{\"id\":\"c1\",\"name\":\"Hold\",\"tag\":\"HLD\",\"color\":\"#ff8800\",\"dimension\":\"minecraft:overworld\",\"label_x\":8,\"label_z\":8,"
            + "\"outer\":[[0,0],[32,0],[32,32],[0,32]],\"holes\":[[[12,12],[20,12],[20,20],[12,20]]],\"lines\":[\"&6Owner: Alex\"]}],"
            + "\"pins\":[{\"id\":\"p1\",\"kind\":\"spawn\",\"label\":\"Spawn\",\"dimension\":\"minecraft:overworld\",\"x\":1,\"y\":64,\"z\":2,\"lines\":[]}],"
            + "\"players\":[{\"uuid\":\"u1\",\"name\":\"Alex\",\"dimension\":\"minecraft:the_nether\",\"x\":5,\"y\":70,\"z\":6,\"yaw\":90}]}";

    @Test void dimensionSlugsMatchThePanel() {
        assertEquals("the_nether", MapScene.slug("minecraft:the_nether"));
        assertEquals("the_nether", MapScene.slug("nether"));
        assertEquals("the_end", MapScene.slug("end"));
        assertEquals("mod__dim", MapScene.slug("mod:dim"));
        assertEquals("overworld", MapScene.slug("minecraft:overworld"));
    }

    @Test void parsesTheOverlay() {
        MapScene s = MapScene.parse(JsonParser.parseString(DATA).getAsJsonObject());
        assertTrue(s.ready);
        assertEquals(2, s.dimensions.size());
        assertArrayEquals(new int[]{-2, 1, -2, 1}, s.dimension("overworld").core());
        assertArrayEquals(new int[]{0, 1, 0, 0}, s.dimension("the_nether").core()); // too few tiles for a "core": the full bounds
        assertEquals(1, s.claims.size());
        assertEquals(0xff8800, s.claims.get(0).color());
        assertEquals("[HLD] Hold", s.claims.get(0).title());
        assertEquals("Owner: Alex", s.claims.get(0).lines().get(0));
        assertEquals("the_nether", s.players.get(0).slug());
    }

    @Test void claimsAreEvenOddWithHoles() {
        MapScene.Claim c = MapScene.parse(JsonParser.parseString(DATA).getAsJsonObject()).claims.get(0);
        assertTrue(MapScene.contains(c, 4, 4));
        assertFalse(MapScene.contains(c, 16, 16));   // inside the hole
        assertFalse(MapScene.contains(c, 40, 4));
        double[] across = MapScene.spans(c, 16);      // a line through the hole: two separate ranges
        assertArrayEquals(new double[]{0, 12, 20, 32}, across);
        assertArrayEquals(new double[]{0, 32}, MapScene.spans(c, 4));
    }

    @Test void cameraZoomAndScreenMaths() {
        MapCamera cam = new MapCamera();
        cam.resize(400, 300); cam.cx = 100; cam.cz = 50; cam.scale = 2;
        assertEquals(200, cam.sx(100), 1e-9);
        assertEquals(150, cam.sy(50), 1e-9);
        assertEquals(100, cam.wx(200), 1e-9);
        double wx = cam.wx(300), wz = cam.wz(75);
        cam.zoomAround(300, 75, 2, false, 0);          // the point under the cursor does not move
        assertEquals(4, cam.scale, 1e-9);
        assertEquals(300, cam.sx(wx), 1e-9);
        assertEquals(75, cam.sy(wz), 1e-9);
        cam.zoomAround(0, 0, 1e9, false, 0);
        assertEquals(MapCamera.MAX_SCALE, cam.scale);
        cam.zoomAround(0, 0, 1e-9, false, 0);
        assertEquals(MapCamera.MIN_SCALE, cam.scale);
    }

    @Test void tileLevelsFollowTheScale() {
        MapCamera cam = new MapCamera();
        cam.scale = 1; assertEquals(0, cam.zoomLevel(6));
        cam.scale = 0.5; assertEquals(1, cam.zoomLevel(6));
        cam.scale = 0.2; assertEquals(2, cam.zoomLevel(6));
        cam.scale = 1.0 / 128; assertEquals(6, cam.zoomLevel(6));
        cam.scale = 1.0 / 128; assertEquals(3, cam.zoomLevel(3));
        cam.scale = 40; assertEquals(0, cam.zoomLevel(6));
        int[] levels = {40, 10, 3, 1, 0, 0, 0};
        assertEquals(3, MapCamera.usableZoom(6, levels));
        assertEquals(1, MapCamera.usableZoom(1, levels));
        assertEquals(5, MapCamera.usableZoom(5, null));
        assertTrue(MapCamera.inBounds(0, 0, 0, new int[]{0, 1, 0, 0}));
        assertFalse(MapCamera.inBounds(0, 2, 0, new int[]{0, 1, 0, 0}));
        assertTrue(MapCamera.inBounds(1, 0, 0, new int[]{1, 1, 0, 0})); // a coarse tile covers fine tiles 0 and 1
    }

    @Test void framingFitsTheDimensionWithAMargin() {
        MapCamera cam = new MapCamera();
        cam.resize(512, 512);
        cam.frame(new int[]{-1, 0, -1, 0});            // 2 x 2 tiles = 512 blocks: fit 512/512 * 0.9
        assertEquals(0, cam.cx, 1e-9); assertEquals(0, cam.cz, 1e-9);
        assertEquals(0.9, cam.scale, 1e-9);
    }

    @Test void flyingEasesAndLands() {
        MapCamera cam = new MapCamera();
        cam.resize(100, 100); cam.cx = 0; cam.cz = 0; cam.scale = 1;
        cam.flyTo(100, 200, 4, 1000);
        assertTrue(cam.tick(1100));
        assertTrue(cam.cx > 0 && cam.cx < 100 && cam.scale > 1 && cam.scale < 4);
        cam.tick(1600);
        assertEquals(100, cam.cx, 1e-9); assertEquals(200, cam.cz, 1e-9); assertEquals(4, cam.scale, 1e-9);
        assertFalse(cam.flying());
    }
}
