package net.scopenet.integration;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.sun.net.httpserver.HttpServer;
import org.junit.jupiter.api.*;
import org.junit.jupiter.api.io.TempDir;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;
import java.util.concurrent.*;
import java.util.concurrent.atomic.AtomicInteger;
import static org.junit.jupiter.api.Assertions.*;

class OperationLedgerTest {
    private static final String TOKEN = "sn_" + "b".repeat(40);
    private HttpServer server;
    private PanelClient client;
    @TempDir Path dir;

    @BeforeEach void start() throws Exception {
        server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        server.start();
        client = new PanelClient(Settings.of("http://127.0.0.1:" + server.getAddress().getPort(), TOKEN));
    }

    @AfterEach void stop() {
        client.close();
        server.stop(0);
    }

    private void reply(String path, int[] statuses, AtomicInteger calls, List<String> seen) {
        server.createContext("/api/server/v1/" + path, exchange -> {
            String body = new String(exchange.getRequestBody().readAllBytes(), StandardCharsets.UTF_8);
            int n = calls.getAndIncrement();
            seen.add(JsonParser.parseString(body).getAsJsonObject().get("operation_id").getAsString());
            int status = statuses[Math.min(n, statuses.length - 1)];
            byte[] out = (status == 200 ? "{\"ok\":true,\"balance\":42}" : "{\"error\":\"nope\"}").getBytes(StandardCharsets.UTF_8);
            exchange.sendResponseHeaders(status, out.length);
            exchange.getResponseBody().write(out);
            exchange.close();
        });
    }

    private static JsonObject payload() {
        JsonObject p = new JsonObject();
        p.addProperty("delta", 5);
        return p;
    }

    @Test void retriesWithTheSameIdUntilThePanelAnswers() throws Exception {
        AtomicInteger calls = new AtomicInteger();
        List<String> ids = new CopyOnWriteArrayList<>();
        reply("economy/adjust", new int[]{503, 503, 200}, calls, ids);
        BlockingQueue<JsonObject> results = new LinkedBlockingQueue<>();
        try (OperationLedger ledger = new OperationLedger(dir.resolve("pending.json"), client, s -> {}, (id, r) -> results.add(r))) {
            String id = ledger.submit("economy/adjust", payload());
            JsonObject result = results.poll(15, TimeUnit.SECONDS);
            assertNotNull(result);
            assertEquals(42, result.get("balance").getAsInt());
            assertEquals(3, calls.get());
            assertEquals(Set.of(id), new HashSet<>(ids), "every retry carries the same operation id");
            assertEquals(0, ledger.size());
        }
        assertEquals("{}", Files.readString(dir.resolve("pending.json")).trim());
    }

    @Test void permanentRefusalsAreReportedNotRetried() throws Exception {
        AtomicInteger calls = new AtomicInteger();
        reply("economy/adjust", new int[]{400}, calls, new CopyOnWriteArrayList<>());
        BlockingQueue<JsonObject> results = new LinkedBlockingQueue<>();
        try (OperationLedger ledger = new OperationLedger(dir.resolve("pending.json"), client, s -> {}, (id, r) -> results.add(r))) {
            ledger.submit("economy/adjust", payload());
            JsonObject result = results.poll(10, TimeUnit.SECONDS);
            assertNotNull(result);
            assertEquals("nope", result.get("error").getAsString());
            Thread.sleep(2500);
            assertEquals(1, calls.get());
        }
    }

    @Test void pendingOperationsSurviveARestart() throws Exception {
        Path file = dir.resolve("pending.json");
        AtomicInteger calls = new AtomicInteger();
        List<String> ids = new CopyOnWriteArrayList<>();
        reply("economy/adjust", new int[]{503}, calls, ids);
        String id;
        try (OperationLedger first = new OperationLedger(file, client, s -> {}, (i, r) -> {})) {
            id = first.submit("economy/adjust", payload());
            assertTrue(Files.readString(file).contains(id), "saved before anything is sent");
        }
        server.removeContext("/api/server/v1/economy/adjust");
        AtomicInteger ok = new AtomicInteger();
        List<String> resent = new CopyOnWriteArrayList<>();
        reply("economy/adjust", new int[]{200}, ok, resent);
        BlockingQueue<JsonObject> results = new LinkedBlockingQueue<>();
        List<String> logs = new CopyOnWriteArrayList<>();
        try (OperationLedger second = new OperationLedger(file, client, logs::add, (i, r) -> results.add(r))) {
            assertNotNull(results.poll(10, TimeUnit.SECONDS));
            assertEquals(List.of(id), resent);
            assertTrue(logs.stream().anyMatch(l -> l.contains("resending 1")));
        }
    }
}
