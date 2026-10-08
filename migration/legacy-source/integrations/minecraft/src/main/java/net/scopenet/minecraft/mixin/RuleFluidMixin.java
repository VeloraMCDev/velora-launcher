package net.scopenet.minecraft.mixin;

import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.LevelAccessor;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.material.FluidState;
import net.scopenet.minecraft.Bridge;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Water and lava do not flow onto claimed land unless the claim's {@code fluid_flow} rule is on. Optional: skipped if the target differs. */
@Mixin(net.minecraft.world.level.material.FlowingFluid.class)
public abstract class RuleFluidMixin {
    @Inject(method = "spreadTo", at = @At("HEAD"), cancellable = true, require = 0)
    private void scopenet$noFlow(LevelAccessor level, BlockPos pos, BlockState state, Direction direction, FluidState fluid, CallbackInfo ci) {
        if (level instanceof Level real && Bridge.environmentProtected(real, pos, "fluid_flow")) ci.cancel();
    }
}
