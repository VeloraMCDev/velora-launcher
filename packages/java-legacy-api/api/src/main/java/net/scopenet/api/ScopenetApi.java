package net.scopenet.api;

import java.util.List;
import java.util.Optional;
import java.util.UUID;
import java.util.concurrent.CompletableFuture;

/**
 * The Velora developer API. Everything that talks to the panel returns a
 * {@link CompletableFuture} and never blocks the server thread; the
 * {@code getCached…} methods answer instantly from data Velora already holds.
 * Futures complete on a worker thread, so hop back with the Bukkit scheduler
 * before touching the world.
 *
 * <pre>{@code
 * ScopenetApi api = ScopenetApiProvider.get();
 * api.getPlayer(player.getUniqueId()).thenAccept(p -> getLogger().info(p.name() + " is level " + p.globalLevel()));
 * api.addXP(player.getUniqueId(), XpScope.SERVER, 250, "boss kill");
 * }</pre>
 *
 * Events ({@link net.scopenet.api.event.ScopenetLevelUpEvent},
 * {@link net.scopenet.api.event.GuildJoinEvent},
 * {@link net.scopenet.api.event.GuildLeaveEvent},
 * {@link net.scopenet.api.event.AchievementUnlockEvent}) are ordinary Bukkit events.
 */
public interface ScopenetApi {
    /** Bumped when the API changes incompatibly. */
    int API_VERSION = 1;

    /** Load a player's Velora profile. Completes with empty if they have no Velora account. */
    CompletableFuture<Optional<ScopenetPlayer>> getPlayer(UUID uuid);

    /** The last profile loaded for a player (kept fresh for online players), if any. Never blocks. */
    Optional<ScopenetPlayer> getCachedPlayer(UUID uuid);

    /** Find a guild on this server's instance by id, tag or name. */
    CompletableFuture<Optional<ScopenetGuild>> getGuild(String idTagOrName);

    /** The guild a player belongs to. */
    CompletableFuture<Optional<ScopenetGuild>> getPlayerGuild(UUID uuid);

    /** A player's balance on this server's economy. */
    CompletableFuture<Double> getBalance(UUID uuid);

    /**
     * Give (positive) or take (negative) money. Completes with the new balance, or
     * exceptionally if the player can't afford it.
     */
    CompletableFuture<Double> addBalance(UUID uuid, String playerName, double amount, String reason);

    /** Give (positive) or take (negative) XP. Level-ups fire {@link net.scopenet.api.event.ScopenetLevelUpEvent}. */
    CompletableFuture<XpResult> addXP(UUID uuid, XpScope scope, long amount, String reason);

    /**
     * Move a quest forward. {@code objective} is a quest id (with {@code amount} 0 meaning "complete it")
     * or an action like {@code block_broken:DIAMOND_ORE} that any matching quest counts.
     * Completes with whether any quest advanced.
     */
    CompletableFuture<Boolean> completeQuestObjective(UUID uuid, String objective, int amount);

    /** A player's accepted friends. */
    CompletableFuture<List<ScopenetFriend>> getFriends(UUID uuid);
}
