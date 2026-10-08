package net.scopenet.minecraft.mixin;

import net.minecraft.server.level.ServerPlayer;
import net.minecraft.server.level.ServerPlayerGameMode;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.InteractionResult;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.Level;
import net.minecraft.world.phys.BlockHitResult;
import net.scopenet.minecraft.Bridge;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

@Mixin(ServerPlayerGameMode.class)
public abstract class BlockInteractMixin {
    @Inject(method = "useItemOn", at = @At("HEAD"), cancellable = true)
    private void scopenet$protectInteraction(ServerPlayer player, Level level, ItemStack stack,
            InteractionHand hand, BlockHitResult hit, CallbackInfoReturnable<InteractionResult> cir) {
        // Chests and doors are separate rules on a claim; placing a block is a build on the neighbouring position.
        net.minecraft.core.BlockPos clicked = hit.getBlockPos();
        String rule = level.getBlockEntity(clicked) instanceof net.minecraft.world.Container ? "containers" : "interact";
        boolean placing = stack.getItem() instanceof net.minecraft.world.item.BlockItem;
        if (!Bridge.canModify(player, clicked, rule)
                || (placing && !Bridge.canModify(player, clicked.relative(hit.getDirection()), "build"))) {
            cir.setReturnValue(InteractionResult.FAIL);
        }
    }
}
