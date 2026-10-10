package net.scopenet.integration;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.sun.net.httpserver.HttpServer;
import org.junit.jupiter.api.*;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.util.UUID;
import java.util.concurrent.atomic.AtomicInteger;
import static org.junit.jupiter.api.Assertions.*;

class ClaimIndexTest {
    private static final UUID MEMBER = UUID.fromString("b50ad385-829d-3141-a216-7e7d7539ba7f");
    private static final UUID OUTSIDER = UUID.randomUUID();
    private static final String INDEX = """
            {"revision":"7","unchanged":false,
             "guilds":[{"id":"g1","name":"Iron Fortress","tag":"IRON"}],
             "claims":[["minecraft:overworld",3,-4,0],["minecraft:the_nether",-1,-1,0]],
             "members":{"g1":["b50ad385-829d-3141-a216-7e7d7539ba7f","not-a-uuid"]}}""";

    private static JsonObject json(String s) { return JsonParser.parseString(s).getAsJsonObject(); }

    @Test void adminClaimFlagsAreAnsweredLocallyAndOnlyForAdminClaims() {
        ClaimIndex index = new ClaimIndex();
        assertNull(index.adminFlag("minecraft:overworld", 0, 0, "pvp"), "nothing before the first load");
        index.apply(json("""
                {"revision":"1","unchanged":false,
                 "guilds":[{"id":"g1","name":"Iron","tag":"ADMIN"},
                           {"id":"admin:a1","name":"Spawn","tag":"ADMIN","admin":true,"description":"","color":"#22c55e","flags":{"pvp":false,"build":true,"fly":false}},
                           {"id":"admin:a2","name":"Arena","tag":"ADMIN","admin":true}],
                 "claims":[["minecraft:overworld",0,0,1],["minecraft:overworld",1,0,0],["minecraft:overworld",2,0,2]],
                 "members":{}}"""));
        assertEquals(Boolean.FALSE, index.adminFlag("minecraft:overworld", 0, 0, "pvp"));
        assertEquals(Boolean.TRUE, index.adminFlag("minecraft:overworld", 0, 0, "build"));
        assertEquals(Boolean.FALSE, index.adminFlag("minecraft:overworld", 0, 0, "fly"));
        assertEquals(Boolean.TRUE, index.adminFlag("minecraft:overworld", 0, 0, "mob_spawning"), "an unset flag takes the panel's default");
        assertEquals(Boolean.FALSE, index.adminFlag("minecraft:overworld", 0, 0, "explosions"), "protected by default");
        assertNull(index.adminFlag("minecraft:overworld", 1, 0, "build"), "a faction named like an admin claim is still just a faction");
        assertNull(index.adminFlag("minecraft:overworld", 9, 9, "pvp"), "wilderness");
        assertEquals(Boolean.FALSE, index.adminFlag("minecraft:overworld", 2, 0, "build"), "an admin claim with no flags at all is fully protected");
        assertEquals(Boolean.TRUE, index.adminFlag("minecraft:overworld", 2, 0, "pvp"));
    }

    @Test void guildLandRulesComeFromTheGuildAndDefaultToHowGuildLandAlwaysWorked() {
        ClaimIndex index = new ClaimIndex();
        index.apply(json("""
                {"revision":"1","unchanged":false,
                 "guilds":[{"id":"g1","name":"Iron","tag":"IRON","flags":{"build":true,"pvp":false,"entry":false}},
                           {"id":"g2","name":"Gold","tag":"GOLD"}],
                 "claims":[["minecraft:overworld",0,0,0],["minecraft:overworld",1,0,1]],
                 "members":{}}"""));
        assertEquals(Boolean.TRUE, index.claimFlag("minecraft:overworld", 0, 0, "build"), "this faction lets visitors build");
        assertEquals(Boolean.FALSE, index.claimFlag("minecraft:overworld", 0, 0, "pvp"), "and keeps the peace");
        assertEquals(Boolean.FALSE, index.claimFlag("minecraft:overworld", 0, 0, "entry"));
        assertEquals(Boolean.FALSE, index.claimFlag("minecraft:overworld", 1, 0, "build"), "an untouched faction stays protected");
        assertEquals(Boolean.TRUE, index.claimFlag("minecraft:overworld", 1, 0, "pvp"));
        assertEquals(Boolean.FALSE, index.claimFlag("minecraft:overworld", 1, 0, "explosions"));
        assertNull(index.claimFlag("minecraft:overworld", 9, 9, "build"), "wilderness has no rules");
        assertNull(index.adminFlag("minecraft:overworld", 0, 0, "build"), "adminFlag still answers only for admin claims");
    }

    @Test void membersAlwaysMayModifyAndVisitorsOnlyWhereTheRuleIsOn() {
        ClaimIndex index = new ClaimIndex();
        index.apply(json("""
                {"revision":"1","unchanged":false,
                 "guilds":[{"id":"g1","name":"Iron","tag":"IRON","flags":{"build":true,"containers":false}},
                           {"id":"g2","name":"Gold","tag":"GOLD"},
                           {"id":"admin:a1","name":"Spawn","tag":"ADMIN","admin":true,"flags":{"interact":true}}],
                 "claims":[["minecraft:overworld",0,0,0],["minecraft:overworld",1,0,1],["minecraft:overworld",2,0,2]],
                 "members":{"g1":["b50ad385-829d-3141-a216-7e7d7539ba7f"],"g2":["b50ad385-829d-3141-a216-7e7d7539ba7f"]}}"""));
        String d = "minecraft:overworld";
        assertTrue(index.mayModify(d, 9, 9, OUTSIDER, "build"), "wilderness is open");
        assertTrue(index.mayModify(d, 0, 0, OUTSIDER, "build"), "this faction opened building to visitors");
        assertFalse(index.mayModify(d, 0, 0, OUTSIDER, "containers"), "but not its chests");
        assertFalse(index.mayModify(d, 1, 0, OUTSIDER, "build"), "an untouched faction stays closed");
        assertTrue(index.mayModify(d, 1, 0, MEMBER, "build"), "members always may");
        assertTrue(index.mayModify(d, 2, 0, OUTSIDER, "interact"), "an admin claim can leave doors usable");
        assertFalse(index.mayModify(d, 2, 0, OUTSIDER, "build"));
    }

    @Test void answersLocallyAndTreatsWildernessAsOpen() {
        ClaimIndex index = new ClaimIndex();
        assertFalse(index.loaded());
        assertTrue(index.check("minecraft:overworld", 3, -4, OUTSIDER).allowed(), "nothing is enforced before the first load");
        assertTrue(index.apply(json(INDEX)));
        assertEquals("7", index.revision());
        assertEquals(2, index.claimCount());

        ChunkCheckResult mine = index.check("minecraft:overworld", 3, -4, MEMBER);
        assertTrue(mine.claimed() && mine.allowed());
        ChunkCheckResult theirs = index.check("minecraft:overworld", 3, -4, OUTSIDER);
        assertTrue(theirs.claimed() && !theirs.allowed());
        assertEquals("IRON", theirs.guildTag());
        assertFalse(index.check("minecraft:overworld", 3, -3, OUTSIDER).claimed());
        assertFalse(index.check("minecraft:the_end", 3, -4, OUTSIDER).claimed(), "claims are per dimension");
        assertTrue(index.isClaimed("minecraft:the_nether", -1, -1));
        assertFalse(index.isClaimed("minecraft:overworld", -1, -1));
    }

    @Test void negativeCoordinatesDoNotCollide() {
        ClaimIndex index = new ClaimIndex();
        index.apply(json("""
                {"revision":"1","guilds":[{"id":"g","name":"A","tag":"A"}],
                 "claims":[["d",-1,0,0],["d",0,-1,0],["d",-1,-1,0]],"members":{}}"""));
        assertTrue(index.isClaimed("d", -1, 0));
        assertTrue(index.isClaimed("d", 0, -1));
        assertTrue(index.isClaimed("d", -1, -1));
        assertFalse(index.isClaimed("d", 0, 0));
        assertFalse(index.isClaimed("d", 1, -1));
    }

    @Test void unchangedResponsesKeepTheCurrentCopy() {
        ClaimIndex index = new ClaimIndex();
        index.apply(json(INDEX));
        assertFalse(index.apply(json("{\"revision\":\"7\",\"unchanged\":true}")));
        assertEquals(2, index.claimCount());
    }

    @Test void pollingSendsTheRevisionAndSurvivesPanelOutages() throws Exception {
        HttpServer server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        AtomicInteger calls = new AtomicInteger();
        StringBuilder lastBody = new StringBuilder();
        server.createContext("/api/server/v1/guilds/claim-index", exchange -> {
            String body = new String(exchange.getRequestBody().readAllBytes(), StandardCharsets.UTF_8);
            lastBody.setLength(0);
            lastBody.append(body);
            int n = calls.incrementAndGet();
            byte[] out = (n == 1 ? INDEX : n == 2 ? "{\"revision\":\"7\",\"unchanged\":true}" : "boom").getBytes(StandardCharsets.UTF_8);
            exchange.sendResponseHeaders(n >= 3 ? 503 : 200, out.length);
            exchange.getResponseBody().write(out);
            exchange.close();
        });
        server.start();
        try {
            PanelClient client = new PanelClient(Settings.of("http://127.0.0.1:" + server.getAddress().getPort(), "sn_" + "a".repeat(40)));
            assertTrue(client.refreshClaims());
            assertEquals("{}", lastBody.toString(), "first poll has no revision");
            assertFalse(client.refreshClaims());
            assertEquals("{\"revision\":\"7\"}", lastBody.toString());
            assertThrows(java.io.IOException.class, client::refreshClaims);
            // The last good copy still protects land while the panel is down.
            assertFalse(client.checkChunk("minecraft:overworld", 3, -4, OUTSIDER).allowed());
            assertTrue(client.checkChunk("minecraft:overworld", 3, -4, MEMBER).allowed());
            assertTrue(client.isClaimed("minecraft:overworld", 3, -4));
            client.close();
        } finally {
            server.stop(0);
        }
    }

    @Test void adminClaimsProtectLandAndDescribeThemselves() {
        ClaimIndex index = new ClaimIndex();
        index.apply(JsonParser.parseString("""
                {"revision":"9","unchanged":false,
                 "guilds":[{"id":"admin:ac_1","name":"Spawn","tag":"ADMIN","admin":true,"description":"No building here","color":"#22c55e"}],
                 "claims":[["minecraft:overworld",0,0,0],["minecraft:overworld",1,0,0]],
                 "members":{}}""").getAsJsonObject());
        assertFalse(index.check("minecraft:overworld", 0, 0, MEMBER).allowed(), "nobody builds without the bypass permission");
        var info = index.info("minecraft:overworld", 1, 0);
        assertEquals("Spawn", info.name());
        assertEquals("No building here", info.description());
        assertTrue(info.admin());
        assertNull(index.info("minecraft:overworld", 5, 5));

        var region = index.byDimension().get("minecraft:overworld").get(0);
        assertTrue(region.admin());
        assertEquals(0x22c55e, region.color());
        var overlay = net.scopenet.core.map.MapOverlay.toJson(net.scopenet.core.map.MapModel.build(new net.scopenet.core.map.MapModel.Sources(
                index.byDimension(), java.util.Map.of(), java.util.List.of(), java.util.Map.of(), java.util.Map.of(), java.util.List.of(),
                new com.google.gson.JsonArray(), java.util.List.of(), "", false, java.util.Map.of())));
        var claim = overlay.getAsJsonArray("claims").get(0).getAsJsonObject();
        assertEquals("#22c55e", claim.get("color").getAsString());
        assertEquals("No building here", claim.getAsJsonArray("lines").get(1).getAsString());
    }
}
