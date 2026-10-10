package net.scopenet.core;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.UUID;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.TimeUnit;
import java.util.function.Supplier;

/**
 * Vaults stored in the panel. Opening one leases it to this server; the live container is saved within a second of every
 * change, renewed while open and released when the last viewer closes it. Players on this server who open the same vault
 * share one container. A delivery is acknowledged by the same panel write that stores it, so it cannot arrive twice.
 * If a lease is lost and the vault changed elsewhere, the live copy is written to a recovery file for staff instead of
 * overwriting anything. Server thread only, except where noted.
 */
public final class CloudVaults {
    static final long RENEW_MS = 30_000;

    private final Env env;
    private final Map<String, Session> sessions = new HashMap<>();
    private final Set<String> opening = new HashSet<>();
    /** Vaults an offline delivery is filling right now ("player:uuid#n"); touched from the I/O pool too. */
    private final Set<String> busy = ConcurrentHashMap.newKeySet();
    /** Players whose local vault file has been dealt with this run. */
    private final Set<UUID> migrated = ConcurrentHashMap.newKeySet();

    static final class Session {
        final String key;
        final int number;
        final int rows;
        String lease;
        long revision;
        Platform.VaultView view;
        final Set<UUID> viewers = new HashSet<>();
        final Set<UUID> everyone = new HashSet<>();
        final List<Long> deliveries = new ArrayList<>();
        boolean dirty, saving, reacquiring;
        long renewedAt;
        int failures;
        JsonObject pendingSave;
        Transfer transfer;

        Session(String key, int number, int rows, String lease, long revision) {
            this.key = key; this.number = number; this.rows = rows; this.lease = lease; this.revision = revision;
        }

        String id() { return key + "#" + number; }
    }

    public CloudVaults(Env env) { this.env = env; }
    private static final class Transfer {
        final UUID actor; final JsonObject prepare;
        final java.util.function.Function<String,Boolean> apply;
        final java.util.function.Consumer<Boolean> done;
        Boolean commit;
        boolean sending;
        Transfer(UUID actor,JsonObject prepare,java.util.function.Function<String,Boolean> apply,java.util.function.Consumer<Boolean> done){this.actor=actor;this.prepare=prepare;this.apply=apply;this.done=done;}
    }

    public boolean supported() { return env.platform.cloudVaults(); }

    /** Open one vault: {@code owner} is "player" or "faction". */
    public void open(CorePlayer p, String owner, int number, String title) {
        String ask = owner + ":" + p.uuid() + ":" + number;
        if (owner.equals("player") && busy.contains("player:" + p.uuid() + "#" + number)) {
            p.send(Format.YELLOW + "A delivery is arriving in that vault; try again in a moment.");
            return;
        }
        if (!opening.add(ask)) return;
        JsonObject body = new JsonObject();
        body.addProperty("uuid", p.uuid().toString());
        body.addProperty("owner", owner);
        body.addProperty("number", number);
        body.addProperty("permitted", owner.equals("player") && (p.hasPermission("scopenet.vault." + number) || p.hasPermission("scopenet.vault.*")));
        env.io(() -> {
            if (owner.equals("player")) migrate(p.uuid());
            return env.panel.call("vault/open", body).getAsJsonObject();
        }, r -> {
            opening.remove(ask);
            String key = r.get("key").getAsString();
            Session s = sessions.get(key + "#" + number);
            if (s == null) {
                Session fresh = new Session(key, number, r.get("rows").getAsInt(), r.get("lease").getAsString(), r.get("revision").getAsLong());
                fresh.view = env.platform.createVault(title, fresh.rows, r.getAsJsonArray("contents"), () -> fresh.dirty = true, who -> closed(fresh, who), p.uuid());
                if (fresh.view == null) { p.send(Format.RED + "Vaults are not available on this server."); release(fresh); return; }
                fresh.view.transfers((actor,after,apply,done)->beginTransfer(fresh,actor,after,apply,done));
                fresh.renewedAt = env.clock.getAsLong();
                sessions.put(fresh.id(), fresh);
                s = fresh;
            }
            if (!env.platform.showVault(p.uuid(), s.view)) {
                p.send(Format.RED + "Velora Core Client is required to open a seven-row vault.");
                return;
            }
            s.viewers.add(p.uuid());
            s.everyone.add(p.uuid());
        }, e -> { opening.remove(ask); p.send(Format.RED + e); });
    }

    private void closed(Session s, UUID who) {
        s.viewers.remove(who);
        if (s.viewers.isEmpty()) s.dirty = true;
    }

    /** About once a second: save changed vaults, release closed ones and renew leases. */
    public void tick() {
        long now = env.clock.getAsLong();
        for (Session s : new ArrayList<>(sessions.values())) {
            if(s.transfer!=null){driveTransfer(s);continue;}
            if (s.saving || s.reacquiring) continue;
            boolean release = s.viewers.isEmpty();
            if (s.dirty || release || !s.deliveries.isEmpty()) save(s, release);
            else if (now - s.renewedAt >= RENEW_MS) renew(s);
        }
    }

    private void beginTransfer(Session s,UUID actor,JsonArray after,java.util.function.Function<String,Boolean> apply,java.util.function.Consumer<Boolean> done) {
        if(s.saving||s.reacquiring||s.dirty||s.pendingSave!=null||s.transfer!=null||!s.deliveries.isEmpty()){done.accept(false);return;}
        JsonObject body=new JsonObject();body.addProperty("id",UUID.randomUUID().toString());body.addProperty("uuid",actor.toString());
        body.addProperty("key",s.key);body.addProperty("number",s.number);body.addProperty("revision",s.revision);body.addProperty("lease",s.lease);body.add("contents",after.deepCopy());
        body.addProperty("permitted",s.key.startsWith("player:")&&env.platform.player(actor).isPresent()&&(env.platform.player(actor).get().hasPermission("scopenet.vault."+s.number)||env.platform.player(actor).get().hasPermission("scopenet.vault.*")));
        s.transfer=new Transfer(actor,body,apply,done);env.platform.lockInventory(actor,true);driveTransfer(s);
    }
    private void driveTransfer(Session s) {
        Transfer t=s.transfer;if(t==null||t.sending)return;t.sending=true;
        if(t.commit==null) {
            env.io(()->env.panel.call("vault/transfers/prepare",t.prepare),response->{
                t.sending=false;
                try {t.commit=java.util.Objects.requireNonNull(t.apply.apply(t.prepare.get("id").getAsString()),"ambiguous inventory checkpoint");}
                catch(RuntimeException uncertain){
                    sessions.remove(s.id());s.view.closeAll();
                    env.log.severe("Inventory checkpoint requires reconnect recovery: "+uncertain.getMessage());
                    // Keep the inventory locked. Login recovery reads the persisted checkpoint before resolving custody.
                    return;
                }
                driveTransfer(s);
            },error->{t.sending=false;if(error.contains("lease")||error.contains("owner")||error.contains("identity")||error.contains("disabled")){
                // A previous prepare may have succeeded despite a lost response. Resolve custody before unlocking.
                sessions.remove(s.id());s.view.closeAll();recoverPlayer(t.actor);
            }});
        } else {
            JsonObject body=new JsonObject();body.addProperty("id",t.prepare.get("id").getAsString());body.addProperty("uuid",t.actor.toString());body.addProperty("commit",t.commit);
            env.io(()->env.panel.call("vault/transfers/finish",body).getAsJsonObject(),response->{
                s.revision=response.get("revision").getAsLong();
                if(t.commit)s.view.replace(t.prepare.getAsJsonArray("contents"));
                s.dirty=s.viewers.isEmpty();s.renewedAt=env.clock.getAsLong();s.transfer=null;
                env.platform.lockInventory(t.actor,false);t.done.accept(t.commit);
            },error->{t.sending=false;});
        }
    }
    /** Called before inventory interaction on login. The checkpoint and the prepared custody row determine recovery. */
    public void recoverPlayer(UUID player) {
        env.platform.lockInventory(player,true);
        env.io(()->{
            JsonObject body=new JsonObject();body.addProperty("uuid",player.toString());
            JsonArray pending=env.panel.call("vault/transfers/pending",body).getAsJsonObject().getAsJsonArray("pending");
            String checkpoint=onMain(()->env.platform.inventoryCheckpoint(player));
            for(var id:pending){JsonObject finish=new JsonObject();finish.addProperty("uuid",player.toString());finish.addProperty("id",id.getAsString());finish.addProperty("commit",id.getAsString().equals(checkpoint));env.panel.call("vault/transfers/finish",finish);}
            return true;
        },ok->env.platform.lockInventory(player,false),error->{
            env.platform.player(player).ifPresent(p->p.send(Format.RED+"Inventory recovery is waiting for the panel. Reconnect to retry; your items remain protected."));
        });
    }

    private JsonObject saveBody(Session s, JsonArray contents, List<Long> deliveries, boolean release) {
        JsonObject body = new JsonObject();
        body.addProperty("operation_id", UUID.randomUUID().toString());
        body.addProperty("key", s.key);
        body.addProperty("number", s.number);
        body.addProperty("lease", s.lease);
        body.addProperty("revision", s.revision);
        body.add("contents", contents);
        body.addProperty("release", release);
        if (!deliveries.isEmpty()) {
            JsonArray ids = new JsonArray();
            deliveries.forEach(ids::add);
            body.add("delivery_ids", ids);
        }
        return body;
    }

    private void save(Session s, boolean release) {
        s.saving = true;
        if (s.pendingSave == null) {
            s.dirty = false;
            s.pendingSave = saveBody(s, s.view.contents(), new ArrayList<>(s.deliveries), release);
        }
        JsonObject body = s.pendingSave;
        boolean releasing = body.get("release").getAsBoolean();
        List<Long> deliveries = new ArrayList<>();
        if (body.has("delivery_ids")) body.getAsJsonArray("delivery_ids").forEach(id -> deliveries.add(id.getAsLong()));
        env.io(() -> env.panel.call("vault/save", body).getAsJsonObject(), r -> {
            s.pendingSave = null;
            s.saving = false;
            s.failures = 0;
            s.revision = r.get("revision").getAsLong();
            s.renewedAt = env.clock.getAsLong();
            s.deliveries.removeAll(deliveries);
            for (UUID player : s.everyone) env.platform.savePlayer(player);
            if (releasing) {
                if (s.viewers.isEmpty() && !s.dirty && s.deliveries.isEmpty()) sessions.remove(s.id());
                else { s.lease = null; reacquire(s); } // someone opened it again while it was being released
            }
        }, e -> {
            s.saving = false;
            s.dirty = true;
            s.failures++;
            if (e.contains("lease") || e.contains("changed") || e.contains("identity")) {
                s.pendingSave = null;
                reacquire(s);
            }
        });
    }

    private void renew(Session s) {
        JsonObject body = new JsonObject();
        body.addProperty("key", s.key);
        body.addProperty("number", s.number);
        body.addProperty("lease", s.lease);
        s.renewedAt = env.clock.getAsLong();
        env.io(() -> env.panel.call("vault/renew", body), r -> { }, e -> { if (e.contains("lease")) reacquire(s); });
    }

    /** Take the lease again. Safe only when nobody changed the vault meanwhile; otherwise keep the live copy for staff. */
    private void reacquire(Session s) {
        if (s.reacquiring) return;
        s.reacquiring = true;
        UUID who = s.everyone.isEmpty() ? ownerOf(s.key) : s.everyone.iterator().next();
        if (who == null) { recover(s, "no player to authorize the vault"); return; }
        JsonObject body = new JsonObject();
        body.addProperty("uuid", who.toString());
        body.addProperty("owner", s.key.startsWith("shop:") ? s.key : s.key.startsWith("faction:") ? "faction" : "player");
        body.addProperty("number", s.number);
        body.addProperty("permitted", true);
        env.io(() -> env.panel.call("vault/open", body).getAsJsonObject(), r -> {
            s.reacquiring = false;
            if (r.get("revision").getAsLong() != s.revision || !r.get("key").getAsString().equals(s.key)) { recover(s, "the vault changed elsewhere"); return; }
            s.lease = r.get("lease").getAsString();
            s.failures = 0;
            s.dirty = true;
        }, e -> {
            s.reacquiring = false;
            if (e.contains("another server")) recover(s, e);
            // Otherwise the panel is away: the next tick tries again; the live copy stays in memory.
        });
    }

    private static UUID ownerOf(String key) {
        if (!key.startsWith("player:")) return null;
        try { return UUID.fromString(key.substring("player:".length())); } catch (IllegalArgumentException e) { return null; }
    }

    /** Write the live copy to a recovery file, close it and tell everyone involved. Nothing is overwritten. */
    void recover(Session s, String why) {
        sessions.remove(s.id());
        JsonObject saved = new JsonObject();
        saved.addProperty("key", s.key);
        saved.addProperty("number", s.number);
        saved.addProperty("revision", s.revision);
        saved.addProperty("reason", why);
        saved.add("contents", s.view.contents());
        JsonArray ids = new JsonArray();
        s.deliveries.forEach(ids::add);
        saved.add("unacknowledged_deliveries", ids);
        String name = s.key.replaceAll("[^a-zA-Z0-9-]", "_") + "-" + s.number + "-" + env.clock.getAsLong() + ".json";
        try {
            Path dir = env.dataDir.resolve("vault-recovery");
            Files.createDirectories(dir);
            Files.writeString(dir.resolve(name), saved.toString());
            env.log.severe("[Velora Core] Vault " + s.id() + " could not be saved (" + why + "); its contents are in vault-recovery/" + name);
        } catch (Exception e) {
            env.log.severe("[Velora Core] Vault " + s.id() + " could not be saved or recovered: " + why + " / " + e + " contents=" + saved);
        }
        s.view.closeAll();
        for (UUID player : s.everyone)
            env.platform.player(player).ifPresent(p -> p.send(Format.RED + "Your vault could not be saved; staff have a recovery copy. Please contact an administrator."));
    }

    private void release(Session s) {
        if (s.lease == null) return;
        JsonObject body = saveBody(s, s.view == null ? new JsonArray() : s.view.contents(), List.of(), true);
        if (s.view == null) return; // nothing was shown: the lease simply expires
        env.io(() -> env.panel.call("vault/save", body), r -> { }, e -> { });
    }

    /** Move a player's local vault file into the panel once. I/O thread. */
    private void migrate(UUID owner) throws Exception {
        if (migrated.contains(owner)) return;
        JsonArray local = onMain(() -> env.platform.localVaults(owner));
        if (local == null) { migrated.add(owner); return; }
        JsonObject body = new JsonObject();
        body.addProperty("uuid", owner.toString());
        body.add("vaults", local);
        JsonObject r = env.panel.call("vault/import", body).getAsJsonObject();
        if (r.has("migrated") && r.get("migrated").getAsBoolean()) {
            onMain(() -> { env.platform.localVaultsMigrated(owner); return null; });
            migrated.add(owner);
        } else {
            env.log.warning("[Velora Core] Local vaults for " + owner + " were not imported: " + (r.has("reason") ? r.get("reason").getAsString() : "refused"));
            migrated.add(owner);
        }
    }

    private <T> T onMain(Supplier<T> work) throws Exception {
        CompletableFuture<T> result = new CompletableFuture<>();
        env.platform.runMain(() -> {
            try { result.complete(work.get()); } catch (RuntimeException e) { result.completeExceptionally(e); }
        });
        return result.get(30, TimeUnit.SECONDS);
    }

    // ---- deliveries ------------------------------------------------------------------------------------------

    /**
     * Store one delivery for {@code owner} in vault 1..{@code vaults}. An open vault on this server takes it at once and
     * acknowledges it with its next save; otherwise each vault is leased, filled and released in turn. {@code done} gets
     * true when stored, false when no vault had room and null when the panel was unreachable (offered again later).
     * Server thread.
     */
    public void deliver(UUID owner, long id, Item item, int vaults, java.util.function.Consumer<Boolean> done) {
        String key = "player:" + owner;
        for (int n = 1; n <= vaults; n++) {
            Session s = sessions.get(key + "#" + n);
            if (s != null && s.transfer==null && !s.reacquiring && s.view.deposit(item)) {
                s.deliveries.add(id);
                s.dirty = true;
                done.accept(true);
                return;
            }
        }
        List<Integer> free = new ArrayList<>();
        for (int n = 1; n <= vaults; n++) if (!sessions.containsKey(key + "#" + n) && busy.add(key + "#" + n)) free.add(n);
        env.io(() -> {
            try {
                migrate(owner);
                for (int n : free) if (offline(owner, n, id, item)) return true;
                return false;
            } finally {
                for (int n : free) busy.remove(key + "#" + n);
            }
        }, done, e -> { env.log.warning("[Velora Core] Vault delivery " + id + " will be retried: " + e); done.accept(null); });
    }

    /** I/O thread: lease vault n, add the item on the server thread, store it with the delivery or release it unchanged. */
    private boolean offline(UUID owner, int n, long id, Item item) throws Exception {
        JsonObject open = new JsonObject();
        open.addProperty("uuid", owner.toString());
        open.addProperty("owner", "player");
        open.addProperty("number", n);
        JsonObject r;
        try { r = env.panel.call("vault/open", open).getAsJsonObject(); }
        catch (Exception e) { return false; } // locked, or open on another server: try the next vault
        JsonArray contents = r.getAsJsonArray("contents");
        int rows = r.get("rows").getAsInt();
        JsonArray filled = onMain(() -> env.platform.vaultInsert(contents, rows, item));
        JsonObject save = new JsonObject();
        save.addProperty("key", r.get("key").getAsString());
        save.addProperty("number", n);
        save.addProperty("lease", r.get("lease").getAsString());
        save.addProperty("revision", r.get("revision").getAsLong());
        save.addProperty("release", true);
        save.add("contents", filled == null ? contents : filled);
        if (filled != null) {
            JsonArray ids = new JsonArray();
            ids.add(id);
            save.add("delivery_ids", ids);
        }
        env.panel.call("vault/save", save);
        return filled != null;
    }

    /**
     * Server stopping: save and release every open vault now, on this thread. A save still in flight may already have
     * stored the same contents; that counts as saved, anything else goes to a recovery file.
     */
    public void shutdown() {
        for (Session s : new ArrayList<>(sessions.values())) {
            if(s.transfer!=null){s.view.closeAll();continue;}
            JsonArray contents = s.view.contents();
            try {
                env.panel.call("vault/save", saveBody(s, contents, s.deliveries, true));
            } catch (Exception first) {
                try {
                    JsonObject open = new JsonObject();
                    UUID who = s.everyone.isEmpty() ? ownerOf(s.key) : s.everyone.iterator().next();
                    open.addProperty("uuid", String.valueOf(who));
                    open.addProperty("owner", s.key.startsWith("shop:") ? s.key : s.key.startsWith("faction:") ? "faction" : "player");
                    open.addProperty("number", s.number);
                    open.addProperty("permitted", true);
                    JsonObject r = env.panel.call("vault/open", open).getAsJsonObject();
                    if (!r.getAsJsonArray("contents").equals(contents) || !s.deliveries.isEmpty()) throw first;
                    s.lease = r.get("lease").getAsString();
                    s.revision = r.get("revision").getAsLong();
                    env.panel.call("vault/save", saveBody(s, contents, List.of(), true));
                } catch (Exception e) {
                    recover(s, "shutdown save failed: " + first.getMessage());
                    continue;
                }
            }
            s.view.closeAll();
        }
        sessions.clear();
    }

    /** Vaults open on this server right now, for status output. */
    public int openCount() { return sessions.size(); }

    /** Test hook: the live session for a vault, or null. */
    Session session(String key, int number) { return sessions.get(key + "#" + number); }

}
