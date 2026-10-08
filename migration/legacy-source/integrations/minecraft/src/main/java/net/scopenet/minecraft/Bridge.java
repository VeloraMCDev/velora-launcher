package net.scopenet.minecraft;

import net.scopenet.integration.*;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.network.chat.Component;
import net.minecraft.core.BlockPos;
import java.net.*;
import java.nio.file.Path;
import java.util.*;
import java.util.concurrent.CompletableFuture;

public final class Bridge {
    private static volatile Integration integration;
    private static final Map<UUID, String> online = new HashMap<>();
    private static int mapTicks;
    private Bridge() {}

    /** The running integration, or null before the server has started. */
    public static Integration integration() { return integration; }

    /** Lets a permissions system (LuckPerms) say who may build in other guilds' claims. Default: nobody. */
    public static volatile java.util.function.Predicate<ServerPlayer> claimBypass = player -> false;

    public static void start(MinecraftServer server) {
        try {
            integration = new Integration(Settings.load(Path.of("config/scopenet.properties")),
                    BuildInfo.LOADER, server.getServerVersion(), BuildInfo.VERSION, server.usesAuthentication(),
                    server.getMaxPlayers(), System.err::println,
                    (id, message) -> server.execute(() -> {
                        ServerPlayer player = server.getPlayerList().getPlayer(id);
                        if (player != null) player.connection.disconnect(Component.literal(message));
                    }));
            integration.startClaimSync();
            Path config = Path.of("config/scopenet.properties");
            if (Settings.mapEnabled(config)) integration.enableMap(server.getWorldPath(net.minecraft.world.level.storage.LevelResource.ROOT), Path.of("config/scopenet/map"));
        } catch (Exception e) {
            // Throwing aborts startup; silently disabling an auth mod is unsafe.
            throw new IllegalStateException("SCOPENET cannot start: " + e.getMessage(), e);
        }
    }

    public static CompletableFuture<String> login(UUID uuid, String name, SocketAddress address) {
        Integration current = integration;
        if (current == null) return CompletableFuture.completedFuture(Integration.UNAVAILABLE);
        String ip = null;
        if (address instanceof InetSocketAddress socket) {
            ip = socket.getAddress() != null ? socket.getAddress().getHostAddress() : socket.getHostString();
        }
        return current.login(uuid, name, ip);
    }

    public static void tick(MinecraftServer server) {
        Integration current = integration;
        if (current == null) return;
        Set<UUID> present = new HashSet<>();
        for (ServerPlayer player : server.getPlayerList().getPlayers()) {
            UUID id = player.getUUID();
            present.add(id);
            if (!online.containsKey(id)) {
                String name = player.getName().getString();
                online.put(id, name);
                current.activity.join(id, name);
            }
        }
        online.entrySet().removeIf(entry -> {
            if (present.contains(entry.getKey())) return false;
            current.activity.leave(entry.getKey(), entry.getValue());
            return true;
        });
        current.tick();
        if (++mapTicks >= 20) {
            mapTicks = 0;
            List<net.scopenet.worldmap.WorldMapSync.Player> positions = new ArrayList<>();
            for (ServerPlayer player : server.getPlayerList().getPlayers()) {
                positions.add(new net.scopenet.worldmap.WorldMapSync.Player(player.getUUID().toString(), player.getName().getString(), Compat.dimension(player),
                        Math.round(player.getX() * 10) / 10.0, Math.round(player.getY() * 10) / 10.0, Math.round(player.getZ() * 10) / 10.0,
                        Math.round(player.getYRot()), Math.round(player.getXRot())));
            }
            current.mapPlayers(positions);
        }
    }

    public static void stat(ServerPlayer player, String key, int amount) {
        Integration current = integration;
        if (current == null) return;
        current.activity.add(player.getUUID(), player.getName().getString(), key, amount);
        if (key.equals("deaths")) current.activity.event(player.getUUID(), player.getName().getString(), "death", null);
    }

    public static void action(ServerPlayer player, String action, int amount) {
        Integration current = integration;
        if (current != null) current.activity.action(player.getUUID(), player.getName().getString(), action, amount);
    }

    /** Fail closed while a claim lookup is pending or the panel is unavailable. Building: members, or visitors where the claim allows it. */
    public static boolean canModify(ServerPlayer player, BlockPos pos) { return canModify(player, pos, "build", true); }

    /** Like {@link #canModify(ServerPlayer, BlockPos)} for one kind of change: {@code build}, {@code interact} or {@code containers}. */
    public static boolean canModify(ServerPlayer player, BlockPos pos, String flag) { return canModify(player, pos, flag, true); }

    /** The same answer without telling the player (for checks the game makes on its own, like a bucket's reach). */
    public static boolean mayModify(ServerPlayer player, BlockPos pos, String flag) { return canModify(player, pos, flag, false); }

    private static boolean canModify(ServerPlayer player, BlockPos pos, String flag, boolean tell) {
        Integration current = integration;
        if (current == null) return false;
        if (!current.settings().guildsEnabled() || !current.settings().landClaimingEnabled()) return true;
        if (claimBypass.test(player)) return true;
        if (current.mayModify(Compat.dimension(player), pos.getX() >> 4, pos.getZ() >> 4, player.getUUID(), flag)) return true;
        if (tell) Compat.actionbar(player, Component.literal("This land is protected by a guild or the claim check is unavailable."));
        return false;
    }

    /** Fire, fluids and the like never touch claimed land unless that claim's rule for {@code flag} allows it. */
    public static boolean environmentProtected(net.minecraft.world.level.Level level, BlockPos pos, String flag) {
        Integration current = integration;
        if (current == null) return false;
        String dimension = Compat.dimension(level);
        int chunkX = pos.getX() >> 4, chunkZ = pos.getZ() >> 4;
        return current.isClaimed(dimension, chunkX, chunkZ) && !current.adminAllows(dimension, chunkX, chunkZ, flag);
    }

    /** Does the claim here forbid {@code flag} (pvp, mob_spawning…)? False in wilderness and before the first sync. */
    public static boolean ruleDenies(net.minecraft.world.level.Level level, BlockPos pos, String flag) {
        Integration current = integration;
        return current != null && !current.adminAllows(Compat.dimension(level), pos.getX() >> 4, pos.getZ() >> 4, flag);
    }

    /** Should this damage to a player be cancelled by the land rules? PVP and general damage, per claim. Staff are not held back. */
    public static boolean blocksDamage(ServerPlayer victim, net.minecraft.world.damagesource.DamageSource source) {
        if (integration == null) return false;
        net.minecraft.world.level.Level level = Compat.level(victim);
        BlockPos at = victim.blockPosition();
        if (source.getEntity() instanceof ServerPlayer attacker && attacker != victim) {
            if (!claimBypass.test(attacker) && ruleDenies(level, at, "pvp")) return true;
        }
        return ruleDenies(level, at, "player_damage");
    }

    public static void command(ServerPlayer player, String command) {
        String name = command.strip().split("\\s+", 2)[0];
        Integration current = integration;
        if (current != null) current.activity.event(player.getUUID(), player.getName().getString(), "command", "/" + name);
    }

    public static void stop() {
        Integration current = integration;
        integration = null;
        if (current != null) current.close();
        online.clear();
    }
}
