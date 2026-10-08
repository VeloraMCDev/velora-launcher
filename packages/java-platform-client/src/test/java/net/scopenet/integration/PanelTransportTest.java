package net.scopenet.integration;

import com.google.gson.*;
import com.sun.net.httpserver.HttpServer;
import org.junit.jupiter.api.*;
import java.net.*;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.atomic.*;
import static org.junit.jupiter.api.Assertions.*;

class PanelTransportTest {
    private HttpServer server;
    private final String token = "sn_" + "a".repeat(40);
    private ServerConnection connection;
    @BeforeEach void start() throws Exception {
        server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        server.start();
        connection = ServerConnection.of("http://127.0.0.1:" + server.getAddress().getPort(), token);
    }
    @AfterEach void stop() { server.stop(0); }
    private void respond(String path, int status, String body) {
        server.createContext("/api/server/v1/" + path, exchange -> {
            byte[] bytes = body.getBytes(StandardCharsets.UTF_8);
            exchange.sendResponseHeaders(status, bytes.length);
            exchange.getResponseBody().write(bytes); exchange.close();
        });
    }
    @Test void jsonAndBinaryRequestsPreserveAuthenticationHeadersBodiesAndResponseKinds() throws Exception {
        AtomicReference<String> received = new AtomicReference<>();
        server.createContext("/api/server/v1/report", exchange -> {
            assertEquals("Bearer " + token, exchange.getRequestHeaders().getFirst("Authorization"));
            assertEquals("application/json", exchange.getRequestHeaders().getFirst("Content-Type"));
            received.set(new String(exchange.getRequestBody().readAllBytes(), StandardCharsets.UTF_8));
            byte[] bytes = "[1,2]".getBytes(StandardCharsets.UTF_8);
            exchange.sendResponseHeaders(200, bytes.length); exchange.getResponseBody().write(bytes); exchange.close();
        });
        PanelTransport client = new PanelTransport(connection);
        JsonObject payload = new JsonObject(); payload.addProperty("sample", 7);
        assertEquals(2, client.postElement("report", payload).getAsJsonArray().size());
        assertEquals(7, JsonParser.parseString(received.get()).getAsJsonObject().get("sample").getAsInt());
        server.createContext("/api/server/v1/bytes", exchange -> {
            assertEquals("Bearer " + token, exchange.getRequestHeaders().getFirst("Authorization"));
            assertEquals("application/octet-stream", exchange.getRequestHeaders().getFirst("Content-Type"));
            assertArrayEquals(new byte[]{1,2,3}, exchange.getRequestBody().readAllBytes());
            byte[] bytes = "{\"ok\":true}".getBytes(StandardCharsets.UTF_8);
            exchange.sendResponseHeaders(200, bytes.length); exchange.getResponseBody().write(bytes); exchange.close();
        });
        assertTrue(client.postBytes("bytes", new byte[]{1,2,3}).get("ok").getAsBoolean());
    }
    @Test void errorPrecedenceAndLegacyFailureFactoriesRemainUsable() {
        class LegacyFailure extends IOException { LegacyFailure(int status, String message) { super(status + ":" + message); } }
        respond("error", 403, "{\"message\":\"preferred\",\"error\":\"other\"}");
        PanelTransport client = new PanelTransport(() -> connection, LegacyFailure::new);
        LegacyFailure error = assertThrows(LegacyFailure.class, () -> client.get("error"));
        assertEquals("403:preferred", error.getMessage());
        respond("invalid", 200, "not json");
        assertThrows(IOException.class, () -> client.get("invalid"));
    }
    @Test void redirectsDoNotForwardCredentialsAndConnectionUpdatesAreObserved() throws Exception {
        AtomicInteger redirected = new AtomicInteger();
        HttpServer target = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        target.createContext("/stolen", exchange -> { redirected.incrementAndGet(); exchange.close(); }); target.start();
        try {
            server.createContext("/api/server/v1/redirect", exchange -> {
                exchange.getResponseHeaders().set("Location", "http://127.0.0.1:" + target.getAddress().getPort() + "/stolen");
                exchange.sendResponseHeaders(302, -1); exchange.close();
            });
            PanelTransport client = new PanelTransport(() -> connection, PanelTransport.HttpFailure::new);
            assertThrows(PanelTransport.HttpFailure.class, () -> client.get("redirect"));
            assertEquals(0, redirected.get());
            AtomicReference<String> header = new AtomicReference<>();
            server.createContext("/api/server/v1/state", exchange -> {
                header.set(exchange.getRequestHeaders().getFirst("Authorization"));
                byte[] bytes = "{}".getBytes(StandardCharsets.UTF_8);
                exchange.sendResponseHeaders(200, bytes.length); exchange.getResponseBody().write(bytes); exchange.close();
            });
            connection = new ServerConnection(connection.panel(), "sn_" + "b".repeat(40));
            client.get("state"); assertEquals("Bearer " + connection.token(), header.get());
        } finally { target.stop(0); }
    }
    @Test void connectionValidationRetainsLegacyRulesAndDoesNotPrintCredentials() {
        assertThrows(IllegalArgumentException.class, () -> ServerConnection.of("https://user:pass@example.invalid", token));
        assertThrows(IllegalArgumentException.class, () -> ServerConnection.of("file:///tmp/test", token));
        assertThrows(IllegalArgumentException.class, () -> ServerConnection.of("https://example.invalid/?query=yes", token));
        assertThrows(IllegalArgumentException.class, () -> ServerConnection.of("https://example.invalid", ""));
        assertFalse(connection.toString().contains(token));
    }
}
