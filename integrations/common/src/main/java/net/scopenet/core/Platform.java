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

    // ---- cloud vaults (contents live in the panel; see CloudVaults) ----------------------------------------

    /** The live container behind an open cloud vault. Server thread only. */
    interface VaultView {
        /** Every stack as panel JSON: slot, item (registry id), count, name, data (exact serialization). */
        com.google.gson.JsonArray contents();
        /** Put the whole delivery in, or change nothing and answer false. */
        boolean deposit(Item item);
        /** Close this vault for everyone looking at it. */
        void closeAll();
        default void transfers(VaultTransfer handler) {}
        default void replace(com.google.gson.JsonArray contents) { throw new UnsupportedOperationException(); }
    }
    interface VaultTransfer {
        void begin(UUID actor,com.google.gson.JsonArray after,java.util.function.Function<String,Boolean> applyDurably,java.util.function.Consumer<Boolean> done);
    }
    /** Checkpoint loaded from the player's durable Minecraft data, never an elapsed-time guess. */
    default String inventoryCheckpoint(UUID player) { throw new UnsupportedOperationException(); }
    default void lockInventory(UUID player,boolean locked) {}

    /** True when this platform stores vaults in the panel instead of local files. */
    default boolean cloudVaults() { return false; }

    /**
     * Build the live container for a vault. {@code changed} runs on every edit; {@code closed} when a viewer closes it.
     * Refuse to open when stored slots exceed the configured rows; custody must not spill items into an inventory.
     */
    default VaultView createVault(String title, int rows, com.google.gson.JsonArray contents, Runnable changed,
                                  java.util.function.Consumer<UUID> closed, UUID firstViewer) { return null; }

    /** Open the vault for one more player; false if they cannot see it (seven rows need the Core client). */
    default boolean showVault(UUID player, VaultView view) { return false; }

    /** The contents with {@code item} added, or null if it does not fit. Pure: used for offline deliveries. */
    default com.google.gson.JsonArray vaultInsert(com.google.gson.JsonArray contents, int rows, Item item) { return null; }

    /** A player's not-yet-migrated local vault file as {@code [{number, contents}]}, or null when there is none. */
    default com.google.gson.JsonArray localVaults(UUID owner) { return null; }

    /** The local vault file now lives in the panel: keep it aside so it is never imported again. */
    default void localVaultsMigrated(UUID owner) {}

    /** Write an online player's data now, narrowing the window between a vault save and the next world save. */
    default void savePlayer(UUID player) {}

    /** Outpost flags use the existing banner appearance; authority and limits live in the panel. */
    default boolean canPlaceOutpostFlag(Pos pos) { return false; }
    default boolean placeOutpostFlag(Pos pos) { return false; }
}
