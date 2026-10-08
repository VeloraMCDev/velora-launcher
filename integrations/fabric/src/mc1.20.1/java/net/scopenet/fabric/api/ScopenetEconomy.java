package net.scopenet.fabric.api;

import com.google.gson.JsonObject;
import net.scopenet.core.Panel;

import java.util.UUID;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.atomic.AtomicReference;

/**
 * Velora's economy for other mods (the Fabric stand-in for Vault). Balances live in the panel, so every call is
 * asynchronous. Every change carries an operation id, so a retried call never pays twice.
 *
 * <pre>{@code
 * ScopenetEconomy.get().ifPresent(eco -> eco.balance(uuid).thenAccept(b -> ...));
 * }</pre>
 */
public final class ScopenetEconomy {
    private static final AtomicReference<ScopenetEconomy> INSTANCE = new AtomicReference<>();

    private final Panel panel;
    private final java.util.concurrent.Executor io;

    public ScopenetEconomy(Panel panel, java.util.concurrent.Executor io) { this.panel = panel; this.io = io; }

    /** Empty until the server has started and Velora economy is enabled. */
    public static java.util.Optional<ScopenetEconomy> get() { return java.util.Optional.ofNullable(INSTANCE.get()); }

    public static void install(ScopenetEconomy economy) { INSTANCE.set(economy); }

    public static void uninstall() { INSTANCE.set(null); }

    public CompletableFuture<Double> balance(UUID player, String name) {
        JsonObject body = new JsonObject();
        body.addProperty("uuid", player.toString());
        body.addProperty("username", name);
        return CompletableFuture.supplyAsync(() -> {
            try { return panel.call("economy/balance", body).getAsJsonObject().get("balance").getAsDouble(); }
            catch (Exception e) { throw new java.util.concurrent.CompletionException(e); }
        }, io);
    }

    /** Add (positive) or remove (negative) money. Fails, without changing anything, if the player can't afford it. Returns the new balance. */
    public CompletableFuture<Double> change(UUID player, String name, double delta, String reason) {
        JsonObject body = new JsonObject();
        body.addProperty("operation_id", UUID.randomUUID().toString());
        body.addProperty("uuid", player.toString());
        body.addProperty("username", name);
        body.addProperty("delta", delta);
        body.addProperty("description", reason == null || reason.isBlank() ? "Other mod" : reason);
        return CompletableFuture.supplyAsync(() -> {
            try { return panel.call("economy/adjust", body).getAsJsonObject().get("balance").getAsDouble(); }
            catch (Exception e) { throw new java.util.concurrent.CompletionException(e); }
        }, io);
    }

    public CompletableFuture<Double> deposit(UUID player, String name, double amount, String reason) { return change(player, name, Math.abs(amount), reason); }

    public CompletableFuture<Double> withdraw(UUID player, String name, double amount, String reason) { return change(player, name, -Math.abs(amount), reason); }
}
