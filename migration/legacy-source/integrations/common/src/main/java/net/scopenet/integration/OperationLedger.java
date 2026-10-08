package net.scopenet.integration;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;
import java.util.function.BiConsumer;
import java.util.function.Consumer;

/**
 * A write-ahead queue for panel requests that must not be lost or repeated,
 * like the money moved by Vault plugins. Each request gets an operation id and
 * is saved to disk before it is sent; it is retried until the panel answers
 * (the panel applies an operation id once), and only then forgotten. A panel
 * outage or a server crash therefore never drops or duplicates a payment.
 */
public final class OperationLedger implements AutoCloseable {
    private record Pending(String endpoint, JsonObject payload) {}

    private final Path file;
    private final PanelClient client;
    private final Consumer<String> log;
    private final BiConsumer<String, JsonObject> done;
    private final Map<String, Pending> pending = new LinkedHashMap<>();
    private final ScheduledExecutorService worker = Executors.newSingleThreadScheduledExecutor(r -> {
        Thread t = new Thread(r, "scopenet-ledger");
        t.setDaemon(true);
        return t;
    });
    private volatile boolean closed;

    /**
     * @param done called with (operation id, response) after the panel accepted an operation, or with
     *             (operation id, {"error": "..."}) after it permanently refused it
     */
    public OperationLedger(Path file, PanelClient client, Consumer<String> log, BiConsumer<String, JsonObject> done) {
        this.file = file;
        this.client = client;
        this.log = log;
        this.done = done;
        load();
        worker.scheduleWithFixedDelay(this::drain, 1, 2, TimeUnit.SECONDS);
    }

    public synchronized int size() { return pending.size(); }

    /** Save the operation and start sending it. Returns its operation id. */
    public String submit(String endpoint, JsonObject payload) throws IOException {
        String id = UUID.randomUUID().toString();
        payload.addProperty("operation_id", id);
        synchronized (this) {
            pending.put(id, new Pending(endpoint, payload));
            try { save(); } catch (IOException e) { pending.remove(id); throw e; }
        }
        try { worker.execute(this::drain); } catch (RuntimeException ignored) { /* closing */ }
        return id;
    }

    private void drain() {
        if (closed) return;
        Map<String, Pending> batch;
        synchronized (this) { batch = new LinkedHashMap<>(pending); }
        for (Map.Entry<String, Pending> entry : batch.entrySet()) {
            if (closed) return;
            String id = entry.getKey();
            JsonObject result;
            try {
                result = client.post(entry.getValue().endpoint(), entry.getValue().payload().deepCopy());
            } catch (PanelClient.HttpFailure e) {
                if (e.status >= 400 && e.status < 500 && e.status != 408 && e.status != 429) {
                    // The panel will never accept this one; report it and move on.
                    JsonObject error = new JsonObject();
                    error.addProperty("error", e.getMessage());
                    finish(id, error);
                    continue;
                }
                return; // panel trouble: try again later, keeping the order
            } catch (IOException e) {
                return; // unreachable: try again later
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
                return;
            }
            finish(id, result);
        }
    }

    private void finish(String id, JsonObject result) {
        synchronized (this) {
            pending.remove(id);
            try { save(); } catch (IOException e) { log.accept("SCOPENET could not update " + file.getFileName() + ": " + e.getMessage()); }
        }
        try { done.accept(id, result); } catch (RuntimeException e) { log.accept("SCOPENET operation callback failed: " + e.getMessage()); }
    }

    private void load() {
        try {
            if (!Files.isRegularFile(file)) return;
            JsonObject root = JsonParser.parseString(Files.readString(file, StandardCharsets.UTF_8)).getAsJsonObject();
            for (Map.Entry<String, com.google.gson.JsonElement> e : root.entrySet()) {
                JsonObject op = e.getValue().getAsJsonObject();
                pending.put(e.getKey(), new Pending(op.get("endpoint").getAsString(), op.getAsJsonObject("payload")));
            }
            if (!pending.isEmpty()) log.accept("SCOPENET is resending " + pending.size() + " saved transaction(s)");
        } catch (Exception e) {
            log.accept("SCOPENET could not read " + file.getFileName() + ": " + e.getMessage());
        }
    }

    private void save() throws IOException {
        JsonObject root = new JsonObject();
        for (Map.Entry<String, Pending> e : pending.entrySet()) {
            JsonObject op = new JsonObject();
            op.addProperty("endpoint", e.getValue().endpoint());
            op.add("payload", e.getValue().payload());
            root.add(e.getKey(), op);
        }
        Path parent = file.toAbsolutePath().getParent();
        if (parent != null) Files.createDirectories(parent);
        Path temp = file.resolveSibling(file.getFileName() + ".tmp");
        Files.writeString(temp, root.toString(), StandardCharsets.UTF_8);
        Files.move(temp, file, StandardCopyOption.REPLACE_EXISTING);
    }

    @Override public void close() {
        closed = true;
        worker.shutdownNow();
    }
}
