package net.scopenet.core;

import java.util.Collection;
import java.util.Optional;
import java.util.UUID;

/** The server side of the adapter: who is online, the world, and threads. */
public interface Platform {
    /** Send a resource pack offer to a connected player. */
    default boolean offerResourcePack(UUID player, String url, String sha1, boolean required) { return false; }
    /** Snapshot the 36 storage slots without removing anything, including platform metadata. */
    default com.google.gson.JsonArray inventorySnapshot(UUID player) { return new com.google.gson.JsonArray(); }
    Optional<CorePlayer> player(UUID uuid);
    Optional<CorePlayer> playerByName(String name);
    Collection<? extends CorePlayer> online();
    /**
     * The spot to stand on at (x, z): the top solid, non-liquid block. Empty if unsafe or unloadable.
     * Called on the server thread.
     */
    Optional<Pos> safeSurface(String world, int x, int z);
    Pos worldSpawn(String world);
    /** Run on the server thread (may run immediately if already there). */
    void runMain(Runnable task);
    /** Run off the server thread, for network calls. */
    void runAsync(Runnable task);

    /** Put items in the player's inventory (anything that doesn't fit drops at their feet). False if the item doesn't exist here. */
    default boolean giveItem(UUID player, String itemId, int amount) { return false; }

    /** Run a command as the console; true if it was found and run. Called on the server thread. */
    default boolean console(String command) { return false; }

    // ---- utilities (/heal /feed /fly /vault /echest /kit, claim banner) --------------------------------------

    /** Give a designed item (name, lore, enchantments…). Platforms that can't style it fall back to the plain item. */
    default boolean giveSpec(UUID player, ItemSpec spec) { return giveItem(player, spec.item(), spec.amount()); }

    /** Restore health points; zero or less means full health. Does nothing in creative or spectator. */
    default void heal(UUID player, double amount) {}

    /** Restore hunger points (20 fills the bar) along with a little saturation. */
    default void feed(UUID player, int amount) {}

    /** Allow or take away flight. Taking it away also grounds the player safely. Creative and spectator are left alone. */
    default void setFlight(UUID player, boolean allowed) {}

    /** Open vault {@code number} (1-based) as a chest with {@code rows} rows; contents are saved when it closes. */
    default void openVault(UUID player, int number, int rows) {}

    default void openEnderChest(UUID player) {}

    /** Show text above the hotbar. Legacy section-sign colour codes are allowed. */
    default void actionbar(UUID player, String text) {}

    /**
     * Put a stack into one of the player's vaults (1..count, {@code rows} rows each), online or not, merging into matching stacks
     * first. False if every vault is full or the item can't be built here.
     */
    default boolean depositToVault(UUID player, Item item, int vaults, int rows) { return false; }
}
