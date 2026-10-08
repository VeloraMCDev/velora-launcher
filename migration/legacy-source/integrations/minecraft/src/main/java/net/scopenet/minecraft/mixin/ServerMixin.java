package net.scopenet.minecraft.mixin;

import net.minecraft.server.MinecraftServer;
import net.scopenet.minecraft.Bridge;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(MinecraftServer.class)
public abstract class ServerMixin {
    @Inject(method = "tickServer", at = @At("TAIL"))
    private void scopenet$tick(CallbackInfo ci) { Bridge.tick((MinecraftServer) (Object) this); }

    @Inject(method = "stopServer", at = @At("HEAD"))
    private void scopenet$stop(CallbackInfo ci) { Bridge.stop(); }
}
