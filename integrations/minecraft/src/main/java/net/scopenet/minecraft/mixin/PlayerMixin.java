package net.scopenet.minecraft.mixin;

import net.minecraft.server.level.ServerPlayer;
import net.minecraft.stats.*;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.entity.EntityType;
import net.minecraft.core.registries.BuiltInRegistries;
import net.scopenet.minecraft.Bridge;
import net.scopenet.minecraft.Compat;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(ServerPlayer.class)
public abstract class PlayerMixin {
    @Inject(method = "die", at = @At("HEAD"))
    private void velora$pvp(net.minecraft.world.damagesource.DamageSource source, CallbackInfo ci) {
        if (source.getEntity() instanceof ServerPlayer killer && killer != (Object) this)
            Bridge.pvpKill(killer, (ServerPlayer) (Object) this);
    }
    @Inject(method = "awardStat", at = @At("TAIL"))
    private void scopenet$stat(Stat<?> stat, int amount, CallbackInfo ci) {
        Bridge.action((ServerPlayer) (Object) this, stat.getName(), amount);
        String key = null;
        if (stat.getType() == Stats.BLOCK_MINED) {
            key = "blocks_broken";
            String material = BuiltInRegistries.BLOCK.getKey((Block) stat.getValue()).getPath().toUpperCase(java.util.Locale.ROOT);
            Bridge.action((ServerPlayer) (Object) this, "block_broken:" + material + "@"
                    + Compat.dimension((ServerPlayer) (Object) this), amount);
        } else if (stat.getType() == Stats.ENTITY_KILLED) {
            String entity = BuiltInRegistries.ENTITY_TYPE.getKey((EntityType<?>) stat.getValue()).getPath().toUpperCase(java.util.Locale.ROOT);
            Bridge.action((ServerPlayer) (Object) this, "mob_kills:" + entity, amount);
        }
        if (stat.getType() == Stats.CUSTOM) {
            key = switch (stat.getValue().toString()) {
                case "minecraft:deaths" -> "deaths";
                case "minecraft:player_kills" -> "player_kills";
                case "minecraft:mob_kills" -> "mob_kills";
                default -> null;
            };
        }
        if (key != null) Bridge.stat((ServerPlayer) (Object) this, key, amount);
    }
}
