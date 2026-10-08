package net.scopenet.integration;

import com.sun.net.httpserver.HttpServer;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import static org.junit.jupiter.api.Assertions.*;

class MissingMapProviderTest {
    @Test void sdk_only_gameplay_build_has_useful_missing_provider_behavior(@TempDir Path dir) throws Exception {
        var server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        server.createContext("/api/server/v1/", exchange -> {
            exchange.getRequestBody().readAllBytes();
            byte[] body = "{\"kick\":[]}".getBytes(StandardCharsets.UTF_8);
            exchange.sendResponseHeaders(200, body.length);
            exchange.getResponseBody().write(body);
            exchange.close();
        });
        server.start();
        var messages = new CopyOnWriteArrayList<String>();
        try (var integration = new Integration(Settings.of("http://127.0.0.1:" + server.getAddress().getPort(), "sn_" + "a".repeat(40)),
                "synthetic", "1.20.1", "test", true, 1, messages::add, (id, message) -> {})) {
            integration.setMapOverlay(() -> "{\"pins\":[]}");
            integration.enableMap(dir.resolve("world"), dir.resolve("cache"));
            assertTrue(integration.mapStatus().contains("Map is not started"));
            assertTrue(messages.stream().anyMatch(message -> message.contains("map producer is not installed")));
            integration.mapPlayers(List.of());
            integration.mapSaved();
        } finally { server.stop(0); }
    }
}
