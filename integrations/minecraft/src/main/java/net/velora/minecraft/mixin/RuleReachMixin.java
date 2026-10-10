package net.velora.minecraft.mixin;

import net.minecraft.core.BlockPos;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.entity.player.Player;
import net.velora.minecraft.Bridge;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/**
 * The game asks this before a bucket empties or fills, so guarding it keeps visitors from pouring water or lava on claimed land
 * (the block hooks do not see buckets). Optional: skipped if the target differs.
 */
@Mixin(ServerLevel.class)
public abstract class RuleReachMixin {
    @Inject(method = "mayInteract", at = @At("HEAD"), cancellable = true, require = 0)
    private void velora$claimReach(Player player, BlockPos pos, CallbackInfoReturnable<Boolean> cir) {
        if (player instanceof ServerPlayer sp && !Bridge.mayModify(sp, pos, "build")) cir.setReturnValue(false);
    }
}
