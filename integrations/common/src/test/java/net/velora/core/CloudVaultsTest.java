package net.velora.core;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;

import static org.junit.jupiter.api.Assertions.*;

/** Cloud vault sessions against an in-memory panel with the same lease and revision rules as the real one. */
class CloudVaultsTest {
    /** The panel's vault table: contents, revision and lease per vault. */
    static final class Store {
        final Map<String, JsonArray> contents = new HashMap<>();
        final Map<String, Long> revisions = new HashMap<>();
        final Map<String, String> leases = new HashMap<>();
        final Set<Long> mailbox = new HashSet<>();
        final Map<String,JsonObject> receipts = new HashMap<>();
        int saves;

        void install(Kit.FakePanel panel) {
            panel.handlers.put("vault/open", b -> {
                String key = "player:" + b.get("uuid").getAsString(), id = key + "#" + b.get("number").getAsInt();
                if (leases.containsKey(id) && leases.get(id).startsWith("other")) throw new Exception("this vault is open on another server");
                leases.putIfAbsent(id, UUID.randomUUID().toString());
                JsonObject r = new JsonObject();
                r.addProperty("key", key);
                r.addProperty("lease", leases.get(id));
                r.addProperty("revision", revisions.getOrDefault(id, 0L));
                r.addProperty("rows", 1);
                r.add("contents", contents.getOrDefault(id, new JsonArray()).deepCopy());
                return r;
            });
            panel.handlers.put("vault/save", b -> {
                String operation = b.has("operation_id") ? b.get("operation_id").getAsString() : null;
                if (operation != null && receipts.containsKey(operation)) return receipts.get(operation).deepCopy();
                String id = b.get("key").getAsString() + "#" + b.get("number").getAsInt();
                if (!b.get("lease").getAsString().equals(leases.get(id))) throw new Exception("the vault lease expired or the vault changed; reopen it");
                if (b.get("revision").getAsLong() != revisions.getOrDefault(id, 0L)) throw new Exception("the vault lease expired or the vault changed; reopen it");
                if (b.has("delivery_ids")) for (var d : b.getAsJsonArray("delivery_ids")) if (!mailbox.remove(d.getAsLong())) throw new Exception("this delivery was already stored");
                saves++;
                contents.put(id, b.getAsJsonArray("contents").deepCopy());
                revisions.merge(id, 1L, Long::sum);
                if (b.get("release").getAsBoolean()) leases.remove(id);
                JsonObject r = new JsonObject();
                r.addProperty("revision", revisions.get(id));
                if (operation != null) receipts.put(operation,r.deepCopy());
                return r;
            });
            panel.handlers.put("vault/renew", b -> new JsonObject());
        }
    }

    /** A one-row container of item ids; a delivery fits while there is an empty slot. */
    static final class Platform9 extends Kit.FakePlatform {
        final List<String> shown = new ArrayList<>();
        int closedAll, playerSaves;
        JsonArray local;
        boolean migratedFile;
        String checkpoint="";
        final Set<UUID> locked=new HashSet<>();
        @Override public String inventoryCheckpoint(UUID player){return checkpoint;}
        @Override public void lockInventory(UUID player,boolean value){if(value)locked.add(player);else locked.remove(player);}
        final class View implements Platform.VaultView {
            final String[] slots = new String[9];
            Runnable changed;
            java.util.function.Consumer<UUID> closed;
            Platform.VaultTransfer transfer;
            public void transfers(Platform.VaultTransfer handler){transfer=handler;}
            public void replace(JsonArray contents){Arrays.fill(slots,null);for(var row:contents){var s=row.getAsJsonObject();slots[s.get("slot").getAsInt()]=s.get("item").getAsString();}}
            public JsonArray contents() {
                JsonArray out = new JsonArray();
                for (int i = 0; i < 9; i++) if (slots[i] != null) out.add(stack(i, slots[i]));
                return out;
            }
            public boolean deposit(Item item) {
                for (int i = 0; i < 9; i++) if (slots[i] == null) { slots[i] = item.id(); changed.run(); return true; }
                return false;
            }
            public void closeAll() { closedAll++; }
            void put(int slot, String id) { slots[slot] = id; changed.run(); }
        }
        View last;
        @Override public boolean cloudVaults() { return true; }
        @Override public VaultView createVault(String title, int rows, JsonArray contents, Runnable changed, java.util.function.Consumer<UUID> closed, UUID first) {
            View v = new View();
            v.changed = changed; v.closed = closed;
            for (var e : contents) v.slots[e.getAsJsonObject().get("slot").getAsInt()] = e.getAsJsonObject().get("item").getAsString();
            last = v;
            return v;
        }
        @Override public boolean showVault(UUID player, VaultView view) { shown.add(online.get(player).name); return true; }
        @Override public JsonArray vaultInsert(JsonArray contents, int rows, Item item) {
            if (contents.size() >= 9) return null;
            JsonArray out = contents.deepCopy();
            Set<Integer> used = new HashSet<>();
            for (var e : contents) used.add(e.getAsJsonObject().get("slot").getAsInt());
            for (int i = 0; i < 9; i++) if (!used.contains(i)) { out.add(stack(i, item.id())); return out; }
            return null;
        }
        @Override public JsonArray localVaults(UUID owner) { return migratedFile ? null : local; }
        @Override public void localVaultsMigrated(UUID owner) { migratedFile = true; }
        @Override public void savePlayer(UUID player) { playerSaves++; }
    }

    static JsonObject stack(int slot, String id) {
        JsonObject o = new JsonObject();
        o.addProperty("slot", slot);
        o.addProperty("item", id);
        o.addProperty("count", 1);
        o.addProperty("name", id);
        o.addProperty("data", "");
        return o;
    }

    @TempDir Path dir;

    private record Setup(Platform9 platform, Kit.FakePanel panel, Store store, CloudVaults vaults, Env env) {}

    private Setup setup() {
        Platform9 platform = new Platform9();
        Kit.FakePanel panel = new Kit.FakePanel();
        Store store = new Store();
        store.install(panel);
        panel.on("vault/import", "{\"imported\":true,\"migrated\":true}");
        Env env = new Env(platform, panel, Features.all(), dir, () -> 1_000L, new Random(1), java.util.logging.Logger.getLogger("test"));
        return new Setup(platform, panel, store, new CloudVaults(env), env);
    }

    @Test void committedTransferRetriesLostAcknowledgmentWithoutApplyingInventoryTwice(){
        Setup s=setup();Kit.Player player=s.platform.add("Checkpoint");s.vaults.open(player,"player",1,"Vault");
        java.util.concurrent.atomic.AtomicInteger applied=new java.util.concurrent.atomic.AtomicInteger(),finished=new java.util.concurrent.atomic.AtomicInteger();
        s.panel.on("vault/transfers/prepare","{\"status\":\"prepared\"}");
        s.panel.handlers.put("vault/transfers/finish",body->{assertTrue(body.get("commit").getAsBoolean());if(finished.incrementAndGet()==1)throw new java.io.IOException("lost response");return com.google.gson.JsonParser.parseString("{\"revision\":1}");});
        JsonArray after=new JsonArray();after.add(stack(0,"minecraft:diamond"));
        s.platform.last.transfer.begin(player.uuid(),after,id->{applied.incrementAndGet();s.platform.checkpoint=id;return true;},ok->assertTrue(ok));
        assertTrue(s.platform.locked.contains(player.uuid()),"lost acknowledgment keeps custody locked");
        s.vaults.tick();assertEquals(1,applied.get(),"retry never reapplies the player inventory mutation");assertEquals(2,finished.get());assertFalse(s.platform.locked.contains(player.uuid()));
        assertEquals("minecraft:diamond",s.platform.last.slots[0]);
    }

    @Test void reconnectResolvesPreparedCustodyUsingOnlyTheDurableCheckpoint(){
        for(boolean persisted:List.of(false,true)){
            Setup s=setup();Kit.Player player=s.platform.add("Reconnect");String id=UUID.randomUUID().toString();s.platform.checkpoint=persisted?id:"";
            s.panel.on("vault/transfers/pending","{\"pending\":[\""+id+"\"]}");
            s.panel.handlers.put("vault/transfers/finish",body->{assertEquals(persisted,body.get("commit").getAsBoolean());assertEquals(id,body.get("id").getAsString());return new JsonObject();});
            s.vaults.recoverPlayer(player.uuid());assertFalse(s.platform.locked.contains(player.uuid()));
        }
    }

    @Test void editsAreSavedAndTheLeaseIsReleasedWhenTheLastViewerCloses() {
        Setup s = setup();
        Kit.Player alex = s.platform.add("Alex");
        s.vaults.open(alex, "player", 1, "Vault 1");
        assertEquals(List.of("Alex"), s.platform.shown);
        String id = "player:" + alex.uuid() + "#1";
        assertTrue(s.store.leases.containsKey(id));
        s.platform.last.put(3, "minecraft:diamond");
        s.vaults.tick();
        assertEquals(1, s.store.saves);
        assertEquals("minecraft:diamond", s.store.contents.get(id).get(0).getAsJsonObject().get("item").getAsString());
        assertTrue(s.store.leases.containsKey(id), "still open: still leased");
        assertTrue(s.platform.playerSaves > 0, "the player's own data is saved right after the vault");
        s.platform.last.closed.accept(alex.uuid());
        s.vaults.tick();
        assertFalse(s.store.leases.containsKey(id), "released after the last viewer closed");
        assertEquals(0, s.vaults.openCount());
    }

    @Test void lostSaveResponseRetriesTheExactRequestBeforeSavingLaterEdits() {
        Setup s=setup(); Kit.Player alex=s.platform.add("SyntheticRetry");
        s.vaults.open(alex,"player",1,"Vault 1");
        var save=s.panel.handlers.get("vault/save");
        java.util.concurrent.atomic.AtomicBoolean lost=new java.util.concurrent.atomic.AtomicBoolean();
        s.panel.handlers.put("vault/save",body->{var result=save.handle(body);if(!lost.getAndSet(true))throw new java.io.IOException("response lost");return result;});
        s.platform.last.put(0,"minecraft:diamond"); s.vaults.tick();
        s.platform.last.put(1,"minecraft:emerald"); s.vaults.tick();
        assertEquals(1,s.store.saves,"receipt retry did not write a second revision");
        var requests=s.panel.bodies.stream().filter(b->b.has("operation_id")).toList();
        assertEquals(requests.get(0),requests.get(1),"retry carries the same contents, lease, revision and identity");
        s.vaults.tick();
        assertEquals(2,s.store.saves,"new edits receive their own save after recovery");
        assertEquals(2,s.store.contents.get("player:"+alex.uuid()+"#1").size());
        assertEquals(0,s.platform.closedAll,"lost acknowledgment did not force manual recovery");
    }

    @Test void deliveriesIntoAnOpenVaultAreAcknowledgedByItsSaveAndOfflineOnesByTheirOwnWrite() {
        Setup s = setup();
        Kit.Player alex = s.platform.add("Alex");
        s.vaults.open(alex, "player", 1, "Vault 1");
        s.store.mailbox.add(7L);
        List<Boolean> results = new ArrayList<>();
        s.vaults.deliver(alex.uuid(), 7, new Item("minecraft:emerald", "Emerald", 1, ""), 1, results::add);
        assertEquals(List.of(true), results);
        assertTrue(s.store.mailbox.contains(7L), "not acknowledged until the vault is saved");
        s.vaults.tick();
        assertFalse(s.store.mailbox.contains(7L), "acknowledged with the save that stored it");

        UUID offline = UUID.randomUUID();
        s.store.mailbox.add(8L);
        s.vaults.deliver(offline, 8, new Item("minecraft:gold_ingot", "Gold", 1, ""), 2, results::add);
        assertEquals(List.of(true, true), results);
        assertFalse(s.store.mailbox.contains(8L));
        assertEquals(1, s.store.contents.get("player:" + offline + "#1").size());
        assertFalse(s.store.leases.containsKey("player:" + offline + "#1"), "offline vaults are released after the delivery");
    }

    @Test void aFullVaultReportsNoRoomAndALostLeaseNeverOverwritesNewerContents() throws Exception {
        Setup s = setup();
        UUID offline = UUID.randomUUID();
        JsonArray full = new JsonArray();
        for (int i = 0; i < 9; i++) full.add(stack(i, "minecraft:stone"));
        s.store.contents.put("player:" + offline + "#1", full);
        s.store.mailbox.add(9L);
        List<Boolean> results = new ArrayList<>();
        s.vaults.deliver(offline, 9, new Item("minecraft:emerald", "Emerald", 1, ""), 1, results::add);
        assertEquals(List.of(false), results, "no room: the panel moves it to the mailbox");
        assertTrue(s.store.mailbox.contains(9L));

        Kit.Player alex = s.platform.add("Alex");
        s.vaults.open(alex, "player", 1, "Vault 1");
        String id = "player:" + alex.uuid() + "#1";
        // Another writer changed the vault after this server lost its lease.
        s.store.leases.put(id, "stolen");
        s.store.revisions.put(id, 5L);
        s.store.leases.remove(id);
        s.platform.last.put(0, "minecraft:netherite_ingot");
        s.vaults.tick();
        assertEquals(1, s.platform.closedAll, "the live copy is closed instead of overwriting newer contents");
        assertTrue(alex.heard("staff have a recovery copy"));
        try (var files = Files.list(dir.resolve("vault-recovery"))) {
            String saved = Files.readString(files.findFirst().orElseThrow());
            assertTrue(saved.contains("netherite_ingot"), saved);
        }
    }

    @Test void localVaultFilesAreImportedOnceBeforeTheFirstOpen() {
        Setup s = setup();
        Kit.Player alex = s.platform.add("Alex");
        s.platform.local = new JsonArray();
        s.vaults.open(alex, "player", 1, "Vault 1");
        assertEquals(List.of("vault/import", "vault/open"), s.panel.calls.subList(0, 2));
        assertTrue(s.platform.migratedFile);
        s.platform.last.closed.accept(alex.uuid());
        s.vaults.tick();
        s.panel.calls.clear();
        s.vaults.open(alex, "player", 1, "Vault 1");
        assertFalse(s.panel.calls.contains("vault/import"));
    }

    @Test void shutdownSavesAndReleasesOpenVaults() {
        Setup s = setup();
        Kit.Player alex = s.platform.add("Alex");
        s.vaults.open(alex, "player", 2, "Vault 2");
        s.platform.last.put(1, "minecraft:apple");
        s.vaults.shutdown();
        String id = "player:" + alex.uuid() + "#2";
        assertFalse(s.store.leases.containsKey(id));
        assertEquals(1, s.store.contents.get(id).size());
    }
}
