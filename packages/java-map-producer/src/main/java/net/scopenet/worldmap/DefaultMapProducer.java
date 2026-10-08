package net.scopenet.worldmap;

import java.nio.file.Path;
import java.util.function.Consumer;
import net.scopenet.integration.PanelTransport;
import net.scopenet.integration.map.MapProducerFactory;
import net.scopenet.integration.map.MapSink;

/** Local map producer registered through the neutral SDK SPI. */
public final class DefaultMapProducer implements MapProducerFactory {
    @Override public int apiVersion() { return 1; }
    @Override public MapSink create(PanelTransport transport, Path world, Path cache, Consumer<String> log) {
        return new WorldMapSync(transport, world, cache, log);
    }
}
