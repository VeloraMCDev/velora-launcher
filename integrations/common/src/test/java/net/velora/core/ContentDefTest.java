package net.velora.core;

import com.google.gson.JsonArray;
import com.google.gson.JsonParser;
import org.junit.jupiter.api.Test;

import java.util.List;
import java.util.Random;

import static org.junit.jupiter.api.Assertions.*;

class ContentDefTest {
    private static JsonArray content(String json) { return JsonParser.parseString(json).getAsJsonArray(); }

    @Test void parsesEveryKindAndKeepsAnItemsIdentity() {
        Utilities u = Utilities.fromJson(null, new JsonArray(), content("""
            [{"id":"bench","kind":"decoration","title":"Bench","spec":{"display":{"item":"minecraft:paper","custom_model_data":100001,"scale":1.5},
              "name":"&6Bench","lore":["&7Sit"],"seat":true,"solid":true,"hitbox":{"width":2.0,"height":0.8}}},
             {"id":"ore","kind":"block","title":"Ore","spec":{"display":{"item":"minecraft:paper","custom_model_data":100002},"hardness":3,"light":7,
              "drops":[{"item":"diamond","min":1,"max":3,"chance":0.5}]}},
             {"id":"vault","kind":"chest","title":"Vault","spec":{"display":{"item":"minecraft:paper","custom_model_data":100003},"rows":9,"title":"Treasure"}},
             {"id":"bad","kind":"teleporter","title":"x","spec":{}},
             {"id":"wheat","kind":"crop","title":"Wheat","spec":{"display":{"item":"minecraft:paper","custom_model_data":100004},"growth_seconds":100,
              "stages":[{"item":"minecraft:paper","custom_model_data":100004},{"item":"minecraft:paper","custom_model_data":100005},{"item":"minecraft:paper","custom_model_data":100006}]}}]
            """));
        assertEquals(4, u.content.size(), "unknown kinds are ignored");
        ContentDef bench = u.content.get("bench");
        assertEquals(1.5, bench.look().scale());
        assertTrue(bench.seat() && bench.solid());
        assertEquals(2.0, bench.hitWidth());
        assertSame(bench, u.contentByItem.get("minecraft:paper#100001"), "held items map back to their content");
        assertEquals("&6Bench", bench.toItemSpec().name());
        assertEquals(100001, bench.toItemSpec().customModelData());
        ContentDef ore = u.content.get("ore");
        assertEquals(7, ore.light());
        assertEquals(5, ore.punchesToBreak());
        assertEquals(new ContentDef.Drop("diamond", 1, 3, 0.5), ore.drops().get(0));
        assertEquals(6, u.content.get("vault").rows(), "rows are clamped to a chest's six");
        assertEquals("Treasure", u.content.get("vault").chestTitle());
        ContentDef wheat = u.content.get("wheat");
        assertEquals(3, wheat.stages().size());
        assertEquals(100006, wheat.stageLook(99).customModelData());
        assertEquals(2, wheat.lastStage());
    }

    @Test void cropsGrowEvenlyAndReportTheWait() {
        long t0 = 1_000_000;
        assertEquals(0, ContentDef.cropStage(t0, t0, 100, 3));
        assertEquals(0, ContentDef.cropStage(t0, t0 + 49_000, 100, 3));
        assertEquals(1, ContentDef.cropStage(t0, t0 + 50_000, 100, 3));
        assertEquals(1, ContentDef.cropStage(t0, t0 + 99_000, 100, 3));
        assertEquals(2, ContentDef.cropStage(t0, t0 + 100_000, 100, 3));
        assertEquals(2, ContentDef.cropStage(t0, t0 + 9_999_999, 100, 3));
        assertEquals(0, ContentDef.cropStage(t0, t0 + 5, 100, 1), "a single stage never grows");
        assertEquals(0, ContentDef.cropStage(t0 + 10_000, t0, 100, 3), "a clock that went backwards stays at the start");
        assertEquals(50_000, ContentDef.millisToNextStage(t0, t0, 100, 3));
        assertEquals(1_000, ContentDef.millisToNextStage(t0, t0 + 49_000, 100, 3));
        assertEquals(0, ContentDef.millisToNextStage(t0, t0 + 100_000, 100, 3));
    }

    @Test void dropsRespectChanceAndAmounts() {
        List<ContentDef.Drop> drops = List.of(new ContentDef.Drop("diamond", 2, 4, 1.0), new ContentDef.Drop("emerald", 1, 1, 0.0), new ContentDef.Drop("stick", 0, 0, 1.0));
        for (int i = 0; i < 50; i++) {
            List<ContentDef.Rolled> r = ContentDef.rollDrops(drops, new Random(i));
            assertEquals(1, r.size(), "chance 0 never drops and zero amounts are skipped");
            assertTrue(r.get(0).amount() >= 2 && r.get(0).amount() <= 4);
        }
        int hits = 0;
        Random random = new Random(42);
        for (int i = 0; i < 2000; i++) hits += ContentDef.rollDrops(List.of(new ContentDef.Drop("x", 1, 1, 0.25)), random).size();
        assertTrue(hits > 380 && hits < 620, "about a quarter of the time: " + hits);
    }

    @Test void customFoodIsRemembered() {
        Utilities u = Utilities.fromJson(null, content("""
            [{"id":"pie","title":"Pie","spec":{"item":"minecraft:bread","amount":1,"custom_model_data":100010,"food":{"nutrition":9,"saturation":6.5}}},
             {"id":"plain","title":"Plain","spec":{"item":"minecraft:bread","amount":1}}]
            """), null);
        assertEquals(new ContentDef.Food(9, 6.5), u.foods.get("minecraft:bread#100010"));
        assertEquals(1, u.foods.size());
    }

    @Test void brokenContentNeverBreaksTheRest() {
        Utilities u = Utilities.fromJson(null, new JsonArray(), content("[42, {\"id\":\"ok\",\"kind\":\"npc\",\"title\":\"Ok\",\"spec\":{}}, {\"kind\":\"npc\"}]"));
        assertEquals(List.of("ok"), List.copyOf(u.content.keySet()));
        assertEquals("minecraft:paper#-1", u.content.get("ok").look().key());
        assertTrue(u.contentByItem.isEmpty(), "content without a model has no item identity");
    }
}
