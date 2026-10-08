package net.scopenet.minecraft.mixin;

import com.mojang.authlib.GameProfile;
import net.minecraft.network.Connection;
import net.minecraft.network.chat.Component;
import net.minecraft.server.network.ServerLoginPacketListenerImpl;
import net.scopenet.minecraft.Bridge;
import java.util.concurrent.CompletableFuture;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(ServerLoginPacketListenerImpl.class)
public abstract class LoginMixin {
    @Shadow private GameProfile gameProfile;
    @Shadow @Final public Connection connection;
    @Shadow public abstract void disconnect(Component reason);
    @Unique private CompletableFuture<String> scopenet$verdict;

    @Inject(method = "handleAcceptedLogin", at = @At("HEAD"), cancellable = true)
    private void scopenet$gate(CallbackInfo ci) {
        if (scopenet$verdict == null)
            scopenet$verdict = Bridge.login(gameProfile.getId(), gameProfile.getName(), connection.getRemoteAddress());
        if (!scopenet$verdict.isDone()) { ci.cancel(); return; }
        String denial = scopenet$verdict.join();
        if (denial != null) { disconnect(Component.literal(denial)); ci.cancel(); }
    }
}
