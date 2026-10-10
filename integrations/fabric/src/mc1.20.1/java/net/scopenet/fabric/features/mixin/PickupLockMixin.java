package net.scopenet.fabric.features.mixin;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.world.entity.player.Player;
import net.scopenet.fabric.features.InventoryLocks;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(ItemEntity.class)
public abstract class PickupLockMixin {
    @Inject(method="playerTouch",at=@At("HEAD"),cancellable=true)
    private void velora$locked(Player player,CallbackInfo ci){if(InventoryLocks.locked(player.getUUID()))ci.cancel();}
}
