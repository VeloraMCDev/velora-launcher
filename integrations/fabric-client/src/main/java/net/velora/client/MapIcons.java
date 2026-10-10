package net.velora.client;

import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.minecraft.world.item.SpawnEggItem;

/** Uses the installed game's textures; no Minecraft artwork is bundled. */
public final class MapIcons {
    private record Face(String texture, int u, int v, int width, int height, int atlasWidth, int atlasHeight) {}
    private MapIcons() {}
    public static void pin(GuiGraphics g, String kind, int x, int y, int color) {
        g.fill(x - 9, y - 9, x + 9, y + 9, 0xe6171421);
        g.fill(x - 9, y + 8, x + 9, y + 9, color);
        if (kind.equals("you") && net.minecraft.client.Minecraft.getInstance().player != null) {
            var skin = net.minecraft.client.Minecraft.getInstance().player.getSkinTextureLocation();
            g.blit(skin, x - 6, y - 6, 12, 12, 8f, 8f, 8, 8, 64, 64);
            g.blit(skin, x - 6, y - 6, 12, 12, 40f, 8f, 8, 8, 64, 64);
            return;
        }
        String item = switch (kind) {
            case "you", "player" -> "player_head";
            case "spawn" -> "compass";
            case "warp" -> "ender_pearl";
            case "home", "guild_home" -> "red_bed";
            case "shop", "market" -> "chest";
            default -> "purple_banner";
        };
        g.renderItem(Ui.stackFor(item, 1), x - 8, y - 8);
    }
    public static void mob(GuiGraphics g, Entity entity, int x, int y) {
        String id = BuiltInRegistries.ENTITY_TYPE.getKey(entity.getType()).toString();
        Face face = switch (id) {
            case "minecraft:creeper" -> new Face("creeper/creeper", 8, 8, 8, 8, 64, 32);
            case "minecraft:zombie" -> new Face("zombie/zombie", 8, 8, 8, 8, 64, 64);
            case "minecraft:husk" -> new Face("zombie/husk", 8, 8, 8, 8, 64, 64);
            case "minecraft:drowned" -> new Face("zombie/drowned", 8, 8, 8, 8, 64, 64);
            case "minecraft:skeleton" -> new Face("skeleton/skeleton", 8, 8, 8, 8, 64, 32);
            case "minecraft:stray" -> new Face("skeleton/stray", 8, 8, 8, 8, 64, 32);
            case "minecraft:wither_skeleton" -> new Face("skeleton/wither_skeleton", 8, 8, 8, 8, 64, 32);
            case "minecraft:enderman" -> new Face("enderman/enderman", 8, 8, 8, 8, 64, 32);
            case "minecraft:pig" -> new Face("pig/pig", 8, 8, 8, 8, 64, 32);
            case "minecraft:cow" -> new Face("cow/cow", 6, 6, 8, 8, 64, 32);
            case "minecraft:sheep" -> new Face("sheep/sheep", 6, 6, 6, 6, 64, 32);
            case "minecraft:chicken" -> new Face("chicken", 2, 3, 4, 6, 64, 32);
            case "minecraft:wolf" -> new Face("wolf/wolf", 4, 4, 6, 6, 64, 32);
            case "minecraft:spider" -> new Face("spider/spider", 32, 4, 8, 8, 64, 32);
            case "minecraft:cave_spider" -> new Face("spider/cave_spider", 32, 4, 8, 8, 64, 32);
            case "minecraft:slime" -> new Face("slime/slime", 8, 8, 8, 8, 64, 32);
            default -> null;
        };
        g.fill(x - 7, y - 7, x + 7, y + 7, 0xe6171421);
        if (face != null) {
            g.blit(new ResourceLocation("minecraft", "textures/entity/" + face.texture() + ".png"), x - 6, y - 6, 12, 12, (float) face.u(), (float) face.v(), face.width(), face.height(), face.atlasWidth(), face.atlasHeight());
        } else {
            var egg = SpawnEggItem.byId(entity.getType());
            g.renderItem(new ItemStack(egg == null ? Items.EGG : egg), x - 8, y - 8);
        }
    }
}
