package net.velora.fabric.features;
public final class InventoryLocks {
    private static final java.util.Set<java.util.UUID> LOCKED=java.util.concurrent.ConcurrentHashMap.newKeySet();
    public static boolean locked(java.util.UUID player){return LOCKED.contains(player);}
    public static void set(java.util.UUID player,boolean locked){if(locked)LOCKED.add(player);else LOCKED.remove(player);}
}
