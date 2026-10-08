package net.scopenet.core;

import com.google.gson.*;

import java.io.IOException;
import java.io.Reader;
import java.io.Writer;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.*;

/**
 * Money-and-items transactions that must never lose either. The held items are taken and written to disk first
 * ("escrow"), the panel call carries an operation id so a retry can't double-pay, and the items are only
 * returned or delivered once the panel has answered. Survives crashes and panel outages. Server thread only.
 */
public final class Jobs {
    private final Env env;
    private final Path file;
    private final Map<String, JsonObject> jobs = new LinkedHashMap<>();
    private final Set<String> running = new HashSet<>();

    public Jobs(Env env, Path file) {
        this.env = env;
        this.file = file;
        if (Files.exists(file)) {
            try (Reader in = Files.newBufferedReader(file)) {
                JsonObject all = JsonParser.parseReader(in).getAsJsonObject();
                all.entrySet().forEach(e -> jobs.put(e.getKey(), e.getValue().getAsJsonObject()));
            } catch (IOException | RuntimeException e) {
                throw new IllegalStateException("Could not read " + file + ": " + e.getMessage(), e);
            }
        }
    }

    public int pending() { return jobs.size(); }

    private static JsonObject encode(Item item) {
        JsonObject o = new JsonObject();
        o.addProperty("id", item.id());
        o.addProperty("name", item.name());
        o.addProperty("count", item.count());
        o.addProperty("data", item.data());
        return o;
    }

    private static Item decode(JsonElement e) {
        JsonObject o = e.getAsJsonObject();
        return new Item(o.get("id").getAsString(), o.get("name").getAsString(), o.get("count").getAsInt(), o.get("data").getAsString());
    }

    /**
     * Queue {@code endpoint} with {@code payload}. {@code escrow} were taken from the player and are returned if the
     * panel refuses. {@code purchase} is delivered once it accepts. Returns false (and queues nothing) if it can't be saved.
     */
    public boolean enqueue(CorePlayer player, String endpoint, JsonObject payload, List<Item> escrow, Item purchase) {
        String id = UUID.randomUUID().toString();
        payload.addProperty("operation_id", id);
        JsonObject job = new JsonObject();
        job.addProperty("player", player.uuid().toString());
        job.addProperty("endpoint", endpoint);
        job.add("payload", payload);
        JsonArray held = new JsonArray();
        escrow.forEach(i -> held.add(encode(i)));
        job.add("escrow", held);
        if (purchase != null) job.add("purchase", encode(purchase));
        jobs.put(id, job);
        try { save(); } catch (IOException e) {
            jobs.remove(id);
            player.send(Format.RED + "Cannot save transaction; nothing was purchased or sold.");
            return false;
        }
        player.send(Format.GRAY + "Transaction queued. Items are kept safe while the panel responds.");
        return true;
    }

    private void save() throws IOException {
        JsonObject all = new JsonObject();
        jobs.forEach(all::add);
        if (file.getParent() != null) Files.createDirectories(file.getParent());
        Path tmp = file.resolveSibling(file.getFileName() + ".tmp");
        try (Writer w = Files.newBufferedWriter(tmp)) { w.write(all.toString()); }
        Files.move(tmp, file, StandardCopyOption.REPLACE_EXISTING);
    }

    /** Call every few seconds on the server thread. Sends waiting jobs and delivers finished ones. */
    public void tick() {
        for (Map.Entry<String, JsonObject> entry : new ArrayList<>(jobs.entrySet())) {
            String id = entry.getKey();
            JsonObject job = entry.getValue();
            if (job.has("result") || job.has("error")) { deliver(id, job); continue; }
            if (!running.add(id)) continue;
            String endpoint = job.get("endpoint").getAsString();
            JsonObject payload = job.getAsJsonObject("payload").deepCopy();
            env.platform.runAsync(() -> {
                JsonElement result = null;
                String error = null;
                try { result = env.panel.call(endpoint, payload); }
                catch (net.scopenet.integration.PanelClient.HttpFailure e) { if (e.status >= 400 && e.status < 500) error = e.getMessage(); }
                catch (Exception e) { /* Unknown outcome: retry the same operation id; a credit is never duplicated. */ }
                final JsonElement response = result;
                final String failure = error;
                env.platform.runMain(() -> {
                    running.remove(id);
                    if (response != null && response.isJsonObject()) job.add("result", response);
                    if (failure != null) job.addProperty("error", failure);
                    try { save(); } catch (IOException e) { env.log.severe("Could not persist economy result: " + e.getMessage()); return; }
                    if (job.has("result") || job.has("error")) deliver(id, job);
                });
            });
        }
    }

    private void deliver(String id, JsonObject job) {
        Optional<CorePlayer> found = env.platform.player(UUID.fromString(job.get("player").getAsString()));
        if (found.isEmpty()) return; // waits until they are back
        CorePlayer player = found.get();
        try {
            List<Item> items = new ArrayList<>();
            boolean failed = job.has("error");
            String endpoint = job.get("endpoint").getAsString();
            if (failed) {
                job.getAsJsonArray("escrow").forEach(e -> items.add(decode(e)));
            } else if (job.has("purchase")) {
                items.add(decode(job.get("purchase")));
            } else if (job.has("result") && job.getAsJsonObject("result").has("items") && job.getAsJsonObject("result").get("items").isJsonArray()) {
                // Mailbox collections and cancelled listings: a list of stacks to hand back.
                for (JsonElement e : job.getAsJsonObject("result").getAsJsonArray("items")) {
                    JsonObject it = e.getAsJsonObject();
                    String data = it.has("item_data") && !it.get("item_data").isJsonNull() ? it.get("item_data").getAsString() : "";
                    String itemId = it.get("item_id").getAsString();
                    items.add(new Item(itemId, it.has("item_name") ? it.get("item_name").getAsString() : itemId, it.get("amount").getAsInt(), data));
                }
            } else if (endpoint.equals("economy/market/buy")) {
                JsonObject result = job.getAsJsonObject("result");
                if (result.has("item_data") && !result.get("item_data").isJsonNull() && !result.get("item_data").getAsString().isEmpty()) {
                    // Another platform's exact stack can't be rebuilt here, so fall back to id and count.
                    items.add(new Item(result.get("item_id").getAsString(), result.get("item_id").getAsString(), result.get("amount").getAsInt(),
                            result.get("item_data").getAsString()));
                } else {
                    items.add(new Item(result.get("item_id").getAsString(), result.get("item_id").getAsString(), result.get("amount").getAsInt(), ""));
                }
            }
            items.forEach(player::give);
            jobs.remove(id);
            save();
            String symbol = env.features.currencySymbol();
            if (failed) {
                player.send(Format.RED + "Transaction rejected; held items returned: " + job.get("error").getAsString());
                return;
            }
            JsonObject result = job.getAsJsonObject("result");
            if (result.has("message") && result.get("message").isJsonPrimitive()) {
                JsonElement bal = result.get("balance");
                String bank = bal != null && bal.isJsonPrimitive() ? (endpoint.startsWith("guilds/") ? " Bank: " : " Balance: ") + Format.money(symbol, bal.getAsDouble()) : "";
                player.send(Format.GREEN + result.get("message").getAsString() + bank);
            } else {
                JsonElement bal = result.has("new_balance") ? result.get("new_balance") : result.get("balance");
                player.send(Format.GREEN + "Transaction completed." + (bal != null && bal.isJsonPrimitive() ? " Balance: " + Format.money(symbol, bal.getAsDouble()) : ""));
            }
        } catch (Exception e) {
            env.log.severe("Economy delivery requires attention for " + id + ": " + e.getMessage());
        }
    }
}
