package net.scopenet.core;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.scopenet.integration.ChunkCheckResult;

import java.nio.file.Path;
import java.util.*;
import java.util.concurrent.atomic.AtomicLong;
import java.util.logging.Logger;

/** Fakes for the shared-core tests: players, a platform that runs everything inline, and a scriptable panel. */
final class Kit {
    static final class Player implements CorePlayer {
        final UUID id = UUID.randomUUID();
        final String name;
        Pos pos = new Pos("minecraft:overworld", 0, 64, 0, 0, 0);
        final List<String> inbox = new ArrayList<>();
        final Set<String> denied = new HashSet<>();
        /** Nodes that grant extras (limit tiers, cooldown bypass) are off unless given here. */
        final Set<String> granted = new HashSet<>();
        Item held;
        final List<Item> inventory = new ArrayList<>();
        long playtime = 3700;
        Player(String name) { this.name = name; }
        public UUID uuid() { return id; }
        public String name() { return name; }
        public Pos pos() { return pos; }
        public void send(String m) { inbox.add(Format.plain(m)); }
        public void teleport(Pos p) { pos = p; }
        public boolean hasPermission(String n) { return granted.contains(n) || (!n.startsWith("scopenet.homes.") && !n.equals("scopenet.cooldown.bypass") && !denied.contains(n)); }
        public long playtimeSeconds() { return playtime; }
        public Optional<Item> heldItem() { return Optional.ofNullable(held); }
        public void clearHeld() { held = null; }
        public void give(Item i) { inventory.add(i); }
        /** Plain stacks (empty data) of one item can be taken from the inventory; stacks with data are left alone. */
        public List<Item> takeItems(String id, int max) {
            List<Item> taken = new ArrayList<>();
            int left = max;
            for (Iterator<Item> it = inventory.iterator(); it.hasNext() && left > 0; ) {
                Item i = it.next();
                if (!i.id().equalsIgnoreCase(id) || !i.data().isEmpty()) continue;
                int n = Math.min(left, i.count());
                taken.add(new Item(i.id(), i.name(), n, ""));
                left -= n;
                it.remove();
                if (n < i.count()) inventory.add(new Item(i.id(), i.name(), i.count() - n, ""));
            }
            return taken;
        }
        String last() { return inbox.get(inbox.size() - 1); }
        boolean heard(String text) { return inbox.stream().anyMatch(s -> s.contains(text)); }
    }

    static class FakePlatform implements Platform {
        final Map<UUID, Player> online = new LinkedHashMap<>();
        boolean safe = true;
        boolean outpostPlaceable;
        final List<Pos> outpostFlags = new ArrayList<>();
        public boolean canPlaceOutpostFlag(Pos p) { return outpostPlaceable; }
        public boolean placeOutpostFlag(Pos p) { if (!outpostPlaceable) return false; outpostFlags.add(p); return true; }
        Player add(String name) { Player p = new Player(name); online.put(p.id, p); return p; }
        public Optional<CorePlayer> player(UUID u) { return Optional.ofNullable(online.get(u)); }
        public Optional<CorePlayer> playerByName(String n) { return online.values().stream().filter(p -> p.name.equalsIgnoreCase(n)).map(p -> (CorePlayer) p).findFirst(); }
        public Collection<? extends CorePlayer> online() { return online.values(); }
        public Optional<Pos> safeSurface(String w, int x, int z) { return safe ? Optional.of(new Pos(w, x, 70, z, 0, 0)) : Optional.empty(); }
        public Pos worldSpawn(String w) { return new Pos(w, 0, 80, 0, 0, 0); }
        public void runMain(Runnable r) { r.run(); }
        public void runAsync(Runnable r) { r.run(); }

        // What the utility commands asked the platform to do.
        final List<String> healed = new ArrayList<>(), fed = new ArrayList<>(), vaults = new ArrayList<>(), bars = new ArrayList<>(), console = new ArrayList<>();
        final List<ItemSpec> given = new ArrayList<>();
        final Map<UUID, Boolean> flight = new HashMap<>();
        int enderOpened;
        final List<String> deposits = new ArrayList<>();
        boolean vaultHasRoom = true;
        public boolean depositToVault(UUID p, Item item, int vaults, int rows) { if (!vaultHasRoom) return false; deposits.add(p + ":" + item.count() + "x" + item.id() + "/" + vaults + "x" + rows); return true; }
        com.google.gson.JsonArray snapshot = new com.google.gson.JsonArray();
        final List<String> packs = new ArrayList<>();
        public com.google.gson.JsonArray inventorySnapshot(UUID p) { return snapshot.deepCopy(); }
        public boolean offerResourcePack(UUID p, String url, String hash, boolean required) { packs.add(p + ":" + hash + ":" + required); return true; }
        public boolean giveSpec(UUID p, ItemSpec s) { given.add(s); return true; }
        public void heal(UUID p, double a) { healed.add(online.get(p).name + ":" + a); }
        public void feed(UUID p, int a) { fed.add(online.get(p).name + ":" + a); }
        public void setFlight(UUID p, boolean on) { flight.put(p, on); }
        public void openVault(UUID p, int n, int rows) { vaults.add(online.get(p).name + ":" + n + "x" + rows); }
        public void openEnderChest(UUID p) { enderOpened++; }
        public void actionbar(UUID p, String t) { bars.add(Format.plain(t)); }
        public boolean console(String c) { console.add(c); return true; }
    }

    interface Handler { JsonElement handle(JsonObject body) throws Exception; }

    static final class FakePanel implements Panel {
        final Map<String, Handler> handlers = new HashMap<>();
        final List<String> calls = new ArrayList<>();
        final List<JsonObject> bodies = new ArrayList<>();
        final Map<String, ChunkCheckResult> claims = new HashMap<>();
        boolean claimsRefreshed;
        public JsonElement call(String endpoint, JsonObject body) throws Exception {
            calls.add(endpoint);
            bodies.add(body);
            Handler h = handlers.get(endpoint);
            if (h == null) throw new net.scopenet.integration.PanelClient.HttpFailure(404, "no handler for " + endpoint);
            return h.handle(body);
        }
        public void claimsChanged() { claimsRefreshed = true; }
        public ChunkCheckResult check(String dim, int cx, int cz, UUID p) {
            return claims.getOrDefault(dim + ":" + cx + ":" + cz, new ChunkCheckResult(false, true, null, null));
        }
        final Map<String, net.scopenet.integration.ClaimIndex.ClaimInfo> infos = new HashMap<>();
        final Set<String> grounded = new HashSet<>();
        public boolean adminAllows(String dim, int cx, int cz, String flag) { return !(flag.equals("fly") && grounded.contains(dim + ":" + cx + ":" + cz)); }
        public net.scopenet.integration.ClaimIndex.ClaimInfo info(String dim, int cx, int cz) { return infos.get(dim + ":" + cx + ":" + cz); }
        void on(String endpoint, String json) { handlers.put(endpoint, b -> com.google.gson.JsonParser.parseString(json)); }
    }

    final FakePlatform platform = new FakePlatform();
    final FakePanel panel = new FakePanel();
    final AtomicLong now = new AtomicLong(1_000);
    final Env env;

    Kit(Path dir) {
        env = new Env(platform, panel, Features.all(), dir, now::get, new Random(1), Logger.getLogger("test"));
    }

    static CoreCommand find(List<CoreCommand> all, String name) {
        return all.stream().filter(c -> c.name().equals(name)).findFirst().orElseThrow(() -> new AssertionError("no command " + name));
    }
}
