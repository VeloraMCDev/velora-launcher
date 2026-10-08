package net.scopenet.integration.map;

import java.nio.file.Path;
import java.util.function.Consumer;
import net.scopenet.integration.PanelTransport;

/** Reviewed, locally installed ServiceLoader provider; never selected from remote manifests. */
public interface MapProducerFactory {
    int API_VERSION = 1;
    int apiVersion();
    MapSink create(PanelTransport transport, Path world, Path cache, Consumer<String> log);
}
