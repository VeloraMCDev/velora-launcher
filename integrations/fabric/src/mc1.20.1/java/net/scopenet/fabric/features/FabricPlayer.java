package net.scopenet.fabric.features;

import net.minecraft.network.chat.Component;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.stats.Stats;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.item.ItemStack;
import net.scopenet.core.CorePlayer;
import net.scopenet.core.Item;
import net.scopenet.core.Pos;

import java.util.Optional;
import java.util.UUID;

/** A {@link CorePlayer} over a Fabric ServerPlayer. */
final class FabricPlayer implements CorePlayer {
    private final ServerPlayer player;
    private final FabricPlatform platform;
    private final Perms perms;

    FabricPlayer(ServerPlayer player, FabricPlatform platform, Perms perms) {
        this.player = player; this.platform = platform; this.perms = perms;
    }

    ServerPlayer handle() { return player; }

    @Override public UUID uuid() { return player.getUUID(); }
    @Override public String name() { return player.getGameProfile().getName(); }

    @Override public Pos pos() {
        ServerLevel level = player.serverLevel();
        return new Pos(level.dimension().location().toString(), player.getX(), player.getY(), player.getZ(), player.getYRot(), player.getXRot());
    }

    @Override public void send(String message) { player.sendSystemMessage(Component.literal(message)); }

    @Override public void teleport(Pos destination) {
        ServerLevel level = platform.level(destination.world());
        if (level == null) { send("§cThat dimension is not loaded."); return; }
        player.teleportTo(level, destination.x(), destination.y(), destination.z(), destination.yaw(), destination.pitch());
    }

    @Override public boolean hasPermission(String node) { return perms.has(player, node); }

    @Override public long playtimeSeconds() {
        return player.getStats().getValue(Stats.CUSTOM.get(Stats.PLAY_TIME)) / 20L;
    }

    @Override public Optional<Item> heldItem() {
        ItemStack stack = player.getMainHandItem();
        return stack.isEmpty() ? Optional.empty() : Optional.of(FabricItems.toCore(stack));
    }

    @Override public void clearHeld() { player.setItemInHand(InteractionHand.MAIN_HAND, ItemStack.EMPTY); }

    @Override public java.util.List<Item> takeItems(String panelId, int max) {
        java.util.List<Item> taken = new java.util.ArrayList<>();
        var inventory = player.getInventory();
        int left = max;
        for (int i = 0; i < inventory.items.size() && left > 0; i++) {
            ItemStack stack = inventory.items.get(i);
            // Plain stacks only: anything with NBT (enchantments, names, damage) is not handed in.
            if (stack.isEmpty() || stack.hasTag()) continue;
            Item core = FabricItems.toCore(stack);
            if (!core.id().equalsIgnoreCase(panelId)) continue;
            int n = Math.min(left, stack.getCount());
            taken.add(new Item(core.id(), core.name(), n, ""));
            stack.shrink(n);
            if (stack.isEmpty()) inventory.items.set(i, ItemStack.EMPTY);
            left -= n;
        }
        return taken;
    }

    @Override public void give(Item item) {
        ItemStack stack = FabricItems.fromCore(item);
        if (stack.isEmpty()) { send("§cCould not deliver " + item.id() + " x" + item.count() + ". Ask an admin."); return; }
        player.getInventory().add(stack);
        if (!stack.isEmpty()) player.drop(stack, false);
    }
}
