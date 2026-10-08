package net.scopenet.integration.map;

/** Immutable position snapshots supplied by a reviewed game-host adapter. */
public interface MapPosition {
    String uuid();
    String name();
    String dimension();
    double x();
    double y();
    double z();
    float yaw();
    float pitch();
}
