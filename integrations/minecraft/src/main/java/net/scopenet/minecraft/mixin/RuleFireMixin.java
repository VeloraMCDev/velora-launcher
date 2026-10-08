package net.scopenet.minecraft.mixin;

import net.minecraft.core.BlockPos;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.util.RandomSource;
import net.minecraft.world.level.block.state.BlockState;
import net.scopenet.minecraft.Bridge;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Claimed land keeps fire from spreading and burning blocks, unless the claim's {@code fire_spread} rule is on. Optional: skipped if the target differs. */
@Mixin(net.minecraft.world.level.block.FireBlock.class)
public abstract class RuleFireMixin {
    @Inject(method = "tick", at = @At("HEAD"), cancellable = true, require = 0)
    private void scopenet$noSpread(BlockState state, ServerLevel level, BlockPos pos, RandomSource random, CallbackInfo ci) {
        if (Bridge.environmentProtected(level, pos, "fire_spread")) {
            level.removeBlock(pos, false); // the fire dies out instead of lingering unaged
            ci.cancel();
        }
    }
}
