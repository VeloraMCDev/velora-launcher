package net.scopenet.minecraft.mixin;

import net.minecraft.core.BlockPos;
import net.minecraft.world.level.Explosion;
import net.minecraft.world.level.Level;
import net.scopenet.minecraft.Bridge;
import org.spongepowered.asm.mixin.Final;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Explosions leave claimed blocks alone unless the claim's {@code explosions} rule is on. Optional: skipped if the target differs. */
@Mixin(Explosion.class)
public abstract class RuleExplosionMixin {
    @Shadow @Final private Level level;

    @Inject(method = "finalizeExplosion", at = @At("HEAD"), require = 0)
    private void scopenet$spareClaims(boolean particles, CallbackInfo ci) {
        ((Explosion) (Object) this).getToBlow().removeIf((BlockPos pos) -> Bridge.environmentProtected(level, pos, "explosions"));
    }
}
