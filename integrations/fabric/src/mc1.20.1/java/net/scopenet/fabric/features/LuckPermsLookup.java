package net.scopenet.fabric.features;

import net.luckperms.api.LuckPerms;
import net.luckperms.api.LuckPermsProvider;
import net.luckperms.api.model.user.User;
import net.luckperms.api.util.Tristate;
import net.minecraft.server.level.ServerPlayer;

/** Only loaded when the LuckPerms mod is present (see {@link Perms}). */
final class LuckPermsLookup {
    private LuckPermsLookup() {}

    /** True or false when LuckPerms has the node set for this player, null when it has no opinion. */
    static Boolean check(ServerPlayer player, String node) {
        LuckPerms api = LuckPermsProvider.get();
        User user = api.getUserManager().getUser(player.getUUID());
        if (user == null) return null;
        Tristate result = user.getCachedData().getPermissionData().checkPermission(node);
        return result == Tristate.UNDEFINED ? null : result.asBoolean();
    }
}
