package net.scopenet.fabric.features;

import net.fabricmc.loader.api.FabricLoader;
import net.minecraft.server.level.ServerPlayer;

/**
 * Permission checks. With the LuckPerms mod installed its answer wins whenever the node is set; otherwise (or when
 * LuckPerms has no opinion) the defaults apply: ordinary commands follow {@code permissions.default_level}, and
 * admin nodes need operator level 2. Same node names as the Paper plugin, so one LuckPerms setup serves both.
 */
final class Perms {
    private static final boolean LUCKPERMS = FabricLoader.getInstance().isModLoaded("luckperms");
    private final boolean everyoneByDefault;

    Perms(boolean everyoneByDefault) { this.everyoneByDefault = everyoneByDefault; }

    static boolean luckPermsPresent() { return LUCKPERMS; }

    /** Nodes that grant extras (limit tiers like scopenet.homes.10): nobody has them unless a permissions system says so. */
    static boolean isGrantNode(String node) {
        return node.startsWith("scopenet.homes.") || node.startsWith("scopenet.limit.") || node.equals("free.fly") || node.equals("guild.fly")
                || node.startsWith("scopenet.vault.") || node.startsWith("scopenet.kit.") || node.startsWith("group.");
    }

    static boolean isAdminNode(String node) {
        return node.equals("scopenet.claims.bypass") || node.equals("scopenet.cooldown.bypass") || node.startsWith("scopenet.admin") || node.startsWith("scopenet.command.heal.others") || node.startsWith("scopenet.command.feed.others")
                || node.equals("scopenet.command.scopenet.status") || node.equals("scopenet.command.scopenet.reload");
    }

    boolean has(ServerPlayer player, String node) {
        if (LUCKPERMS) {
            try {
                Boolean answer = LuckPermsLookup.check(player, node);
                if (answer != null) return answer;
            } catch (Throwable t) {
                // LuckPerms not ready yet or an API change: fall back to the defaults below.
            }
        }
        if (isGrantNode(node)) return false;
        if (isAdminNode(node)) return player.hasPermissions(2);
        return everyoneByDefault || player.hasPermissions(2);
    }
}
