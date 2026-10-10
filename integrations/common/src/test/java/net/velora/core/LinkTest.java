package net.velora.core;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import net.scopenet.integration.ChunkCheckResult;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.util.*;

import static org.junit.jupiter.api.Assertions.*;

class LinkTest {
    @Test void mapClaimEditsDeriveIdentityAndDimensionAndRejectFractionalCoordinates(){
        kit.panel.on("guilds/claim","{\"ok\":true}");link.receive(alex,msg("{\"t\":\"hello\",\"protocol\":2}"));kit.now.addAndGet(1000);
        link.receive(alex,msg("{\"t\":\"request\",\"id\":\"claim-one\",\"operation\":\"claim_edit\",\"args\":{\"action\":\"claim\",\"x\":-2,\"z\":3,\"uuid\":\"forged\",\"dimension\":\"forged\"}}"));
        int index=kit.panel.calls.indexOf("guilds/claim");assertTrue(index>=0);JsonObject body=kit.panel.bodies.get(index);assertEquals(alex.uuid().toString(),body.get("uuid").getAsString());assertEquals(alex.pos().world(),body.get("dimension").getAsString());assertTrue(kit.panel.claimsRefreshed);
        kit.now.addAndGet(1000);link.receive(alex,msg("{\"t\":\"request\",\"id\":\"claim-two\",\"operation\":\"claim_edit\",\"args\":{\"action\":\"claim\",\"x\":1.5,\"z\":3}}"));assertEquals(1,kit.panel.calls.stream().filter("guilds/claim"::equals).count());
        alex.denied.add("velora.command.faction.claim");kit.now.addAndGet(1000);link.receive(alex,msg("{\"t\":\"request\",\"id\":\"claim-three\",\"operation\":\"claim_edit\",\"args\":{\"action\":\"claim\",\"x\":1,\"z\":3}}"));assertEquals(1,kit.panel.calls.stream().filter("guilds/claim"::equals).count());
    }
    static final String INFO = "{\"exists\":true,\"name\":\"Alex\",\"joined\":\"2026-01-02T03:04:05Z\",\"global\":{\"level\":12,\"xp\":5000,\"next_level_xp\":6000,\"current_level_xp\":4800,\"progress_pct\":16.6,\"title\":\"Miner\",\"rank\":3},"
            + "\"server\":{\"level\":4,\"xp\":300,\"progress_pct\":50.0,\"rank_name\":\"Initiate\"},\"guild\":{\"id\":\"g\",\"name\":\"Iron\",\"tag\":\"IRON\",\"role\":\"leader\",\"claims\":3},"
            + "\"balance\":1234.5,\"playtime_secs\":7300,\"server_playtime_secs\":600,\"kills\":10,\"deaths\":4,\"friends\":6,\"achievements\":9,"
            + "\"quests\":{\"daily\":{\"total\":3,\"completed\":1,\"claimed\":0},\"weekly\":{\"total\":2,\"completed\":2,\"claimed\":1}},\"rank\":{\"group\":\"vip\",\"display\":\"VIP\",\"prefix\":\"[VIP]\"}}";

    @TempDir Path dir;
    Kit kit;
    Kit.Player alex;
    PlayerCache cache;
    ClientLink link;
    final List<JsonObject> sent = new ArrayList<>();

    @BeforeEach void setup() {
        kit = new Kit(dir);
        alex = kit.platform.add("Alex");
        kit.panel.on("player/info", INFO);
        cache = new PlayerCache(kit.env);
        link = new ClientLink(kit.env, cache, "SMP", (p, bytes) -> sent.add(JsonParser.parseString(Wire.decode(bytes)).getAsJsonObject()));
    }

    byte[] msg(String json) { return Wire.encode(json); }
    JsonObject last(String type) { for (int i = sent.size() - 1; i >= 0; i--) if (sent.get(i).get("t").getAsString().equals(type)) return sent.get(i); return null; }

    @Test void wireRoundTripsAndRejectsGarbage() {
        String text = "{\"t\":\"x\",\"u\":\"héllo ☃\"}";
        assertEquals(text, Wire.decode(Wire.encode(text)));
        assertThrows(IllegalArgumentException.class, () -> Wire.decode(new byte[]{(byte) 0xff, (byte) 0xff, (byte) 0xff, (byte) 0xff, (byte) 0xff}));
        assertThrows(IllegalArgumentException.class, () -> Wire.decode(new byte[]{10, 1, 2}));
        assertThrows(IllegalArgumentException.class, () -> Wire.decode(new byte[0]));
        // 300-byte message uses a two-byte length.
        String big = "x".repeat(300);
        assertEquals(302, Wire.encode(big).length);
        assertEquals(big, Wire.decode(Wire.encode(big)));
    }

    @Test void nothingIsSentBeforeHelloAndGarbageIsIgnored() {
        link.tick();
        link.receive(alex, new byte[]{1, 2, 3});
        link.receive(alex, msg("{\"t\":\"market\"}"));
        link.receive(alex, msg("[1,2]"));
        assertTrue(sent.isEmpty());
        assertFalse(link.hasClient(alex.id));
    }

    @Test void helloSendsFeaturesStateAndClaims() {
        kit.panel.claims.put("minecraft:overworld:0:0", new ChunkCheckResult(true, true, "Iron", "IRON"));
        kit.panel.claims.put("minecraft:overworld:1:0", new ChunkCheckResult(true, false, "Void", "VOID"));
        link.receive(alex, msg("{\"t\":\"hello\",\"mod\":\"0.1\"}"));
        JsonObject hello = last("hello");
        assertEquals("SMP", hello.get("server").getAsString());
        assertTrue(hello.getAsJsonObject("features").get("economy").getAsBoolean());
        JsonObject state = last("state");
        assertEquals(12, state.get("level").getAsInt());
        assertEquals(1234.5, state.get("balance").getAsDouble());
        assertEquals("IRON", state.getAsJsonObject("guild").get("tag").getAsString());
        assertEquals(2, state.getAsJsonObject("weekly").get("done").getAsInt());
        assertEquals("VIP", state.get("rank").getAsString());
        JsonObject claims = last("claims");
        assertEquals(81, claims.get("cells").getAsString().length());
        assertEquals('1', claims.get("cells").getAsString().charAt(4 * 9 + 4), "own chunk in the middle");
        assertEquals('2', claims.get("cells").getAsString().charAt(4 * 9 + 5), "someone else's to the east");
        assertEquals("IRON", claims.getAsJsonObject("owner").get("tag").getAsString());
        assertTrue(claims.getAsJsonObject("owner").get("mine").getAsBoolean());
    }

    @Test void updatesAreOnlySentWhenSomethingChanged() {
        link.receive(alex, msg("{\"t\":\"hello\"}"));
        int before = sent.size();
        link.tick();
        link.tick();
        assertEquals(before, sent.size(), "no change, no traffic");
        alex.pos = new Pos("minecraft:overworld", 40, 64, 0, 0, 0); // next chunk over
        link.tick();
        assertEquals(before + 1, sent.size());
        assertEquals(2, last("claims").get("cx").getAsInt());
        link.disconnected(alex.id);
        alex.pos = new Pos("minecraft:overworld", 400, 64, 0, 0, 0);
        link.tick();
        assertEquals(before + 1, sent.size(), "disconnected players get nothing");
    }

    @Test void stateRefreshesFromThePanelOnTheCacheClock() {
        link.receive(alex, msg("{\"t\":\"hello\"}"));
        kit.panel.on("player/info", INFO.replace("\"balance\":1234.5", "\"balance\":99.0"));
        kit.now.addAndGet(PlayerCache.REFRESH_MS + 1);
        cache.tick(p -> link.pushState(p));
        assertEquals(99.0, last("state").get("balance").getAsDouble());
    }

    @Test void requestsAreThrottledPerPlayer() {
        link.receive(alex, msg("{\"t\":\"hello\"}"));
        kit.panel.on("economy/market", "[]");
        kit.now.addAndGet(1000);
        link.receive(alex, msg("{\"t\":\"market\"}"));
        for (int i = 0; i < 20; i++) link.receive(alex, msg("{\"t\":\"market\"}"));
        assertEquals(1, kit.panel.calls.stream().filter(c -> c.equals("economy/market")).count(), "a burst is one panel call");
        kit.now.addAndGet(ClientLink.MIN_REQUEST_GAP_MS + 1);
        link.receive(alex, msg("{\"t\":\"market\"}"));
        assertEquals(2, kit.panel.calls.stream().filter(c -> c.equals("economy/market")).count());
    }

    @Test void marketAndShopRequests() {
        link.receive(alex, msg("{\"t\":\"hello\"}"));
        kit.now.addAndGet(1000);
        kit.panel.on("economy/market", "[{\"id\":7,\"seller_name\":\"Mia\",\"item_id\":\"DIAMOND\",\"item_name\":\"Diamond\",\"amount\":3,\"price\":99.5}]");
        link.receive(alex, msg("{\"t\":\"market\"}"));
        kit.now.addAndGet(1000);
        assertEquals(7, last("market").getAsJsonArray("listings").get(0).getAsJsonObject().get("id").getAsInt());
        alex.held = new Item("DIAMOND", "Diamond", 4, "");
        kit.now.addAndGet(1000);
        link.receive(alex, msg("{\"t\":\"shop\"}"));
        JsonObject shop = last("shop");
        assertTrue(shop.getAsJsonArray("items").size() > 10);
        assertEquals(35.0, shop.getAsJsonObject("held").get("unit").getAsDouble());
    }

    @Test void panelEventsBecomeToasts() {
        link.receive(alex, msg("{\"t\":\"hello\"}"));
        link.onPanelEvents(JsonParser.parseString("[{\"kind\":\"level_up\",\"uuid\":\"" + alex.id + "\",\"data\":{\"scope\":\"global\",\"level\":13}},"
                + "{\"kind\":\"achievement\",\"uuid\":\"" + alex.id + "\",\"data\":{\"title\":\"Deep Diver\"}},"
                + "{\"kind\":\"unknown\",\"uuid\":\"" + alex.id + "\",\"data\":{}},{\"kind\":\"level_up\",\"uuid\":\"nope\"}]").getAsJsonArray());
        List<JsonObject> toasts = sent.stream().filter(m -> m.get("t").getAsString().equals("notify")).toList();
        assertEquals(2, toasts.size());
        assertEquals("Level 13", toasts.get(0).get("text").getAsString());
        assertEquals("Deep Diver", toasts.get(1).get("text").getAsString());
    }

    @Test void disabledClientLinkIsSilent() {
        kit.env.features = new Features(true, true, true, true, true, false, "$");
        link.receive(alex, msg("{\"t\":\"hello\"}"));
        assertTrue(sent.isEmpty());
    }

    @Test void companionRequestsUseTheConnectedPlayersIdentityAndRejectUnknownOperations() {
        kit.panel.on("companion", "[]");
        link.receive(alex,msg("{\"t\":\"hello\",\"protocol\":2}"));
        kit.now.addAndGet(1000);
        link.receive(alex,msg("{\"t\":\"request\",\"id\":\"1234\",\"operation\":\"quests\",\"uuid\":\"forged\",\"args\":{}}"));
        int index=kit.panel.calls.indexOf("companion");
        assertTrue(index>=0);
        assertEquals(alex.id.toString(),kit.panel.bodies.get(index).get("uuid").getAsString());
        assertEquals("1234",kit.panel.bodies.get(index).get("request_id").getAsString());
        kit.now.addAndGet(1000);
        link.receive(alex,msg("{\"t\":\"request\",\"id\":\"5678\",\"operation\":\"economy/adjust\"}"));
        assertEquals(1,kit.panel.calls.stream().filter("companion"::equals).count());
        assertTrue(last("response").get("json").getAsString().contains("Unsupported"));
    }

    @Test void guildScreensFollowExistingCommandPermissions() {
        kit.panel.on("companion", "{}");
        link.receive(alex,msg("{\"t\":\"hello\",\"protocol\":2}"));
        // Management commands use the root guild node; there is no requests node in Paper.
        alex.denied.add("velora.command.faction.requests");
        kit.now.addAndGet(1000);
        link.receive(alex,msg("{\"t\":\"request\",\"id\":\"one\",\"operation\":\"guild_requests\"}"));
        assertEquals(1,kit.panel.calls.stream().filter("companion"::equals).count());
        alex.denied.add("velora.command.faction");
        kit.now.addAndGet(1000);
        link.receive(alex,msg("{\"t\":\"request\",\"id\":\"two\",\"operation\":\"guilds\"}"));
        assertEquals(1,kit.panel.calls.stream().filter("companion"::equals).count());
        assertTrue(last("response").get("json").getAsString().contains("permission"));
        alex.denied.remove("velora.command.faction");
        alex.denied.add("velora.command.faction.bank");
        kit.now.addAndGet(1000);
        link.receive(alex,msg("{\"t\":\"request\",\"id\":\"three\",\"operation\":\"guild_bank\"}"));
        assertEquals(1,kit.panel.calls.stream().filter("companion"::equals).count());
    }

    @Test void fragmentsFitPaperAndPreserveAstralCharactersAndEscapedJson() {
        com.google.gson.JsonPrimitive value=new com.google.gson.JsonPrimitive(("\\\"\n\uD83D\uDE80").repeat(12000));
        sent.clear();link.result(alex,"request",value,null);
        StringBuilder rebuilt=new StringBuilder();
        for(JsonObject part:sent){
            assertTrue(Wire.encode(part.toString()).length<32766,"Paper payload limit");
            String chunk=part.get("json").getAsString();
            assertFalse(Character.isHighSurrogate(chunk.charAt(chunk.length()-1)));
            assertFalse(Character.isLowSurrogate(chunk.charAt(0)));
            rebuilt.append(chunk);
        }
        assertTrue(sent.size()>1);assertTrue(sent.size()<=125);
        assertEquals(value,JsonParser.parseString(rebuilt.toString()).getAsJsonObject().get("data"));
    }

    @Test void placeholdersMatchThePaperNames() {
        Placeholders ph = new Placeholders(kit.env, cache);
        assertEquals("1", ph.resolve(alex.id, "level").orElseThrow(), "before the first answer");
        assertEquals("0.00", ph.resolve(alex.id, "balance").orElseThrow());
        cache.refresh(alex, null);
        Map<String, String> expected = new LinkedHashMap<>();
        expected.put("level", "12"); expected.put("xp", "5000"); expected.put("level_progress", "17"); expected.put("title", "Miner");
        expected.put("leaderboard_rank", "3"); expected.put("server_level", "4"); expected.put("server_rank", "Initiate");
        expected.put("guild", "Iron"); expected.put("guild_tag", "IRON"); expected.put("guild_role", "leader"); expected.put("guild_claims", "3"); expected.put("has_guild", "true");
        expected.put("balance", "1234.50"); expected.put("balance_formatted", "$1,234.50"); expected.put("playtime", "2h 1m"); expected.put("playtime_seconds", "7300");
        expected.put("playtime_hours", "2"); expected.put("server_playtime", "10m"); expected.put("quest_progress", "3/5"); expected.put("quest_daily", "1/3");
        expected.put("quest_weekly", "2/2"); expected.put("kills", "10"); expected.put("deaths", "4"); expected.put("kdr", "2.50"); expected.put("friends", "6");
        expected.put("achievements", "9"); expected.put("joined", "2026-01-02"); expected.put("rank", "vip"); expected.put("rank_prefix", "[VIP]");
        expected.forEach((k, v) -> assertEquals(v, ph.resolve(alex.id, k).orElseThrow(), k));
        assertEquals("§a■■§7■■■■■■■■", ph.resolve(alex.id, "level_bar").orElseThrow());
        assertTrue(ph.resolve(alex.id, "nonsense").isEmpty());
        for (String k : Placeholders.KEYS) assertTrue(ph.resolve(alex.id, k).isPresent(), k);
        assertEquals("NO", "NO".toUpperCase());
    }
}
