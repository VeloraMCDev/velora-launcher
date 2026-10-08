package net.scopenet.integration.map;

import java.util.List;
import java.util.function.Supplier;

/** Neutral map lifecycle; overlays are opaque JSON, with policies owned by extensions. */
public interface MapSink extends AutoCloseable {
    void start();
    void offerPlayers(List<? extends MapPosition> positions);
    void setOverlay(Supplier<String> source);
    void requestScan();
    String status();
    @Override void close();
}
