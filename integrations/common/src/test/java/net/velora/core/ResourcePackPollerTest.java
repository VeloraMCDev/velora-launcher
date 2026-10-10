package net.velora.core;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import java.nio.file.Path;
import static org.junit.jupiter.api.Assertions.*;
class ResourcePackPollerTest {
    @Test void packsAreOfferedOncePerSessionAndUpdatedOnRevision(@TempDir Path dir) {
        Kit kit = new Kit(dir); var p = kit.platform.add("Alex");
        String hash = "a".repeat(40);
        kit.panel.on("resource-pack", "{\"enabled\":true,\"required\":false,\"sha1\":\"" + hash + "\"}");
        var poller = new ResourcePackPoller(kit.env, () -> "https://panel.test");
        poller.tick(); poller.tick();
        assertEquals(0, kit.platform.packs.size(), "not while the player is still loading in");
        kit.now.addAndGet(ResourcePackPoller.SETTLE_MILLIS + 1);
        poller.tick(); poller.tick(); poller.tick();
        assertEquals(1, kit.platform.packs.size());
        poller.left(p.id); poller.tick(); assertEquals(1, kit.platform.packs.size(), "a rejoin waits again");
        kit.now.addAndGet(ResourcePackPoller.SETTLE_MILLIS + 1);
        poller.tick(); assertEquals(2, kit.platform.packs.size());
        kit.now.addAndGet(60_001);
        kit.panel.on("resource-pack", "{\"enabled\":true,\"required\":true,\"sha1\":\"" + "b".repeat(40) + "\"}");
        poller.tick(); poller.tick(); assertEquals(3, kit.platform.packs.size());
        assertTrue(kit.platform.packs.get(2).endsWith(":true"));
    }
    @Test void aFailedDownloadIsOfferedAgainAFewTimes(@TempDir Path dir) {
        Kit kit = new Kit(dir); var p = kit.platform.add("Alex");
        kit.panel.on("resource-pack", "{\"enabled\":true,\"required\":false,\"sha1\":\"" + "a".repeat(40) + "\"}");
        var poller = new ResourcePackPoller(kit.env, () -> "https://panel.test");
        poller.tick();
        kit.now.addAndGet(ResourcePackPoller.SETTLE_MILLIS + 1);
        poller.tick(); poller.tick();
        assertEquals(1, kit.platform.packs.size());
        for (int attempt = 1; attempt <= ResourcePackPoller.RETRIES; attempt++) {
            assertTrue(poller.failed(p.id));
            poller.tick(); assertEquals(attempt, kit.platform.packs.size(), "not until it has settled again");
            kit.now.addAndGet(ResourcePackPoller.SETTLE_MILLIS + 1);
            poller.tick(); assertEquals(attempt + 1, kit.platform.packs.size());
        }
        assertFalse(poller.failed(p.id), "then it gives up instead of looping");
    }
}
