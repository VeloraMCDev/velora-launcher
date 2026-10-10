package net.velora.core;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.util.*;
import java.util.concurrent.atomic.AtomicLong;

import static org.junit.jupiter.api.Assertions.*;

class EssentialsServiceTest {
    static final class Fake implements CorePlayer {
        final UUID id = UUID.randomUUID();
        final String name;
        Pos pos = new Pos("overworld", 0, 64, 0, 0, 0);
        final List<String> inbox = new ArrayList<>();
        Fake(String name) { this.name = name; }
        public UUID uuid() { return id; }
        public String name() { return name; }
        public Pos pos() { return pos; }
        public void send(String m) { inbox.add(m.replaceAll("§.", "")); }
        public void teleport(Pos p) { pos = p; }
        public boolean hasPermission(String n) { return !n.startsWith("velora.homes.") && !n.equals("velora.cooldown.bypass"); }
        public long playtimeSeconds() { return 0; }
        public java.util.Optional<Item> heldItem() { return java.util.Optional.empty(); }
        public void clearHeld() { }
        public void give(Item i) { }
        String last() { return inbox.get(inbox.size() - 1); }
    }

    final Map<UUID, Fake> online = new HashMap<>();
    final AtomicLong now = new AtomicLong(1_000);
    boolean safe = true;
    @TempDir Path dir;
    Platform platform = new Platform() {
        public Optional<CorePlayer> player(UUID u) { return Optional.ofNullable(online.get(u)); }
        public Optional<CorePlayer> playerByName(String n) { return online.values().stream().filter(p -> p.name.equalsIgnoreCase(n)).map(p -> (CorePlayer) p).findFirst(); }
        public Optional<Pos> safeSurface(String w, int x, int z) { return safe ? Optional.of(new Pos(w, x, 70, z, 0, 0)) : Optional.empty(); }
        public Pos worldSpawn(String w) { return new Pos(w, 0, 80, 0, 0, 0); }
        public java.util.Collection<? extends CorePlayer> online() { return online.values(); }
        public void runMain(Runnable r) { r.run(); }
        public void runAsync(Runnable r) { r.run(); }
    };
    Fake alex, steve;
    EssentialsService svc;

    @BeforeEach void setup() {
        alex = new Fake("Alex"); steve = new Fake("Steve");
        online.put(alex.id, alex); online.put(steve.id, steve);
        svc = new EssentialsService(platform, dir.resolve("essentials.json"), EssentialsConfig.defaults(), now::get, new Random(1));
    }

    @Test void homesAreLimitedAndPersisted() {
        svc.configure(new EssentialsConfig(2, 2500, 60_000, 15));
        alex.pos = new Pos("overworld", 10, 64, 20, 90, 0);
        svc.setHome(alex, new String[]{"Base"});
        svc.setHome(alex, new String[]{"mine"});
        svc.setHome(alex, new String[]{"third"});
        assertTrue(alex.last().contains("limit of 2"));
        svc.setHome(alex, new String[]{"base"});   // overwriting is allowed at the limit
        assertEquals(List.of("base", "mine"), svc.homeNames(alex.id));

        EssentialsService reloaded = new EssentialsService(platform, dir.resolve("essentials.json"), EssentialsConfig.defaults(), now::get, new Random(1));
        assertEquals(10, reloaded.homeAt(alex.id, "BASE").orElseThrow().x());
        alex.pos = new Pos("overworld", 0, 0, 0, 0, 0);
        assertEquals(List.of("base", "mine"), reloaded.home(alex, new String[0]), "several homes need a choice");
        reloaded.home(alex, new String[]{"base"});
        assertEquals(10, alex.pos.x());
    }

    @Test void singleHomeTeleportsDirectlyAndDeleteWorks() {
        svc.home(alex, new String[0]);
        assertTrue(alex.last().contains("no homes"));
        svc.setHome(alex, new String[0]);
        alex.pos = new Pos("overworld", 5, 5, 5, 0, 0);
        assertTrue(svc.home(alex, new String[0]).isEmpty());
        assertEquals(0, alex.pos.x());
        svc.delHome(alex, new String[]{"home"});
        svc.delHome(alex, new String[]{"home"});
        assertTrue(alex.last().contains("does not exist"));
    }

    @Test void warpsSeedSpawnAndTeleport() {
        assertEquals(List.of("spawn"), svc.warp(alex, new String[0]));
        svc.warp(alex, new String[]{"spawn"});
        assertEquals(80, alex.pos.y());
        svc.warp(alex, new String[]{"nope"});
        assertTrue(alex.last().contains("does not exist"));
        svc.setWarp(alex, "Arena");
        assertTrue(svc.warpAt("arena").isPresent());
        svc.delWarp(alex, "arena");
        assertTrue(svc.warpAt("arena").isEmpty());
    }

    @Test void backReturnsToWhereYouWere() {
        svc.back(alex);
        assertTrue(alex.last().contains("No previous"));
        alex.pos = new Pos("overworld", 1, 2, 3, 0, 0);
        svc.setWarp(steve, "x"); // unrelated
        svc.warp(alex, new String[]{"x"});
        svc.back(alex);
        assertEquals(1, alex.pos.x());
        svc.remember(alex.id, new Pos("overworld", 9, 9, 9, 0, 0)); // death point
        svc.back(alex);
        assertEquals(9, alex.pos.x());
    }

    @Test void tpaFlowAndExpiry() {
        steve.pos = new Pos("overworld", 100, 64, 100, 0, 0);
        svc.tpa(alex, new String[]{"steve"});
        assertTrue(steve.last().contains("expires in 60s"));
        svc.tpAccept(steve);
        assertEquals(100, alex.pos.x());

        svc.tpa(alex, new String[]{"steve"});
        now.addAndGet(61_000);
        svc.tpAccept(steve);
        assertTrue(steve.last().contains("no active"));

        svc.tpa(alex, new String[]{"alex"});
        assertTrue(alex.last().contains("yourself"));
        svc.tpa(alex, new String[]{"ghost"});
        assertTrue(alex.last().contains("not online"));
        svc.tpa(alex, new String[]{"steve"});
        svc.tpDeny(steve);
        assertTrue(alex.last().contains("denied"));
        svc.tpDeny(steve);
        assertTrue(steve.last().contains("no active"));
    }

    @Test void rtpStaysInRadiusAndFailsGracefully() {
        svc.configure(new EssentialsConfig(5, 100, 60_000, 15));
        svc.rtp(alex);
        assertTrue(Math.abs(alex.pos.x()) <= 101 && Math.abs(alex.pos.z()) <= 101);
        assertEquals(70, alex.pos.y());
        Pos before = alex.pos;
        safe = false;
        svc.rtp(alex);
        assertEquals(before, alex.pos);
        assertTrue(alex.last().contains("Could not find"));
    }
}
