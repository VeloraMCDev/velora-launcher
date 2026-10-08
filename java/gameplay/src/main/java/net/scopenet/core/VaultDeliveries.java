package net.scopenet.core;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

import java.util.ArrayList;
import java.util.List;
import java.util.UUID;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.function.Supplier;

/**
 * Items bought or won through the launcher (and auction results) are queued by the panel. Every half minute this asks for them
 * and puts each into its owner's vault, online or not, then confirms so they are not delivered twice. A vault that has no room
 * sends the item back to the market mailbox for {@code /market claim}.
 */
public final class VaultDeliveries {
    private static final int EVERY_TICKS = 30;
    private final Env env;
    private final Supplier<Utilities> settings;
    private final UtilityStore store;
    private final AtomicBoolean busy = new AtomicBoolean();
    private int ticks;

    public VaultDeliveries(Env env, Supplier<Utilities> settings, UtilityStore store) {
        this.env = env;
        this.settings = settings;
        this.store = store;
    }

    /** Call once a second from the server thread. */
    public void tick() {
        if (++ticks % EVERY_TICKS != 0 || !busy.compareAndSet(false, true)) return;
        JsonObject ask = new JsonObject();
        ask.addProperty("limit", 25);
        env.platform.runAsync(() -> {
            try {
                JsonElement answer = env.panel.call("economy/market/vault", ask);
                JsonArray list = answer.isJsonObject() && answer.getAsJsonObject().has("deliveries") ? answer.getAsJsonObject().getAsJsonArray("deliveries") : new JsonArray();
                if (list.size() == 0) { busy.set(false); return; }
                env.platform.runMain(() -> deliver(list));
            } catch (Exception e) {
                busy.set(false); // the panel is away; try again next time
            }
        });
    }

    /** Server thread. */
    void deliver(JsonArray list) {
        List<Long> done = new ArrayList<>(), failed = new ArrayList<>();
        Utilities.Vault vault = settings.get().vault;
        for (JsonElement e : list) {
            JsonObject d = e.getAsJsonObject();
            long id = d.get("id").getAsLong();
            UUID owner;
            try { owner = UUID.fromString(d.get("uuid").getAsString()); } catch (IllegalArgumentException ex) { done.add(id); continue; }
            // Already delivered but the confirmation was lost: don't deliver again.
            if (store.last(owner, "vault:" + id) >= 0) { done.add(id); continue; }
            String data = d.has("item_data") && !d.get("item_data").isJsonNull() ? d.get("item_data").getAsString() : "";
            String name = d.has("item_name") ? d.get("item_name").getAsString() : d.get("item_id").getAsString();
            Item item = new Item(d.get("item_id").getAsString(), name, d.get("amount").getAsInt(), data);
            boolean ok = vault.enabled() && env.platform.depositToVault(owner, item, vault.count(), vault.rows());
            if (ok) {
                store.stamp(owner, "vault:" + id, env.clock.getAsLong());
                done.add(id);
                env.platform.player(owner).ifPresent(p -> p.send(Format.GOLD + "[Market] " + Format.YELLOW + item.count() + "x " + Format.plain(item.name()) + Format.GRAY + " arrived in your " + Format.WHITE + "/vault" + Format.GRAY + "."));
            } else failed.add(id);
        }
        JsonObject body = new JsonObject();
        JsonArray ids = new JsonArray(), bad = new JsonArray();
        done.forEach(ids::add);
        failed.forEach(bad::add);
        body.add("ids", ids);
        body.add("failed", bad);
        env.platform.runAsync(() -> {
            try { env.panel.call("economy/market/vault/ack", body); } catch (Exception ignored) { /* the lease expires and it is offered again; the stamp stops a duplicate */ }
            finally { busy.set(false); }
        });
    }
}
