package net.scopenet.core;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.util.Map;
import java.util.Set;

import static org.junit.jupiter.api.Assertions.*;

class LimitsTest {
    @Test void permissionTiersRaiseALimit() {
        assertEquals(5, Limits.tier(n -> false, "scopenet.homes.", 5, 500));
        assertEquals(10, Limits.tier(Set.of("scopenet.homes.10")::contains, "scopenet.homes.", 5, 500));
        assertEquals(25, Limits.tier(Set.of("scopenet.homes.10", "scopenet.homes.25", "scopenet.homes.3")::contains, "scopenet.homes.", 5, 500), "highest wins");
        assertEquals(5, Limits.tier(Set.of("scopenet.homes.3")::contains, "scopenet.homes.", 5, 500), "a lower tier never reduces the base");
        assertEquals(Integer.MAX_VALUE, Limits.tier(Set.of("scopenet.homes.unlimited")::contains, "scopenet.homes.", 5, 500));
        assertEquals("unlimited", Limits.show(Integer.MAX_VALUE));
    }

    @Test void homesFollowConfigAndPermissions(@TempDir Path dir) {
        Kit kit = new Kit(dir);
        Kit.Player steve = kit.platform.add("Steve");
        EssentialsConfig cfg = new EssentialsConfig(2, 2500, 60_000, 15, 0, 1, 8, Map.of());
        EssentialsService svc = new EssentialsService(kit.platform, dir.resolve("e.json"), cfg, kit.now::get, kit.env.rng);
        svc.setHome(steve, new String[]{"a"});
        svc.setHome(steve, new String[]{"b"});
        svc.setHome(steve, new String[]{"c"});
        assertTrue(steve.last().contains("limit of 2"));
        steve.granted.add("scopenet.homes.4");
        svc.setHome(steve, new String[]{"c"});
        svc.setHome(steve, new String[]{"d"});
        svc.setHome(steve, new String[]{"e"});
        assertTrue(steve.last().contains("limit of 4"), steve.last());
        svc.setHome(steve, new String[]{"waytoolongname"});
        assertTrue(steve.last().contains("at most 8"), steve.last());
        svc.setWarp(steve, "spawn");
        svc.setWarp(steve, "pvp");
        assertTrue(steve.last().contains("already has 1 warps"), steve.last());
    }

    @Test void teleportCommandsCoolDownOnlyAfterTeleporting(@TempDir Path dir) {
        Kit kit = new Kit(dir);
        Kit.Player steve = kit.platform.add("Steve");
        EssentialsConfig cfg = new EssentialsConfig(5, 2500, 60_000, 15, 0, 0, 32, Map.of("home", 30, "warp", 10));
        CommandSet set = new CommandSet(kit.env, cfg);
        set.essentials.setHome(steve, new String[]{"base"});
        set.run("home", steve, "nope");                       // unknown home: nothing happened, no cooldown
        set.run("home", steve, "base");
        assertTrue(steve.last().contains("Teleported to home"), steve.last());
        set.run("home", steve, "base");
        assertTrue(steve.last().contains("Please wait 30s"), steve.last());
        kit.now.addAndGet(31_000);
        set.run("home", steve, "base");
        assertTrue(steve.last().contains("Teleported to home"), steve.last());
        set.run("warp", steve, "missing");
        set.run("warp", steve, "missing");
        assertFalse(steve.last().contains("Please wait"), "a failed warp costs nothing");
        steve.granted.add("scopenet.cooldown.bypass");
        set.run("home", steve, "base");
        assertTrue(steve.last().contains("Teleported to home"), "bypass skips cooldowns");
    }

    @Test void rtpKeepsAwayFromTheCentre(@TempDir Path dir) {
        Kit kit = new Kit(dir);
        Kit.Player steve = kit.platform.add("Steve");
        EssentialsConfig cfg = new EssentialsConfig(5, 400, 60_000, 50, 300, 0, 32, Map.of());
        EssentialsService svc = new EssentialsService(kit.platform, dir.resolve("e.json"), cfg, kit.now::get, new java.util.Random(7));
        for (int i = 0; i < 20; i++) {
            svc.rtp(steve);
            int x = (int) steve.pos.x(), z = (int) steve.pos.z();
            assertTrue(Math.max(Math.abs(x), Math.abs(z)) >= 300, "landed at " + x + "," + z);
        }
    }
}
