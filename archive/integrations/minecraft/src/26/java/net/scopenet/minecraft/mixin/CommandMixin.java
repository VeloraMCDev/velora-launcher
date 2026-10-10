package net.scopenet.minecraft.mixin;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.server.network.ServerGamePacketListenerImpl;
import net.minecraft.network.protocol.game.*;
import net.scopenet.minecraft.Bridge;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(ServerGamePacketListenerImpl.class)
public abstract class CommandMixin {
    @Shadow public ServerPlayer player;
    @Inject(method = "handleChatCommand", at = @At("RETURN"))
    private void scopenet$command(ServerboundChatCommandPacket packet, CallbackInfo ci) {
        Bridge.command(player, packet.command());
    }

    @Inject(method = "handleSignedChatCommand", at = @At("RETURN"))
    private void scopenet$signed(ServerboundChatCommandSignedPacket packet, CallbackInfo ci) {
        Bridge.command(player, packet.command());
    }
}
