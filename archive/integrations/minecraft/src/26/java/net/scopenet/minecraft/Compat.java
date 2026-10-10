package net.scopenet.minecraft;

import net.minecraft.network.chat.Component;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;

public final class Compat {
    private Compat() {}

    public static ServerLevel level(ServerPlayer player) { return player.level(); }
    public static String dimension(ServerPlayer player) { return level(player).dimension().identifier().toString(); }
    public static String dimension(net.minecraft.world.level.Level level) { return level.dimension().identifier().toString(); }
    public static void actionbar(ServerPlayer player, Component message) { player.sendSystemMessage(message, true); }
}
