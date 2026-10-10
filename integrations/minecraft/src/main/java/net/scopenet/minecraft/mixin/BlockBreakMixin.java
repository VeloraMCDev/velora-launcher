package net.scopenet.minecraft.mixin;

import net.minecraft.core.BlockPos;
import net.minecraft.server.level.ServerPlayerGameMode;
import net.scopenet.minecraft.Bridge;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

@Mixin(ServerPlayerGameMode.class)
public abstract class BlockBreakMixin {
    @Shadow protected net.minecraft.server.level.ServerPlayer player;

    @Inject(method = "destroyBlock", at = @At("HEAD"), cancellable = true)
    private void scopenet$protectBreak(BlockPos pos, CallbackInfoReturnable<Boolean> cir) {
        if (!Bridge.canModify(player, pos, "build", "break")) cir.setReturnValue(false);
    }
}
