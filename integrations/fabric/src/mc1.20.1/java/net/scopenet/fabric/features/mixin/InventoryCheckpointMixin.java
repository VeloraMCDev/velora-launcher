package net.scopenet.fabric.features.mixin;
import net.minecraft.nbt.CompoundTag;
import net.minecraft.server.level.ServerPlayer;
import net.scopenet.fabric.features.InventoryCheckpoint;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(ServerPlayer.class)
public abstract class InventoryCheckpointMixin implements InventoryCheckpoint {
    @Unique private String velora$transfer="";
    public String velora$checkpoint(){return velora$transfer;}
    public void velora$checkpoint(String id){velora$transfer=id;}
    @Inject(method="readAdditionalSaveData",at=@At("TAIL"))
    private void velora$read(CompoundTag tag,CallbackInfo ci){velora$transfer=tag.getString("VeloraInventoryCheckpoint");}
    @Inject(method="addAdditionalSaveData",at=@At("TAIL"))
    private void velora$write(CompoundTag tag,CallbackInfo ci){tag.putString("VeloraInventoryCheckpoint",velora$transfer);}
    @Inject(method="restoreFrom",at=@At("TAIL"))
    private void velora$clone(ServerPlayer old,boolean alive,CallbackInfo ci){velora$transfer=((InventoryCheckpoint)old).velora$checkpoint();}
}
