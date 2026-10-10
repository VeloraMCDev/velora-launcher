package net.velora.core;

import java.util.Optional;
import java.util.UUID;

/** What the shared logic needs from an online player. Each platform provides an adapter. */
public interface CorePlayer {
    UUID uuid();
    String name();
    Pos pos();
    /** Send a chat message. Legacy section-sign colour codes are allowed. Safe to call from the server thread. */
    void send(String message);
    void teleport(Pos destination);
    /** True if the player may use {@code node}. Platforms map this onto LuckPerms or their own default. */
    boolean hasPermission(String node);
    /** Total time played, in seconds. */
    long playtimeSeconds();
    /** The stack in the main hand, empty if none. */
    Optional<Item> heldItem();
    /** Remove whatever is in the main hand. */
    void clearHeld();
    /** Put the stack in the inventory, or drop it at the player's feet if full. */
    void give(Item item);
    /**
     * Take up to {@code max} plain items (no enchantments, custom names or other data) with this panel id from anywhere in
     * the inventory, and return what was taken, stack by stack. Used to hand in buy orders and contracts, so a platform that
     * cannot do it simply returns nothing and the command says there is nothing to submit.
     */
    default java.util.List<Item> takeItems(String panelId, int max) { return java.util.List.of(); }
}
