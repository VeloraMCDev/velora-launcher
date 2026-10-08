package net.scopenet.integration;

import com.google.gson.JsonObject;
import org.junit.jupiter.api.Test;
import java.util.UUID;
import java.util.concurrent.atomic.AtomicLong;
import static org.junit.jupiter.api.Assertions.*;

class ActivityTest {
    @Test void deltasAndFractionalPlaytimeSurviveSnapshots() {
        AtomicLong time = new AtomicLong();
        Activity activity = new Activity(time::get);
        UUID id = UUID.randomUUID();
        activity.join(id, "Steve");
        activity.join(id, "Steve");
        activity.add(id, "Steve", "blocks_broken", 3);
        time.set(1_500_000_000L);
        JsonObject first = activity.drain(19.5);
        JsonObject row = first.getAsJsonArray("stats").get(0).getAsJsonObject();
        assertEquals(1, row.get("joins").getAsInt());
        assertEquals(1, row.get("playtime_secs").getAsInt());
        assertEquals(3, row.get("blocks_broken").getAsInt());
        time.set(2_000_000_000L);
        JsonObject second = activity.drain(20);
        assertEquals(1, second.getAsJsonArray("stats").get(0).getAsJsonObject().get("playtime_secs").getAsInt());
        assertFalse(second.getAsJsonArray("stats").get(0).getAsJsonObject().has("joins"));
        assertNotEquals(first.get("batch_id"), second.get("batch_id"));
        activity.leave(id, "Steve");
        activity.leave(id, "Steve");
        JsonObject last = activity.drain(20);
        assertEquals(0, last.getAsJsonArray("online").size());
        assertEquals(1, last.getAsJsonArray("events").size());
    }

    @Test void shutdownClearsOnlineAndIncludesLastSessionTime() {
        AtomicLong time = new AtomicLong();
        Activity activity = new Activity(time::get);
        activity.join(UUID.randomUUID(), "Steve");
        time.set(3_000_000_000L);
        activity.leaveAll();
        JsonObject payload = activity.drain(Double.NaN);
        assertEquals(0, payload.getAsJsonArray("online").size());
        assertEquals(3, payload.getAsJsonArray("stats").get(0).getAsJsonObject().get("playtime_secs").getAsInt());
        assertEquals(20, payload.get("tps").getAsInt());
    }
}
