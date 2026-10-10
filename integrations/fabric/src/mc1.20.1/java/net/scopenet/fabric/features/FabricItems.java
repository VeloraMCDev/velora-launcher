package net.scopenet.fabric.features;

import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.nbt.CompoundTag;
import net.minecraft.nbt.TagParser;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.world.item.ItemStack;
import net.scopenet.core.Item;

/** ItemStack <-> the shared {@link Item}. The exact stack (enchantments, names, NBT) travels as SNBT in {@code data}. */
final class FabricItems {
    private FabricItems() {}

    static Item toCore(ItemStack stack) {
        String registryId = BuiltInRegistries.ITEM.getKey(stack.getItem()).toString();
        return new Item(Item.panelId(registryId), stack.getHoverName().getString(), stack.getCount(), stack.save(new CompoundTag()).toString());
    }

    /** The stack for an item, restoring the exact NBT when it came from this platform; otherwise rebuilt from id and count. */
    static ItemStack fromCore(Item item) {
        if (!item.data().isEmpty()) {
            try {
                ItemStack stack = ItemStack.of(TagParser.parseTag(item.data()));
                if (!stack.isEmpty()) { stack.setCount(Math.max(1,item.count())); return stack; }
            } catch (Exception ignored) {
                // Another platform's serialisation: fall through to id and count.
            }
        }
        ResourceLocation id = ResourceLocation.tryParse(Item.registryId(item.id()));
        net.minecraft.world.item.Item type = id == null ? null : BuiltInRegistries.ITEM.get(id);
        if (type == null || type == net.minecraft.world.item.Items.AIR) return ItemStack.EMPTY;
        return new ItemStack(type, Math.max(1, item.count()));
    }
}
