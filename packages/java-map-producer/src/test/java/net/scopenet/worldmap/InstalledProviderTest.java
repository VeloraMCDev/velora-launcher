package net.scopenet.worldmap;

import java.nio.file.Path;
import java.util.ServiceLoader;
import net.scopenet.integration.PanelTransport;
import net.scopenet.integration.ServerConnection;
import net.scopenet.integration.map.MapProducerFactory;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import static org.junit.jupiter.api.Assertions.*;

class InstalledProviderTest {
    @Test void reviewed_local_provider_is_discovered_through_sdk_contract(@TempDir Path dir) {
        var providers = ServiceLoader.load(MapProducerFactory.class, MapProducerFactory.class.getClassLoader()).stream().map(ServiceLoader.Provider::get).toList();
        assertEquals(1, providers.size());
        var provider = providers.get(0);
        assertEquals(MapProducerFactory.API_VERSION, provider.apiVersion());
        var transport = new PanelTransport(ServerConnection.of("http://127.0.0.1:1", "sn_" + "a".repeat(40)));
        try (var sink = provider.create(transport, dir.resolve("world"), dir.resolve("cache"), message -> {})) {
            assertInstanceOf(WorldMapSync.class, sink);
            assertTrue(sink.status().contains("waiting for panel/map setting"));
        }
    }

    @Test void discovery_uses_api_loader_even_when_server_context_loader_is_isolated() throws Exception {
        var thread = Thread.currentThread();
        var original = thread.getContextClassLoader();
        try (var isolatedServer = new java.net.URLClassLoader(new java.net.URL[0], null)) {
            thread.setContextClassLoader(isolatedServer);
            var providers = ServiceLoader.load(MapProducerFactory.class, MapProducerFactory.class.getClassLoader()).stream().map(ServiceLoader.Provider::get).toList();
            assertEquals(1, providers.size());
            assertEquals(MapProducerFactory.API_VERSION, providers.get(0).apiVersion());
        } finally { thread.setContextClassLoader(original); }
    }
}
