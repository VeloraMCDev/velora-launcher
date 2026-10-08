package net.scopenet.minecraft.mixin;

import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.Mob;
import net.scopenet.minecraft.Bridge;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/** Mobs do not spawn on land whose {@code mob_spawning} rule is off. Optional: skipped if the target differs. */
@Mixin(ServerLevel.class)
public abstract class RuleSpawnMixin {
    @Inject(method = "addFreshEntity", at = @At("HEAD"), cancellable = true, require = 0)
    private void scopenet$noSpawn(Entity entity, CallbackInfoReturnable<Boolean> cir) {
        if (entity instanceof Mob && Bridge.ruleDenies((ServerLevel) (Object) this, entity.blockPosition(), "mob_spawning")) cir.setReturnValue(false);
    }
}
