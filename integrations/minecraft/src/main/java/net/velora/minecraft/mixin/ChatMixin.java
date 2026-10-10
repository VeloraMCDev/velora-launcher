package net.velora.minecraft.mixin;

import net.minecraft.server.level.ServerPlayer;
import net.minecraft.server.network.ServerGamePacketListenerImpl;
import net.velora.minecraft.Bridge;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(ServerGamePacketListenerImpl.class)
public abstract class ChatMixin {
    @Shadow public ServerPlayer player;
    @Inject(method = "broadcastChatMessage", at = @At("HEAD"))
    private void velora$chat(CallbackInfo ci) { Bridge.stat(player, "messages", 1); }

    @Inject(method = "handleContainerClick", at = @At("RETURN"))
    private void velora$inventory(CallbackInfo ci) { Bridge.action(player, "inventory_click", 1); }
}
