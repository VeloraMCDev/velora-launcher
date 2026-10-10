package net.velora.fabric;

import net.fabricmc.api.DedicatedServerModInitializer;
import net.fabricmc.loader.api.FabricLoader;

/** Server-only; lifecycle and admission hooks are shared mixins. Commands and extras need Fabric API (1.20.1 build). */
public final class VeloraFabric implements DedicatedServerModInitializer {
    @Override public void onInitializeServer() {
        if (Boolean.getBoolean("velora.verifyMixins"))
            org.spongepowered.asm.mixin.MixinEnvironment.getCurrentEnvironment().audit();
        net.velora.core.Legacy.moveDataDirectory(java.nio.file.Path.of("config"));
        try {
            // Create config/velora-core.properties on first launch so operators have a file to edit
            // even if the server stops later because the token is not set yet.
            net.scopenet.integration.Settings.ensureDefault(java.nio.file.Path.of("config", "velora-core.properties"));
        } catch (java.io.IOException e) {
            System.err.println("[Velora] Could not create config/velora-core.properties: " + e.getMessage());
        }
        if (!FabricLoader.getInstance().isModLoaded("fabric-api")) return;
        try {
            if (net.minecraft.SharedConstants.getCurrentVersion().getName().equals("1.20.1"))
                Class.forName("net.velora.core.vault.VaultMenu").getMethod("register").invoke(null);
            Class<?> features = Class.forName("net.velora.fabric.features.FabricFeatures");
            ((Runnable) features.getDeclaredConstructor().newInstance()).run();
        } catch (ClassNotFoundException e) {
            // This build (another Minecraft version) only has the core integration.
        } catch (Throwable t) {
            System.err.println("[Velora] Commands and extras could not start: " + t);
            t.printStackTrace();
        }
    }
}
