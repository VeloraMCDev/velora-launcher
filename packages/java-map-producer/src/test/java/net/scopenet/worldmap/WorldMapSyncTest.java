package net.scopenet.worldmap;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.sun.net.httpserver.HttpServer;
import net.scopenet.integration.PanelTransport;
import net.scopenet.integration.ServerConnection;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.net.InetSocketAddress;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.*;
import java.util.concurrent.*;

import static org.junit.jupiter.api.Assertions.*;

/** A fake panel and a generated world: tiles, players and the overlay all arrive, and only once. */
class WorldMapSyncTest {
    @Test void worldsAreRenderedAndUploaded(@TempDir Path dir) throws Exception {
        byte[] flat = MapRenderTest.chunk(0, List.of("minecraft:air", "minecraft:grass_block"), (x, y, z) -> y < 5 ? 1 : 0);
        MapRenderTest.region(dir.resolve("world/region"), Map.of(RegionFile.index(1, 1), flat, RegionFile.index(20, 3), flat));

        List<String> tiles = new CopyOnWriteArrayList<>();
        List<JsonObject> players = new CopyOnWriteArrayList<>();
        List<JsonObject> overlays = new CopyOnWriteArrayList<>();
        HttpServer server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        server.createContext("/api/server/v1/map/config", ex -> reply(ex, "{\"enabled\":true,\"epoch\":3,\"max_zoom\":2,\"player_interval_ms\":500}"));
        server.createContext("/api/server/v1/map/tiles", ex -> {
            assertEquals("dim=minecraft:overworld", ex.getRequestURI().getRawQuery());
            ByteBuffer body = ByteBuffer.wrap(ex.getRequestBody().readAllBytes());
            while (body.hasRemaining()) {
                int zoom = body.get() & 0xFF, x = body.getInt(), y = body.getInt(), len = body.getInt();
                byte[] png = new byte[len];
                body.get(png);
                assertEquals((byte) 0x89, png[0]);
                tiles.add(zoom + "/" + x + "/" + y);
            }
            reply(ex, "{\"stored\":1}");
        });
        server.createContext("/api/server/v1/map/players", ex -> { players.add(JsonParser.parseString(new String(ex.getRequestBody().readAllBytes(), StandardCharsets.UTF_8)).getAsJsonObject()); reply(ex, "{}"); });
        server.createContext("/api/server/v1/map/overlay", ex -> { overlays.add(JsonParser.parseString(new String(ex.getRequestBody().readAllBytes(), StandardCharsets.UTF_8)).getAsJsonObject()); reply(ex, "{}"); });
        server.start();
        PanelTransport client = new PanelTransport(ServerConnection.of("http://127.0.0.1:" + server.getAddress().getPort(), "sn_" + "a".repeat(40)));
        WorldMapSync sync = new WorldMapSync(client, dir.resolve("world"), dir.resolve("cache"), System.err::println);
        sync.setOverlay(() -> "{\"claims\":[],\"pins\":[{\"id\":\"spawn\"}]}");
        sync.offerPlayers(List.of(new WorldMapSync.Player("11111111-1111-1111-1111-111111111111", "Steve", "minecraft:overworld", 10.5, 64, 20, 90, 0)));
        try {
            sync.start();
            long deadline = System.currentTimeMillis() + 20_000;
            while ((tiles.size() < 4 || players.isEmpty() || overlays.isEmpty()) && System.currentTimeMillis() < deadline) Thread.sleep(100);
            // chunk (1,1) is in tile 0/0/0, chunk (20,3) in 0/1/0; zoom 1 and 2 are built above them
            assertTrue(tiles.contains("0/0/0"), tiles.toString());
            assertTrue(tiles.contains("0/1/0"), tiles.toString());
            assertTrue(tiles.contains("1/0/0"), tiles.toString());
            assertTrue(tiles.contains("2/0/0"), tiles.toString());
            assertEquals("Steve", players.get(0).getAsJsonArray("players").get(0).getAsJsonObject().get("name").getAsString());
            assertEquals("spawn", overlays.get(0).getAsJsonArray("pins").get(0).getAsJsonObject().get("id").getAsString());
            int sent = tiles.size();
            Thread.sleep(1500);
            assertEquals(sent, tiles.size(), "an unchanged world isn't uploaded again");
            sync.close();
            assertTrue(java.nio.file.Files.isRegularFile(dir.resolve("cache/state.json")));
            WorldMapSync resumed = new WorldMapSync(client, dir.resolve("world"), dir.resolve("cache"), System.err::println);
            try {
                resumed.start();
                long restartDeadline = System.currentTimeMillis() + 10_000;
                while (!resumed.status().contains("caught up") && System.currentTimeMillis() < restartDeadline) Thread.sleep(50);
                assertTrue(resumed.status().contains("caught up"), resumed.status());
                assertEquals(sent, tiles.size(), "persisted cache must not replay unchanged tiles after restart");
            } finally { resumed.close(); }
        } finally {
            sync.close();
            server.stop(0);
        }
    }

    private static void reply(com.sun.net.httpserver.HttpExchange ex, String json) throws java.io.IOException {
        byte[] b = json.getBytes(StandardCharsets.UTF_8);
        ex.sendResponseHeaders(200, b.length);
        ex.getResponseBody().write(b);
        ex.close();
    }
}
