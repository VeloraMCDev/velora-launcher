package net.velora.paper.analytics;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import org.junit.jupiter.api.Test;

import java.util.UUID;

import static org.junit.jupiter.api.Assertions.*;

class AnalyticsTrackerTest {

    @Test
    void testBuildPayloadStructure() {
        AnalyticsTracker tracker = new AnalyticsTracker(null, null, null, null);

        UUID playerId = UUID.randomUUID();
        tracker.addStat(playerId, "Steve", "blocks_broken", 15);
        tracker.addStat(playerId, "Steve", "blocks_placed", 5);
        tracker.addEvent(playerId, "Steve", "command", "/spawn");
        tracker.addAction(playerId, "Steve", "block_broken:STONE@minecraft:overworld", 10);

        JsonObject payload = tracker.buildPayload(19.98);

        assertTrue(payload.has("batch_id"));
        assertDoesNotThrow(() -> UUID.fromString(payload.get("batch_id").getAsString()));

        assertTrue(payload.has("tps"));
        assertEquals(19.98, payload.get("tps").getAsDouble(), 0.01);

        assertTrue(payload.has("stats"));
        JsonArray stats = payload.getAsJsonArray("stats");
        assertEquals(1, stats.size());
        JsonObject playerStats = stats.get(0).getAsJsonObject();
        assertEquals(playerId.toString(), playerStats.get("uuid").getAsString());
        assertEquals("Steve", playerStats.get("name").getAsString());
        assertEquals(15, playerStats.get("blocks_broken").getAsLong());
        assertEquals(5, playerStats.get("blocks_placed").getAsLong());

        assertTrue(payload.has("events"));
        JsonArray events = payload.getAsJsonArray("events");
        assertEquals(2, events.size()); // 1 command event + 1 action converted to event

        // Verifying that after drain, stat counters reset to 0
        JsonObject secondPayload = tracker.buildPayload(20.0);
        assertEquals(0, secondPayload.getAsJsonArray("stats").size());
        assertEquals(0, secondPayload.getAsJsonArray("events").size());
    }

    @Test
    void testTpsClamping() {
        AnalyticsTracker tracker = new AnalyticsTracker(null, null, null, null);

        JsonObject high = tracker.buildPayload(25.5);
        assertEquals(20.0, high.get("tps").getAsDouble());

        JsonObject low = tracker.buildPayload(-5.0);
        assertEquals(0.0, low.get("tps").getAsDouble());
    }
}
