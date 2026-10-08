package net.scopenet.core;

/**
 * A stack of items in a platform-neutral form. {@code id} is what the panel stores ("DIAMOND", or
 * "modid:item" for modded items); {@code data} is the platform's own exact serialisation (used to return
 * or deliver the very same stack). When {@code data} is empty the platform rebuilds the stack from id and count.
 */
public record Item(String id, String name, int count, String data) {
    /** Panel id for a registry id: minecraft items become "DIAMOND", others stay "modid:item". */
    public static String panelId(String registryId) {
        String s = registryId.toLowerCase(java.util.Locale.ROOT);
        return s.startsWith("minecraft:") ? s.substring("minecraft:".length()).toUpperCase(java.util.Locale.ROOT) : s;
    }

    /** The registry id for a panel id (inverse of {@link #panelId}). */
    public static String registryId(String panelId) {
        return panelId.contains(":") ? panelId : "minecraft:" + panelId.toLowerCase(java.util.Locale.ROOT);
    }
}
