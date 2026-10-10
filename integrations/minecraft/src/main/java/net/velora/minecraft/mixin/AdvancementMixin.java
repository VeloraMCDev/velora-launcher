package net.velora.minecraft.mixin;

import net.minecraft.server.PlayerAdvancements;
import net.minecraft.server.level.ServerPlayer;
import net.velora.minecraft.Bridge;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

@Mixin(PlayerAdvancements.class)
public abstract class AdvancementMixin {
    @Shadow private ServerPlayer player;
    @Inject(method = "award", at = @At("RETURN"))
    private void velora$advancement(CallbackInfoReturnable<Boolean> cir) {
        if (cir.getReturnValue()) Bridge.action(player, "advancement_criterion", 1);
    }
}
