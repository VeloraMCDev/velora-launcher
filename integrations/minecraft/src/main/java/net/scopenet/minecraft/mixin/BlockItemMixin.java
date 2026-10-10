package net.scopenet.minecraft.mixin;

import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.InteractionResult;
import net.minecraft.world.item.BlockItem;
import net.minecraft.world.item.context.BlockPlaceContext;
import net.minecraft.core.registries.BuiltInRegistries;
import net.scopenet.minecraft.Bridge;
import net.scopenet.minecraft.Compat;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

@Mixin(BlockItem.class)
public abstract class BlockItemMixin {
    @Inject(method = "place(Lnet/minecraft/world/item/context/BlockPlaceContext;)Lnet/minecraft/world/InteractionResult;", at = @At("HEAD"), cancellable = true)
    private void scopenet$protectPlace(BlockPlaceContext context, CallbackInfoReturnable<InteractionResult> cir) {
        if (context.getPlayer() instanceof ServerPlayer player && !Bridge.canModify(player, context.getClickedPos(), "build", "place")) {
            cir.setReturnValue(InteractionResult.FAIL);
        }
    }

    @Inject(method = "place(Lnet/minecraft/world/item/context/BlockPlaceContext;)Lnet/minecraft/world/InteractionResult;", at = @At("RETURN"))
    private void scopenet$placed(BlockPlaceContext context, CallbackInfoReturnable<InteractionResult> cir) {
        if (cir.getReturnValue().consumesAction() && context.getPlayer() instanceof ServerPlayer player) {
            Bridge.stat(player, "blocks_placed", 1);
            String material = BuiltInRegistries.BLOCK.getKey(context.getLevel().getBlockState(context.getClickedPos()).getBlock())
                    .getPath().toUpperCase(java.util.Locale.ROOT);
            Bridge.action(player, "block_placed:" + material + "@" + Compat.dimension(player), 1);
        }
    }
}
