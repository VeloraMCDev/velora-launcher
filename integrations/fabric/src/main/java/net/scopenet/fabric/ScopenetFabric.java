package net.scopenet.fabric;

import net.fabricmc.api.DedicatedServerModInitializer;
import net.fabricmc.loader.api.FabricLoader;

/** Server-only; lifecycle and admission hooks are shared mixins. Commands and extras need Fabric API (1.20.1 build). */
public final class ScopenetFabric implements DedicatedServerModInitializer {
    @Override public void onInitializeServer() {
        if (Boolean.getBoolean("scopenet.verifyMixins"))
            org.spongepowered.asm.mixin.MixinEnvironment.getCurrentEnvironment().audit();
        try {
            // Create config/scopenet.properties on first launch so operators have a file to edit
            // even if the server stops later because the token is not set yet.
            net.scopenet.integration.Settings.ensureDefault(java.nio.file.Path.of("config", "scopenet.properties"));
        } catch (java.io.IOException e) {
            System.err.println("[Velora] Could not create config/scopenet.properties: " + e.getMessage());
        }
        if (!FabricLoader.getInstance().isModLoaded("fabric-api")) return;
        try {
            if (net.minecraft.SharedConstants.getCurrentVersion().getName().equals("1.20.1"))
                Class.forName("net.scopenet.core.vault.VaultMenu").getMethod("register").invoke(null);
            Class<?> features = Class.forName("net.scopenet.fabric.features.FabricFeatures");
            ((Runnable) features.getDeclaredConstructor().newInstance()).run();
        } catch (ClassNotFoundException e) {
            // This build (another Minecraft version) only has the core integration.
        } catch (Throwable t) {
            System.err.println("[Velora] Commands and extras could not start: " + t);
            t.printStackTrace();
        }
    }
}
