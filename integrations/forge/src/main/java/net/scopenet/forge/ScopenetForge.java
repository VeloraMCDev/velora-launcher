package net.scopenet.forge;

import net.minecraftforge.fml.common.Mod;

@Mod("scopenet")
public final class ScopenetForge {
    public ScopenetForge() {
        if (Boolean.getBoolean("scopenet.verifyMixins"))
            org.spongepowered.asm.mixin.MixinEnvironment.getCurrentEnvironment().audit();
    }
}
