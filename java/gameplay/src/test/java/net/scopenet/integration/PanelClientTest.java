package net.scopenet.integration;

import com.google.gson.*;
import com.sun.net.httpserver.HttpServer;
import org.junit.jupiter.api.*;
import java.net.*;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.concurrent.*;
import java.util.concurrent.atomic.AtomicReference;
import static org.junit.jupiter.api.Assertions.*;

class PanelClientTest {
    private HttpServer server;
    private Settings settings;
    private final String token = "sn_" + "a".repeat(40);

    @BeforeEach void start() throws Exception {
        server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        server.start();
        settings = Settings.of("http://127.0.0.1:" + server.getAddress().getPort(), token);
    }
    @AfterEach void stop() { server.stop(0); }

    private void endpoint(String path, int status, String response, AtomicReference<JsonObject> received) {
        server.createContext("/api/server/v1/" + path, exchange -> {
            assertEquals("Bearer " + token, exchange.getRequestHeaders().getFirst("Authorization"));
            assertEquals("POST", exchange.getRequestMethod());
            received.set(JsonParser.parseString(new String(exchange.getRequestBody().readAllBytes(), StandardCharsets.UTF_8)).getAsJsonObject());
            byte[] bytes = response.getBytes(StandardCharsets.UTF_8);
            exchange.sendResponseHeaders(status, bytes.length);
            exchange.getResponseBody().write(bytes);
            exchange.close();
        });
    }

    @Test void postsAuthenticatedIdentityAndPreservesDenialMessage() throws Exception {
        AtomicReference<JsonObject> received = new AtomicReference<>();
        endpoint("login", 200, "{\"allowed\":false,\"message\":\"Account disabled\"}", received);
        Integration integration = integration();
        UUID id = UUID.randomUUID();
        assertEquals("Account disabled", integration.login(id, "Steve", "203.0.113.9").get(2, TimeUnit.SECONDS));
        assertEquals(id.toString(), received.get().get("uuid").getAsString());
        assertEquals("203.0.113.9", received.get().get("ip").getAsString());
        integration.close();
    }

    @Test void malformedAndHttpFailuresDenyAccess() throws Exception {
        endpoint("login", 200, "{}", new AtomicReference<>());
        Integration integration = integration();
        assertEquals(Integration.UNAVAILABLE, integration.login(UUID.randomUUID(), "Steve", null).get(2, TimeUnit.SECONDS));
        server.removeContext("/api/server/v1/login");
        endpoint("login", 503, "{\"allowed\":true}", new AtomicReference<>());
        assertEquals(Integration.UNAVAILABLE, integration.login(UUID.randomUUID(), "Steve", null).get(2, TimeUnit.SECONDS));
        integration.close();
    }

    @Test void onlyExplicitBooleanTrueAllowsLogin() throws Exception {
        assertNull(PanelClient.verdict(JsonParser.parseString("{\"allowed\":true}").getAsJsonObject()));
        for (String bad : List.of("{}", "{\"allowed\":\"true\"}", "{\"allowed\":null}", "{\"allowed\":1}")) {
            assertThrows(IOException.class, () -> PanelClient.verdict(JsonParser.parseString(bad).getAsJsonObject()));
        }
    }

    @Test void redirectsNeverForwardTheServerToken() {
        server.createContext("/api/server/v1/login", exchange -> {
            exchange.getResponseHeaders().set("Location", "http://127.0.0.1:1/stolen");
            exchange.sendResponseHeaders(302, -1);
            exchange.close();
        });
        assertThrows(IOException.class, () -> new PanelClient(settings).post("login", new JsonObject()));
    }

    @Test void invalidSettingsAndOfflineModeAreRejected() {
        assertThrows(IllegalArgumentException.class, () -> Settings.of("https://user:pass@panel.example.com", token));
        assertThrows(IllegalArgumentException.class, () -> Settings.of("file:///tmp/panel", token));
        assertThrows(IllegalArgumentException.class, () -> Settings.of("https://panel.example.com", ""));
        assertThrows(IllegalArgumentException.class, () -> new Integration(settings, "test", "1", "1", false, 20, s -> {}, (id, msg) -> {}));
    }

    private Integration integration() {
        return new Integration(settings, "test", "1.21.1", "test", true, 20, s -> {}, (id, msg) -> {});
    }
}
