package net.velora.core;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import net.velora.core.map.*;
import net.scopenet.integration.ClaimIndex;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.util.*;

import static org.junit.jupiter.api.Assertions.*;

class MapTest {
    @TempDir Path dir;

    static Set<Long> chunks(String... rows) {
        Set<Long> set = new HashSet<>();
        for (int z = 0; z < rows.length; z++) for (int x = 0; x < rows[z].length(); x++) if (rows[z].charAt(x) == '#') set.add(ChunkRegions.key(x, z));
        return set;
    }

    // ---- outlines --------------------------------------------------------------------

    @Test void singleChunkIsASixteenBlockSquare() {
        List<ChunkRegions.Region> r = ChunkRegions.regions(chunks("#"));
        assertEquals(1, r.size());
        assertEquals(4, r.get(0).outer().points().size());
        assertEquals(256, r.get(0).outer().area(), 0.001);
        assertEquals(8, r.get(0).labelX());
        assertEquals(8, r.get(0).labelZ());
    }

    @Test void straightEdgesAreMergedIntoOneSide() {
        List<ChunkRegions.Region> r = ChunkRegions.regions(chunks("####", "####"));
        assertEquals(4, r.get(0).outer().points().size(), "a 4x2 block is still just a rectangle");
        assertEquals(8 * 256, r.get(0).outer().area(), 0.001);
    }

    @Test void lShapeHasSixCorners() {
        List<ChunkRegions.Region> r = ChunkRegions.regions(chunks("#.", "#.", "##"));
        assertEquals(1, r.size());
        assertEquals(6, r.get(0).outer().points().size());
        assertEquals(4 * 256, r.get(0).outer().area(), 0.001);
        assertEquals(4, r.get(0).chunks());
    }

    @Test void aDonutKeepsItsHole() {
        List<ChunkRegions.Region> r = ChunkRegions.regions(chunks("###", "#.#", "###"));
        assertEquals(1, r.size());
        assertEquals(9 * 256, r.get(0).outer().area(), 0.001);
        assertEquals(1, r.get(0).holes().size());
        assertEquals(-256, r.get(0).holes().get(0).area(), 0.001);
        assertEquals(8, r.get(0).chunks());
        assertTrue(r.get(0).holes().get(0).contains(24, 24), "the hole is the middle chunk");
    }

    @Test void separateAreasAreSeparateRegionsAndDiagonalsDoNotJoin() {
        List<ChunkRegions.Region> apart = ChunkRegions.regions(chunks("#.....#"));
        assertEquals(2, apart.size());
        List<ChunkRegions.Region> diagonal = ChunkRegions.regions(chunks("#.", ".#"));
        assertEquals(2, diagonal.size(), "chunks that only touch at a corner are two areas");
        for (ChunkRegions.Region region : diagonal) assertEquals(256, region.outer().area(), 0.001);
    }

    @Test void theLabelSitsInsideABentShape() {
        // A "U": the average is in the gap, but the label must be on land.
        Set<Long> u = chunks("#.#", "#.#", "###");
        ChunkRegions.Region r = ChunkRegions.regions(u).get(0);
        int cx = (int) Math.floor(r.labelX() / 16), cz = (int) Math.floor(r.labelZ() / 16);
        assertTrue(u.contains(ChunkRegions.key(cx, cz)));
    }

    @Test void negativeCoordinatesWork() {
        Set<Long> set = new HashSet<>(List.of(ChunkRegions.key(-3, -2), ChunkRegions.key(-2, -2), ChunkRegions.key(-3, -1)));
        ChunkRegions.Region r = ChunkRegions.regions(set).get(0);
        assertEquals(3 * 256, r.outer().area(), 0.001);
        assertEquals(-48, r.outer().points().stream().mapToDouble(ChunkRegions.Point::x).min().getAsDouble());
        assertEquals(-32, r.outer().points().stream().mapToDouble(ChunkRegions.Point::z).min().getAsDouble());
    }

    @Test void aBigRandomBlobStaysConsistent() {
        Random rng = new Random(7);
        Set<Long> blob = new HashSet<>();
        for (int i = 0; i < 400; i++) blob.add(ChunkRegions.key(rng.nextInt(20), rng.nextInt(20)));
        double land = 0;
        for (ChunkRegions.Region r : ChunkRegions.regions(blob)) {
            double holes = r.holes().stream().mapToDouble(h -> -h.area()).sum();
            land += r.outer().area() - holes;
        }
        // Area inside all outlines (minus holes) is exactly the claimed land.
        assertEquals(blob.size() * 256.0, land, 0.01);
    }

    // ---- the model -----------------------------------------------------------------------

    static ClaimIndex.GuildClaims guild(String id, String name, String tag, String icon, Set<Long> c) { return new ClaimIndex.GuildClaims(id, name, tag, icon, c); }

    static MapModel.Sources sources(Map<String, List<ClaimIndex.GuildClaims>> claims, Collection<ShopPoints.Shop> shops, JsonArray market, boolean homes) {
        Pos spawn = new Pos("minecraft:overworld", 0, 80, 0, 0, 0);
        UUID owner = UUID.randomUUID();
        return new MapModel.Sources(claims, Map.of("arena", new Pos("minecraft:overworld", 100, 65, -40, 0, 0)), List.of(spawn),
                Map.of(owner, Map.of("base", new Pos("minecraft:overworld", 5, 70, 5, 0, 0))), Map.of("g1", new MapModel.NamedPos("[IRON] Iron Wolves", new Pos("minecraft:overworld", 32, 70, 32, 0, 0))),
                shops, market, List.of(), "https://panel.example.com", homes, Map.of(owner, "Alex"));
    }

    @Test void claimsBecomeColouredRegionsWithNamesAndIcons() {
        Map<String, List<ClaimIndex.GuildClaims>> claims = Map.of("minecraft:overworld", List.of(
                guild("g1", "Iron <Wolves>", "IRON", "/textures/abc", chunks("##", "#.")),
                guild("g2", "Void", "VOID", "https://cdn.example.com/v.png", chunks("#"))));
        MapData data = MapModel.build(sources(claims, List.of(), new JsonArray(), false));
        assertEquals(2, data.claims().size());
        MapData.ClaimRegion iron = data.claims().stream().filter(c -> c.tag().equals("IRON")).findFirst().orElseThrow();
        assertEquals("https://panel.example.com/textures/abc", iron.icon(), "panel paths become full addresses");
        assertEquals("https://cdn.example.com/v.png", data.claims().stream().filter(c -> c.tag().equals("VOID")).findFirst().orElseThrow().icon());
        assertFalse(iron.detail().contains("<Wolves>"), "names are HTML-escaped: " + iron.detail());
        assertTrue(iron.detail().contains("&lt;Wolves&gt;"));
        assertEquals(MapModel.guildColor("g1"), iron.color());
        assertNotEquals(MapModel.guildColor("g1"), MapModel.guildColor("g2"));
        assertTrue(iron.color() >= 0 && iron.color() <= 0xFFFFFF);
        assertEquals("", MapModel.iconAddress("javascript:alert(1)", "https://p"), "only web addresses and panel paths are used as icons");
        assertEquals("", MapModel.iconAddress(null, "https://p"));
    }

    @Test void pinsCoverSpawnWarpsGuildHomesAndShopsButHomesAreOptIn() {
        List<ShopPoints.Shop> shops = List.of(new ShopPoints.Shop("Bazaar", "market", new Pos("minecraft:overworld", 10, 64, 10, 0, 0)),
                new ShopPoints.Shop("Smith", "shop", new Pos("minecraft:overworld", 20, 64, 10, 0, 0)));
        MapData off = MapModel.build(sources(Map.of(), shops, new JsonArray(), false));
        Set<String> kinds = new TreeSet<>();
        off.pins().forEach(p -> kinds.add(p.kind()));
        assertEquals(new TreeSet<>(List.of("guild_home", "market", "shop", "spawn", "warp")), kinds, "homes stay private by default");
        MapData on = MapModel.build(sources(Map.of(), shops, new JsonArray(), true));
        MapData.Pin home = on.pins().stream().filter(p -> p.kind().equals("home")).findFirst().orElseThrow();
        assertEquals("Alex's home", home.label());
        assertEquals("[IRON] Iron Wolves", on.pins().stream().filter(p -> p.kind().equals("guild_home")).findFirst().orElseThrow().label());
    }

    @Test void marketPinsSummariseActivity() {
        JsonArray market = JsonParser.parseString("[{\"id\":1,\"seller_name\":\"Mia\",\"seller_guild\":null,\"item_name\":\"Diamond\",\"amount\":3,\"price\":99.5},"
                + "{\"id\":2,\"seller_name\":\"X\",\"seller_guild\":\"IRON\",\"item_name\":\"<b>Hack</b>\",\"amount\":1,\"price\":5}]").getAsJsonArray();
        ShopPoints.Shop shop = new ShopPoints.Shop("Bazaar", "market", new Pos("minecraft:overworld", 10, 64, 10, 0, 0));
        MapData data = MapModel.build(sources(Map.of(), List.of(shop), market, false));
        String detail = data.pins().stream().filter(p -> p.kind().equals("market")).findFirst().orElseThrow().detail();
        assertTrue(detail.contains("2 listings"));
        assertTrue(detail.contains("Diamond x3 &middot; 99.50"));
        assertTrue(detail.contains("[IRON] faction"));
        assertFalse(detail.contains("<b>Hack"), "listing names can't inject HTML");
        assertTrue(MapModel.build(sources(Map.of(), List.of(shop), new JsonArray(), false)).pins().stream().anyMatch(p -> p.detail().contains("No listings")));
    }

    @Test void mapsAreMatchedToDimensions() {
        assertEquals("minecraft:overworld", MapModel.dimensionOf("world", "World"));
        assertEquals("minecraft:the_nether", MapModel.dimensionOf("world_nether", "Nether"));
        assertEquals("minecraft:the_end", MapModel.dimensionOf("world_the_end", "The End"));
        assertEquals("minecraft:overworld", MapModel.dimensionOf("legends", "Friends & Legends"), "'end' inside another word isn't the End");
    }

    @Test void playersJsonHasOnlyPublicFacts() {
        JsonObject j = MapModel.playersJson("SMP", List.of(new MapData.OnlinePlayer("u-1", "Alex", 12, "Miner", 4, "Iron", "IRON", "leader", "VIP", 7300)), 123L);
        JsonObject a = j.getAsJsonObject("players").getAsJsonObject("u-1");
        assertEquals(12, a.get("level").getAsInt());
        assertEquals("IRON", a.get("guildTag").getAsString());
        assertFalse(a.has("balance"));
        assertEquals("SMP", j.get("server").getAsString());
    }

    // ---- shop points ---------------------------------------------------------------------

    @Test void shopPointsAreManagedByStaffAndSurviveARestart() {
        Kit kit = new Kit(dir);
        Kit.Player alex = kit.platform.add("Alex");
        ShopPoints points = new ShopPoints(dir.resolve("shops.json"));
        Pos block = new Pos("minecraft:overworld", 10.4, 64.2, -3.7, 0, 0);
        alex.denied.add(ShopPoints.NODE);
        points.command(alex, new String[]{"add", "Bazaar"}, block);
        assertTrue(alex.last().contains("permission"));
        alex.denied.clear();
        points.command(alex, new String[]{"add", "bad name"}, block);
        assertTrue(alex.last().contains("Names are"));
        points.command(alex, new String[]{"add", "Bazaar"}, block);
        assertTrue(alex.last().contains("Shop point Bazaar set (market)"));
        points.command(alex, new String[]{"add", "Twin", "shop"}, block);
        assertTrue(alex.last().contains("already a shop point"));
        points.command(alex, new String[]{"add", "Smith", "shop"}, new Pos("minecraft:overworld", 12, 64, -3, 0, 0));

        assertEquals("Bazaar", points.at("minecraft:overworld", 10, 64, -4).orElseThrow().name(), "block coordinates floor correctly");
        assertTrue(points.at("minecraft:the_nether", 10, 64, -4).isEmpty());
        assertTrue(points.at("minecraft:overworld", 11, 64, -4).isEmpty());

        ShopPoints reopened = new ShopPoints(dir.resolve("shops.json"));
        assertEquals(2, reopened.all().size());
        reopened.command(alex, new String[]{"remove", "bazaar"}, block);
        assertEquals(1, reopened.all().size());
        reopened.command(alex, new String[]{"remove", "ghost"}, block);
        assertTrue(alex.last().contains("No shop point"));
        reopened.command(alex, new String[]{"list"}, block);
        assertTrue(alex.heard("Smith (shop)"));
    }

    @Test void marketPointCommandUsesTheBlockUnderFoot() {
        Kit kit = new Kit(dir);
        Kit.Player alex = kit.platform.add("Alex");
        CommandSet set = new CommandSet(kit.env, EssentialsConfig.defaults());
        alex.pos = new Pos("minecraft:overworld", 10.5, 65, 20.5, 0, 0);
        set.run("market", alex, "point", "add", "Bazaar");
        assertTrue(set.shops.at("minecraft:overworld", 10, 64, 20).isPresent());
    }

    // ---- actions from the map ------------------------------------------------------------

    @Test void pollerCarriesOutMessagesNoticesAndTeleportRequests() {
        Kit kit = new Kit(dir);
        Kit.Player alex = kit.platform.add("Alex"), steve = kit.platform.add("Steve");
        List<String> tpas = new ArrayList<>();
        ActionPoller poller = new ActionPoller(kit.env, (from, to) -> tpas.add(from.name() + "->" + to.name()));
        String json = "{\"actions\":[{\"id\":1,\"kind\":\"message\",\"from_uuid\":\"" + alex.id + "\",\"from_name\":\"Alex\",\"to_uuid\":\"" + steve.id + "\",\"text\":\"hello\"},"
                + "{\"id\":2,\"kind\":\"tpa\",\"from_uuid\":\"" + alex.id + "\",\"from_name\":\"Alex\",\"to_uuid\":\"" + steve.id + "\",\"text\":\"\"},"
                + "{\"id\":3,\"kind\":\"notify\",\"from_uuid\":\"\",\"from_name\":\"\",\"to_uuid\":\"" + steve.id + "\",\"text\":\"§eAlex invited you\"},"
                + "{\"id\":4,\"kind\":\"message\",\"from_uuid\":\"\",\"from_name\":\"X\",\"to_uuid\":\"" + UUID.randomUUID() + "\",\"text\":\"to nobody\"},"
                + "{\"id\":5,\"kind\":\"explode\",\"from_uuid\":\"\",\"from_name\":\"X\",\"to_uuid\":\"" + steve.id + "\",\"text\":\"\"}]}";
        kit.panel.on("actions/poll", json);
        poller.tick();
        assertTrue(steve.heard("[Map] Alex: hello"));
        assertEquals(List.of("Alex->Steve"), tpas);
        assertTrue(steve.heard("Alex invited you"));
        assertEquals(2, steve.inbox.size(), "unknown actions and ones for absent players do nothing");

        // It asks the panel at most every few seconds, and never overlaps itself.
        kit.panel.calls.clear();
        poller.tick();
        poller.tick();
        assertTrue(kit.panel.calls.isEmpty());
        kit.now.addAndGet(ActionPoller.EVERY_MS + 1);
        poller.tick();
        assertEquals(1, kit.panel.calls.size());
    }

    @Test void teleportRequestsNeedThePermissionAndAPresentRequester() {
        Kit kit = new Kit(dir);
        Kit.Player alex = kit.platform.add("Alex"), steve = kit.platform.add("Steve");
        List<String> tpas = new ArrayList<>();
        ActionPoller poller = new ActionPoller(kit.env, (from, to) -> tpas.add(from.name()));
        alex.denied.add("velora.command.tpa");
        kit.panel.on("actions/poll", "{\"actions\":[{\"id\":1,\"kind\":\"tpa\",\"from_uuid\":\"" + alex.id + "\",\"from_name\":\"Alex\",\"to_uuid\":\"" + steve.id + "\",\"text\":\"\"}]}");
        poller.tick();
        assertTrue(tpas.isEmpty());
        assertTrue(alex.last().contains("do not have permission"));

        kit.platform.online.remove(alex.id);
        kit.now.addAndGet(ActionPoller.EVERY_MS + 1);
        poller.tick();
        assertTrue(steve.heard("has left"));
    }

    @Test void thePollerSurvivesAPanelOutage() {
        Kit kit = new Kit(dir);
        kit.platform.add("Alex");
        ActionPoller poller = new ActionPoller(kit.env, (a, b) -> { });
        kit.panel.handlers.put("actions/poll", b -> { throw new java.io.IOException("down"); });
        poller.tick();
        kit.now.addAndGet(ActionPoller.EVERY_MS + 1);
        kit.panel.on("actions/poll", "{\"actions\":[]}");
        poller.tick();
        assertEquals(2, kit.panel.calls.size());
    }

    // ---- guild invitations in game -----------------------------------------------------------

    @Test void guildInviteAcceptAndDeclineCommands() {
        Kit kit = new Kit(dir);
        Kit.Player alex = kit.platform.add("Alex");
        CommandSet set = new CommandSet(kit.env, EssentialsConfig.defaults());
        kit.panel.on("guilds/invite/send", "{\"ok\":true,\"player\":\"Steve\"}");
        set.run("guild", alex, "invite", "steve");
        assertTrue(alex.heard("Invitation sent to Steve"));
        assertEquals("steve", kit.panel.bodies.get(kit.panel.bodies.size() - 1).get("target").getAsString());
        set.run("guild", alex, "invite");
        assertTrue(alex.last().startsWith("Usage"));

        kit.panel.on("guilds/invite/respond", "{\"ok\":true,\"accepted\":true,\"guild\":\"Iron\",\"tag\":\"IRON\"}");
        set.run("guild", alex, "accept", "IRON");
        assertTrue(alex.heard("Welcome to [IRON] Iron"));
        assertTrue(kit.panel.claimsRefreshed);
        assertEquals("IRON", kit.panel.bodies.get(kit.panel.bodies.size() - 1).get("tag").getAsString());
        kit.panel.on("guilds/invite/respond", "{\"ok\":true,\"accepted\":false,\"guild\":\"Iron\",\"tag\":\"IRON\"}");
        set.run("guild", alex, "decline");
        assertTrue(alex.last().contains("declined"));
        alex.denied.add("velora.command.faction.invite");
        set.run("guild", alex, "invite", "x");
        assertTrue(alex.last().contains("permission to use /faction invite"));
    }

    @Test void mapServiceAssemblesEverythingOnTheServerSide() {
        Kit kit = new Kit(dir);
        Kit.Player alex = kit.platform.add("Alex");
        kit.panel.on("player/info", LinkTest.INFO);
        CommandSet set = new CommandSet(kit.env, EssentialsConfig.defaults());
        PlayerCache cache = new PlayerCache(kit.env);
        cache.refresh(alex, null);
        alex.pos = new Pos("minecraft:overworld", 100, 65, 100, 0, 0);
        set.essentials.setWarp(alex, "Arena");
        set.run("market", alex, "point", "add", "Bazaar");
        kit.panel.on("economy/market", "[{\"id\":1,\"seller_name\":\"Mia\",\"item_name\":\"Diamond\",\"amount\":1,\"price\":5}]");
        ClaimIndex claims = new ClaimIndex();
        claims.apply(JsonParser.parseString("{\"revision\":\"1\",\"guilds\":[{\"id\":\"g1\",\"name\":\"Iron\",\"tag\":\"IRON\",\"icon_url\":\"/textures/x\"}],"
                + "\"claims\":[[\"minecraft:overworld\",0,0,0],[\"minecraft:overworld\",1,0,0]],\"members\":{\"g1\":[]}}").getAsJsonObject());
        MapService service = new MapService(kit.env, claims, set.mapSource(), cache, "https://panel.example.com", false, "SMP");
        service.tick();
        MapData data = service.snapshot();
        assertEquals(1, data.claims().size());
        assertEquals("https://panel.example.com/textures/x", data.claims().get(0).icon());
        assertEquals(2, data.claims().get(0).chunks());
        Set<String> kinds = new TreeSet<>();
        data.pins().forEach(p -> kinds.add(p.kind()));
        assertEquals(new TreeSet<>(List.of("spawn", "warp", "market")), kinds);
        assertTrue(data.pins().stream().filter(p -> p.kind().equals("market")).findFirst().orElseThrow().detail().contains("1 listing"));
        assertEquals("Alex", data.players().get(0).name());
        assertEquals(12, data.players().get(0).level());
        assertEquals("IRON", service.playersJson().getAsJsonObject("players").getAsJsonObject(alex.id.toString()).get("guildTag").getAsString());

        // Turning claims off hides them.
        kit.env.features = new Features(true, true, true, true, false, true, "$");
        assertTrue(service.snapshot().claims().isEmpty());
    }
}
