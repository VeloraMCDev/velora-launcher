package net.velora.fabric.features.mixin;
import net.minecraft.core.BlockPos;
import net.minecraft.world.Container;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.entity.HopperBlockEntity;
import net.velora.fabric.features.PhysicalShops;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/** Registered stock chests are custody portals; hopper automation may not use their vanilla inventory. */
@Mixin(HopperBlockEntity.class)
public abstract class ShopHopperMixin {
    @Inject(method="getContainerAt(Lnet/minecraft/world/level/Level;Lnet/minecraft/core/BlockPos;)Lnet/minecraft/world/Container;",at=@At("HEAD"),cancellable=true)
    private static void velora$stock(Level level,BlockPos pos,CallbackInfoReturnable<Container> cir){
        if(PhysicalShops.isStockChest(level.dimension().location().toString(),pos))cir.setReturnValue(null);
    }
}
