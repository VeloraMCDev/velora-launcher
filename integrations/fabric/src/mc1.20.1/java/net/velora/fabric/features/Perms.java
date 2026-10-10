package net.velora.fabric.features;

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

    /** The pre-rename node for a velora.* node, so permission setups made before the rename keep working. */
    static String legacy(String node) {
        return node.startsWith("velora.") ? "scopenet." + node.substring(7).replace("command.velora", "command.scopenet") : node;
    }

    static boolean luckPermsPresent() { return LUCKPERMS; }

    /** Nodes that grant extras (limit tiers like velora.homes.10): nobody has them unless a permissions system says so. */
    static boolean isGrantNode(String node) {
        return node.startsWith("velora.homes.") || node.startsWith("velora.limit.") || node.equals("free.fly") || node.equals("guild.fly")
                || node.startsWith("velora.vault.") || node.startsWith("velora.kit.") || node.startsWith("group.");
    }

    static boolean isAdminNode(String node) {
        return node.equals("velora.claims.bypass") || node.equals("velora.cooldown.bypass") || node.startsWith("velora.admin") || node.startsWith("velora.command.heal.others") || node.startsWith("velora.command.feed.others")
                || node.equals("velora.command.velora.status") || node.equals("velora.command.velora.reload");
    }

    boolean has(ServerPlayer player, String node) {
        if (LUCKPERMS) {
            try {
                Boolean answer = LuckPermsLookup.check(player, node);
                if (answer == null) answer = LuckPermsLookup.check(player, legacy(node));
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
