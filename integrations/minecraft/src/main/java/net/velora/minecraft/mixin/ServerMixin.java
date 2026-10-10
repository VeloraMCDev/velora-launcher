package net.velora.minecraft.mixin;

import net.minecraft.server.MinecraftServer;
import net.velora.minecraft.Bridge;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(MinecraftServer.class)
public abstract class ServerMixin {
    @Inject(method = "tickServer", at = @At("TAIL"))
    private void velora$tick(CallbackInfo ci) { Bridge.tick((MinecraftServer) (Object) this); }

    @Inject(method = "stopServer", at = @At("HEAD"))
    private void velora$stop(CallbackInfo ci) { Bridge.stop(); }
}
