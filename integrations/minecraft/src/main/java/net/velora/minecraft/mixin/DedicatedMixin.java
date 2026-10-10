package net.velora.minecraft.mixin;

import net.minecraft.server.MinecraftServer;
import net.minecraft.server.dedicated.DedicatedServer;
import net.velora.minecraft.Bridge;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

@Mixin(DedicatedServer.class)
public abstract class DedicatedMixin {
    @Inject(method = "initServer", at = @At("RETURN"))
    private void velora$start(CallbackInfoReturnable<Boolean> cir) {
        if (cir.getReturnValue()) Bridge.start((MinecraftServer) (Object) this);
    }
}
