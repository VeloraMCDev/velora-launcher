package net.scopenet.minecraft.mixin;

import net.minecraft.server.level.ServerPlayer;
import net.minecraft.server.network.ServerGamePacketListenerImpl;
import net.scopenet.minecraft.Bridge;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(ServerGamePacketListenerImpl.class)
public abstract class ChatMixin {
    @Shadow public ServerPlayer player;
    @Inject(method = "broadcastChatMessage", at = @At("HEAD"))
    private void scopenet$chat(CallbackInfo ci) { Bridge.stat(player, "messages", 1); }

    @Inject(method = "handleContainerClick", at = @At("RETURN"))
    private void scopenet$inventory(CallbackInfo ci) { Bridge.action(player, "inventory_click", 1); }
}
