package net.scopenet.minecraft.mixin;

import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.damagesource.DamageSource;
import net.scopenet.minecraft.Bridge;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/** Land rules for damage to players: {@code pvp} and {@code player_damage}. Optional: skipped if the target differs. */
@Mixin(ServerPlayer.class)
public abstract class RulePvpMixin {
    @Inject(method = "hurtServer", at = @At("HEAD"), cancellable = true, require = 0)
    private void scopenet$landRules(ServerLevel level, DamageSource source, float amount, CallbackInfoReturnable<Boolean> cir) {
        if (Bridge.blocksDamage((ServerPlayer) (Object) this, source)) cir.setReturnValue(false);
    }
}
