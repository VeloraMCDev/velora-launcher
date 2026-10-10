package net.velora.core;

/** A spot in a world, independent of any server software. {@code world} is the dimension id, e.g. "minecraft:overworld". */
public record Pos(String world, double x, double y, double z, float yaw, float pitch) {
    public int blockX() { return (int) Math.floor(x); }
    public int blockY() { return (int) Math.floor(y); }
    public int blockZ() { return (int) Math.floor(z); }
}
