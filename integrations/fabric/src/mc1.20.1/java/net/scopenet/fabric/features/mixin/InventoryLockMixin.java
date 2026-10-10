package net.scopenet.fabric.features.mixin;
import net.minecraft.server.network.ServerGamePacketListenerImpl;
import net.minecraft.server.level.ServerPlayer;
import net.scopenet.fabric.features.InventoryLocks;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(ServerGamePacketListenerImpl.class)
public abstract class InventoryLockMixin {
    @Shadow public ServerPlayer player;
    @Inject(method={"handleContainerClick","handleContainerButtonClick","handlePlayerAction","handleUseItem","handleUseItemOn","handleSetCreativeModeSlot","handleChatCommand","handleInteract","handlePlaceRecipe","handlePickItem","handleEditBook","handleSetBeaconPacket"},at=@At("HEAD"),cancellable=true)
    private void velora$locked(CallbackInfo ci){
        // PacketUtils reschedules network-thread calls. Inspect containers only on the server thread.
        if(player.getServer()==null||!player.getServer().isSameThread())return;
        if(InventoryLocks.locked(player.getUUID())){player.containerMenu.broadcastFullState();ci.cancel();}
    }
}
