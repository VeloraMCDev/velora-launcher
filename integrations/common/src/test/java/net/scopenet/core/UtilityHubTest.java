package net.scopenet.core;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import net.scopenet.integration.ChunkCheckResult;
import net.scopenet.integration.ClaimIndex;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class UtilityHubTest {
    @TempDir Path dir;
    Kit kit;
    UtilityHub hub;
    List<CoreCommand> cmds;
    Kit.Player alex, steve;
    volatile Utilities settings;

    static final String SETTINGS = """
            {"heal":{"enabled":true,"amount":10,"cooldown_secs":60},"feed":{"enabled":true,"amount":20,"cooldown_secs":30},
             "fly":{"enabled":true},"vault":{"enabled":true,"count":3,"rows":4,"free_count":1},"echest":{"enabled":true},
             "kits":[{"id":"vip","name":"VIP kit","cooldown_secs":3600,"groups":["vip"],"items":[{"item":"minecraft:diamond","amount":8},{"custom":"blade"}],"commands":["give {player} apple 1"]},
                     {"id":"starter","name":"Starter","one_time":true,"items":[{"item":"minecraft:bread","amount":4}]}]}""";
    static final String ITEMS = """
            [{"id":"blade","title":"Blade","spec":{"item":"minecraft:netherite_sword","name":"&bBlade","enchants":{"sharpness":50},"unbreakable":true}}]""";

    @BeforeEach void setup() {
        kit = new Kit(dir);
        alex = kit.platform.add("Alex");
        steve = kit.platform.add("Steve");
        settings = Utilities.fromJson(JsonParser.parseString(SETTINGS).getAsJsonObject(), JsonParser.parseString(ITEMS).getAsJsonArray());
        hub = new UtilityHub(kit.env, () -> settings, new UtilityStore(dir.resolve("state.json")));
        cmds = hub.commands();
    }

    void run(Kit.Player p, String name, String... args) { Kit.find(cmds, name).run(p, args); }

    @Test void onlyAdminsCanCreateInventoryKitsWithoutRemovingItems() {
        kit.platform.snapshot = JsonParser.parseString("[{\"item\":\"minecraft:diamond_sword\",\"amount\":1,\"data\":{\"format\":\"nbt\",\"value\":\"{Damage:7}\"}}]").getAsJsonArray();
        alex.denied.add("scopenet.admin.kits");
        run(alex, "kit", "create", "blade");
        assertTrue(kit.panel.calls.isEmpty());
        assertFalse(Kit.find(cmds, "kit").complete(alex, new String[]{""}).contains("create"));
        kit.panel.on("kits/create", "{\"ok\":true}");
        run(steve, "kit", "create", "Blade");
        assertEquals("blade", kit.panel.bodies.get(0).get("id").getAsString());
        assertEquals(kit.platform.snapshot, kit.panel.bodies.get(0).getAsJsonArray("items"));
        assertEquals(1, kit.platform.snapshot.size());
        assertTrue(steve.heard("Kit created"));
        run(steve, "kit", "create", "bad.name");
        assertEquals(1, kit.panel.calls.size());
        kit.platform.snapshot = new com.google.gson.JsonArray();
        run(steve, "kit", "create", "empty");
        assertEquals(1, kit.panel.calls.size());
    }

    @Test void capturedItemMetadataSurvivesAmountChanges() {
        var item = JsonParser.parseString("{\"item\":\"minecraft:diamond_sword\",\"data\":{\"format\":\"nbt\",\"value\":\"{Damage:7}\"}}").getAsJsonObject();
        ItemSpec spec = ItemSpec.fromJson(item).withAmount(4);
        assertEquals(4, spec.amount()); assertEquals("nbt", spec.dataFormat()); assertEquals("{Damage:7}", spec.dataValue());
    }

    @Test void healUsesTheConfiguredAmountAndCooldown() {
        run(alex, "heal");
        assertEquals(List.of("Alex:10.0"), kit.platform.healed);
        run(alex, "heal");
        assertEquals(1, kit.platform.healed.size());
        assertTrue(alex.heard("again in 1m 0s"), alex.inbox.toString());
        kit.now.addAndGet(61_000);
        run(alex, "heal");
        assertEquals(2, kit.platform.healed.size());
    }

    @Test void staffCanHealOthersWithoutACooldownAndOthersCannot() {
        alex.denied.add("scopenet.command.heal.others");
        run(alex, "heal", "Steve");
        assertTrue(alex.heard("only /heal yourself"));
        run(steve, "heal", "Alex");
        run(steve, "heal", "Alex");
        assertEquals(2, kit.platform.healed.size(), "healing others has no cooldown");
        assertTrue(alex.heard("You were healed"));
    }

    @Test void feedRestoresHungerAndHonoursItsOwnCooldown() {
        run(alex, "feed");
        run(alex, "feed");
        assertEquals(List.of("Alex:20"), kit.platform.fed);
        alex.granted.add("scopenet.cooldown.bypass");
        run(alex, "feed");
        assertEquals(2, kit.platform.fed.size());
    }

    @Test void flyNeedsEitherPermissionAndGuildFlightStaysInsideOwnLand() {
        alex.denied.addAll(List.of("free.fly", "guild.fly"));
        run(alex, "fly");
        assertTrue(alex.heard("permission"));

        alex.denied.remove("guild.fly");
        run(alex, "fly");
        assertTrue(alex.heard("inside your own guild"), "wilderness");
        kit.panel.claims.put("minecraft:overworld:0:0", new ChunkCheckResult(true, true, "Iron", "IRON"));
        run(alex, "fly");
        assertEquals(Boolean.TRUE, kit.platform.flight.get(alex.id));

        // Walking out of the claim switches it off on the next tick.
        alex.pos = new Pos("minecraft:overworld", 100, 64, 0, 0, 0);
        hub.tick();
        assertEquals(Boolean.FALSE, kit.platform.flight.get(alex.id));
        assertTrue(alex.heard("left your guild"));

        // A rival guild's land doesn't count, and neither does an admin claim.
        kit.panel.claims.put("minecraft:overworld:6:0", new ChunkCheckResult(true, false, "Void", "VOID"));
        run(alex, "fly");
        assertNotEquals(Boolean.TRUE, kit.platform.flight.get(alex.id));
    }

    @Test void freeFlyWorksAnywhereAndToggles() {
        steve.denied.add("guild.fly");
        steve.granted.add("free.fly");
        steve.pos = new Pos("minecraft:overworld", 5000, 64, 5000, 0, 0);
        run(steve, "fly");
        assertEquals(Boolean.TRUE, kit.platform.flight.get(steve.id));
        hub.tick();
        assertEquals(Boolean.TRUE, kit.platform.flight.get(steve.id), "no cutoff for free.fly");
        run(steve, "fly");
        assertEquals(Boolean.FALSE, kit.platform.flight.get(steve.id));
        run(steve, "fly", "on");
        assertEquals(Boolean.TRUE, kit.platform.flight.get(steve.id));
    }

    @Test void vaultsAreNumberedAndLockedByPermission() {
        alex.denied.add("scopenet.vault.2");
        alex.denied.add("scopenet.vault.*");
        run(alex, "vault");
        assertTrue(alex.heard("/vault 2") && alex.heard("locked"), alex.inbox.toString());
        run(alex, "vault", "1");
        assertEquals(List.of("Alex:1x4"), kit.platform.vaults);
        run(alex, "vault", "2");
        assertTrue(alex.heard("locked for your rank"));
        run(alex, "vault", "9");
        assertTrue(alex.heard("from 1 to 3"));
        alex.denied.remove("scopenet.vault.3");
        run(alex, "vault", "3");
        assertEquals(2, kit.platform.vaults.size());
        run(alex, "echest");
        assertEquals(1, kit.platform.enderOpened);
    }

    @Test void kitsFollowLuckPermsGroupsCooldownsAndOneTimeRules() {
        alex.denied.add("group.vip");
        alex.denied.add("scopenet.kit.vip");
        run(alex, "kit", "vip");
        assertTrue(alex.heard("no kit called"), "not in the group");
        run(alex, "kit");
        assertTrue(alex.heard("/kit starter") && !alex.heard("/kit vip"));

        run(steve, "kit", "vip");
        assertEquals(2, kit.platform.given.size());
        assertEquals("minecraft:diamond", kit.platform.given.get(0).item());
        assertEquals(8, kit.platform.given.get(0).amount());
        assertEquals(50, kit.platform.given.get(1).enchants().get("sharpness").intValue(), "custom item resolved from the panel");
        assertEquals(List.of("give Steve apple 1"), kit.platform.console);
        run(steve, "kit", "vip");
        assertTrue(steve.heard("ready again in 1h 0m"));
        kit.now.addAndGet(3_600_001);
        run(steve, "kit", "vip");
        assertEquals(4, kit.platform.given.size());

        run(alex, "kit", "starter");
        run(alex, "kit", "starter");
        assertTrue(alex.heard("already claimed"));
    }

    @Test void cooldownsSurviveARestart() {
        run(alex, "heal");
        UtilityHub again = new UtilityHub(kit.env, () -> settings, new UtilityStore(dir.resolve("state.json")));
        Kit.find(again.commands(), "heal").run(alex, new String[0]);
        assertEquals(1, kit.platform.healed.size());
    }

    @Test void customItemsAreGivenByAdminsOnly() {
        run(alex, "customitem", "give", "Steve", "blade", "2");
        assertEquals(2, kit.platform.given.get(0).amount());
        assertTrue(steve.heard("special item"));
        run(alex, "customitem", "give", "Steve", "nope");
        assertTrue(alex.heard("No custom item"));
        run(alex, "customitem", "list");
        assertTrue(alex.heard("blade"));
        assertEquals("scopenet.admin.customitem", Kit.find(cmds, "customitem").permission());
    }

    @Test void adminClaimCommandsTalkToThePanelWithTheChunkYouStandIn() {
        alex.pos = new Pos("minecraft:overworld", -17, 64, 40, 0, 0);
        JsonObject[] seen = new JsonObject[1];
        kit.panel.handlers.put("admin-claims", b -> { seen[0] = b; return JsonParser.parseString("{\"ok\":true,\"added\":9,\"skipped\":0}"); });
        run(alex, "adminclaim", "create", "Spawn", "Welcome", "to", "spawn", "1");
        assertEquals("create", seen[0].get("action").getAsString());
        assertEquals("Spawn", seen[0].get("name").getAsString());
        assertEquals("Welcome to spawn", seen[0].get("value").getAsString());
        assertEquals(-2, seen[0].get("chunk_x").getAsInt());
        assertEquals(2, seen[0].get("chunk_z").getAsInt());
        assertEquals(1, seen[0].get("radius").getAsInt());
        assertTrue(kit.panel.claimsRefreshed);
        assertTrue(alex.heard("9 chunk"));

        run(alex, "adminclaim", "describe", "Spawn", "No", "PvP");
        assertEquals("describe", seen[0].get("action").getAsString());
        assertEquals("No PvP", seen[0].get("value").getAsString());
        run(alex, "adminclaim", "remove");
        assertEquals("", seen[0].get("name").getAsString(), "remove without a name releases whatever you stand in");
    }

    @Test void enteringAndLeavingClaimsShowsABanner() {
        kit.panel.infos.put("minecraft:overworld:0:0", new ClaimIndex.ClaimInfo("admin:1", "Spawn", "ADMIN", "No building here", true));
        kit.panel.infos.put("minecraft:overworld:3:0", new ClaimIndex.ClaimInfo("g1", "Iron Fortress", "IRON", "", false));
        alex.pos = new Pos("minecraft:overworld", 200, 64, 0, 0, 0);
        hub.tick();
        assertTrue(kit.platform.bars.isEmpty(), "first sighting is not an entry");
        alex.pos = new Pos("minecraft:overworld", 4, 64, 4, 0, 0);
        hub.tick();
        assertEquals("⚑ Spawn · No building here", kit.platform.bars.get(0));
        hub.tick();
        assertEquals(1, kit.platform.bars.size(), "standing still doesn't repeat it");
        alex.pos = new Pos("minecraft:overworld", 50, 64, 4, 0, 0);
        hub.tick();
        assertEquals("⚑ [IRON] Iron Fortress territory", kit.platform.bars.get(1));
        alex.pos = new Pos("minecraft:overworld", 500, 64, 4, 0, 0);
        hub.tick();
        assertEquals("Wilderness", kit.platform.bars.get(2));
    }

    @Test void questsShowProgressForYouAndForOthers() {
        String resets = java.time.Instant.now().plusSeconds(3 * 3600).toString();
        kit.panel.on("quests/player", "{\"ok\":true,\"player\":\"Steve\",\"uuid\":\"" + steve.id + "\",\"daily_resets\":\"" + resets + "\",\"weekly_resets\":\"" + resets + "\","
                + "\"daily\":[{\"id\":\"d1\",\"title\":\"Mine 50 stone\",\"progress\":20,\"target\":50,\"xp\":100,\"completed\":false,\"claimed\":false}],"
                + "\"weekly\":[{\"id\":\"w1\",\"title\":\"Slay 20 mobs\",\"progress\":20,\"target\":20,\"xp\":500,\"completed\":true,\"claimed\":false}]}");
        run(alex, "quests", "Steve");
        assertEquals("Steve", kit.panel.bodies.get(kit.panel.bodies.size() - 1).get("player").getAsString());
        assertTrue(alex.heard("Steve's quests"), alex.inbox.toString());
        assertTrue(alex.heard("Mine 50 stone 20/50 +100 XP"), alex.inbox.toString());
        assertTrue(alex.heard("Slay 20 mobs complete"), alex.inbox.toString());
        assertTrue(alex.heard("resets in 2h") || alex.heard("resets in 3h"), alex.inbox.toString());
        run(steve, "quests");
        assertEquals(steve.id.toString(), kit.panel.bodies.get(kit.panel.bodies.size() - 1).get("uuid").getAsString());
        assertTrue(steve.heard("Your quests") && steve.heard("claim it in the launcher"));
    }

    @Test void launcherPurchasesAreDeliveredToTheVaultOnceAndConfirmed() {
        kit.panel.on("economy/market/vault", "{\"deliveries\":[{\"id\":7,\"uuid\":\"" + steve.id + "\",\"item_id\":\"DIAMOND_SWORD\",\"item_name\":\"Diamond Sword\",\"amount\":1,\"item_data\":\"x\",\"note\":\"bought\"}]}");
        kit.panel.on("economy/market/vault/ack", "{\"ok\":true}");
        for (int i = 0; i < 30; i++) hub.tick();
        assertEquals(List.of(steve.id + ":1xDIAMOND_SWORD/3x4"), kit.platform.deposits, "uses the configured vault count and rows");
        assertTrue(steve.heard("arrived in your /vault"));
        JsonObject ack = kit.panel.bodies.get(kit.panel.bodies.size() - 1);
        assertEquals(7, ack.getAsJsonArray("ids").get(0).getAsLong());
        assertEquals(0, ack.getAsJsonArray("failed").size());

        // The panel offers it again because the confirmation was lost: it is confirmed but not delivered twice.
        for (int i = 0; i < 30; i++) hub.tick();
        assertEquals(1, kit.platform.deposits.size());
        assertEquals(7, kit.panel.bodies.get(kit.panel.bodies.size() - 1).getAsJsonArray("ids").get(0).getAsLong());
    }

    @Test void aFullVaultSendsTheItemBackToTheMailbox() {
        kit.platform.vaultHasRoom = false;
        kit.panel.on("economy/market/vault", "{\"deliveries\":[{\"id\":9,\"uuid\":\"" + alex.id + "\",\"item_id\":\"BOW\",\"item_name\":\"Bow\",\"amount\":1,\"item_data\":null,\"note\":\"won\"}]}");
        kit.panel.on("economy/market/vault/ack", "{\"ok\":true}");
        for (int i = 0; i < 30; i++) hub.tick();
        assertTrue(kit.platform.deposits.isEmpty());
        JsonObject ack = kit.panel.bodies.get(kit.panel.bodies.size() - 1);
        assertEquals(0, ack.getAsJsonArray("ids").size());
        assertEquals(9, ack.getAsJsonArray("failed").get(0).getAsLong());
    }
}
